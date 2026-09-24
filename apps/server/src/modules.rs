//! The Server's own switchable modules — the parts that live on the Server
//! rather than being passed through to the Agent. Switch them off in
//! `server.toml` (`[modules] disabled = ["audit"]`) or, taking precedence,
//! with `DECKOX_DISABLED_MODULES=audit,notifications` (comma-separated); a
//! module that is off answers `404 module_disabled` and starts no background
//! work.
//! The Agent's modules are switched in `agent.toml` and reach the Web through
//! `GET /api/v1/modules`.

use std::{
    collections::HashSet,
    env, fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::http::StatusCode;
use axum::{
    extract::{OriginalUri, Request, State},
    middleware::Next,
    response::Response,
};
use deckox_protocol::{
    ModuleDefinition, ModuleInfo, module_infos, module_owner, validate_disabled,
};

use crate::error_response;

const DEFAULT_CONFIG: &str = "/etc/deckox/server.toml";

pub fn config_path() -> PathBuf {
    PathBuf::from(env::var("DECKOX_SERVER_CONFIG").unwrap_or_else(|_| DEFAULT_CONFIG.to_owned()))
}

/// The `[modules] disabled` list of `server.toml`. A missing file, table or
/// key means nothing is switched off; a file that cannot be read or parsed is
/// an error, so a typo never silently leaves a module on.
pub fn disabled_from_file(path: &Path) -> Result<Vec<String>, String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
    };
    let value: toml::Value =
        toml::from_str(&text).map_err(|error| format!("invalid {}: {error}", path.display()))?;
    let Some(list) = value.get("modules").and_then(|table| table.get("disabled")) else {
        return Ok(Vec::new());
    };
    list.as_array()
        .ok_or_else(|| format!("{}: [modules] disabled must be a list", path.display()))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{}: [modules] disabled must list names", path.display()))
        })
        .collect()
}

/// The environment variable wins when it names any module; otherwise the file's list applies.
fn resolve(from_environment: Option<&str>, from_file: Vec<String>) -> Vec<String> {
    let listed: Vec<String> = from_environment
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .collect();
    if listed.is_empty() { from_file } else { listed }
}

const DEFINITIONS: &[ModuleDefinition] = &[
    ModuleDefinition {
        id: "audit",
        requires: &[],
        routes: &["/api/v1/audit"],
    },
    ModuleDefinition {
        id: "realtime",
        requires: &[],
        routes: &["/api/v1/events/metrics"],
    },
    ModuleDefinition {
        id: "notifications",
        requires: &[],
        routes: &["/api/v1/settings/webhook"],
    },
    ModuleDefinition {
        id: "update-check",
        requires: &[],
        routes: &["/api/v1/update"],
    },
];

#[derive(Clone)]
pub struct ModuleRegistry {
    disabled: Arc<HashSet<String>>,
}

impl ModuleRegistry {
    pub fn new(disabled: &[String]) -> Result<Self, String> {
        Ok(Self {
            disabled: Arc::new(validate_disabled(DEFINITIONS, disabled)?),
        })
    }

    pub fn from_environment() -> Result<Self, String> {
        let from_file = disabled_from_file(&config_path())?;
        Self::new(&resolve(
            env::var("DECKOX_DISABLED_MODULES").ok().as_deref(),
            from_file,
        ))
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        !self.disabled.contains(id)
    }

    pub fn infos(&self) -> Vec<ModuleInfo> {
        module_infos(DEFINITIONS, &self.disabled)
    }

    fn owner_of(path: &str) -> Option<&'static str> {
        module_owner(DEFINITIONS, path)
    }
}

/// Answers `404 module_disabled` for a path whose module is switched off.
pub async fn gate(
    State(registry): State<ModuleRegistry>,
    OriginalUri(uri): OriginalUri,
    request: Request,
    next: Next,
) -> Response {
    // This runs inside a router nested under `/api/v1`, which is stripped
    // from `request.uri()`; the module paths are written in full.
    if let Some(module) = ModuleRegistry::owner_of(uri.path())
        && !registry.is_enabled(module)
    {
        return error_response(
            StatusCode::NOT_FOUND,
            "module_disabled",
            format!("the {module} module is switched off on this host"),
        );
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::{ModuleRegistry, disabled_from_file, resolve};

    fn config_file(name: &str, contents: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("deckox-server-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        std::fs::write(&path, contents).expect("write");
        path
    }

    #[test]
    fn server_toml_lists_the_modules_to_switch_off() {
        let listed = config_file(
            "a.toml",
            "listen_addr = \"x\"\n[modules]\ndisabled = [\"audit\", \"realtime\"]\n",
        );
        assert_eq!(
            disabled_from_file(&listed).expect("parsed"),
            ["audit", "realtime"]
        );
        let bare = config_file("b.toml", "listen_addr = \"x\"\n");
        assert!(disabled_from_file(&bare).expect("parsed").is_empty());
        let missing = std::path::Path::new("/nonexistent/server.toml");
        assert!(
            disabled_from_file(missing)
                .expect("missing is fine")
                .is_empty()
        );
    }

    #[test]
    fn a_broken_server_toml_is_refused() {
        let broken = config_file("c.toml", "[modules\ndisabled = [");
        assert!(disabled_from_file(&broken).is_err());
        let wrong_type = config_file("d.toml", "[modules]\ndisabled = \"audit\"\n");
        assert!(disabled_from_file(&wrong_type).is_err());
        let wrong_item = config_file("e.toml", "[modules]\ndisabled = [1]\n");
        assert!(disabled_from_file(&wrong_item).is_err());
    }

    #[test]
    fn the_environment_overrides_the_file_only_when_it_names_a_module() {
        let file = || vec!["audit".to_owned()];
        assert_eq!(resolve(None, file()), ["audit"]);
        assert_eq!(resolve(Some(""), file()), ["audit"]);
        assert_eq!(resolve(Some(" , "), file()), ["audit"]);
        assert_eq!(
            resolve(Some("realtime, notifications"), file()),
            ["realtime", "notifications"]
        );
    }

    #[test]
    fn paths_map_to_their_module() {
        assert_eq!(ModuleRegistry::owner_of("/api/v1/audit"), Some("audit"));
        assert_eq!(
            ModuleRegistry::owner_of("/api/v1/audit/report"),
            Some("audit")
        );
        assert_eq!(
            ModuleRegistry::owner_of("/api/v1/events/metrics"),
            Some("realtime")
        );
        assert_eq!(
            ModuleRegistry::owner_of("/api/v1/update"),
            Some("update-check")
        );
        assert_eq!(
            ModuleRegistry::owner_of("/api/v1/system/update"),
            None,
            "that one is the Agent's update module"
        );
        assert_eq!(
            ModuleRegistry::owner_of("/api/v1/events"),
            None,
            "the event feed is core"
        );
        assert_eq!(ModuleRegistry::owner_of("/api/v1/auth/login"), None);
    }

    #[test]
    fn a_typo_in_the_environment_list_is_refused() {
        assert!(ModuleRegistry::new(&["audti".to_owned()]).is_err());
        let registry = ModuleRegistry::new(&["audit".to_owned()]).expect("valid");
        assert!(!registry.is_enabled("audit"));
        assert!(registry.is_enabled("notifications"));
        assert_eq!(
            registry
                .infos()
                .iter()
                .filter(|module| !module.enabled)
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn the_gate_works_inside_a_router_nested_under_the_api_prefix() {
        use axum::{Router, middleware, routing::get};

        use super::gate;

        let registry = ModuleRegistry::new(&["audit".to_owned()]).expect("valid");
        let api = Router::new()
            .route("/audit", get(|| async { "audit" }))
            .route("/update", get(|| async { "update" }))
            .route_layer(middleware::from_fn_with_state(registry, gate));
        let app = Router::new().nest("/api/v1", api);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let address = listener.local_addr().expect("address");
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        let client = reqwest::Client::new();
        let off = client
            .get(format!("http://{address}/api/v1/audit"))
            .send()
            .await
            .expect("request");
        assert_eq!(off.status(), 404);
        let body: serde_json::Value =
            serde_json::from_str(&off.text().await.expect("text")).expect("json");
        assert_eq!(body["code"], "module_disabled");

        let on = client
            .get(format!("http://{address}/api/v1/update"))
            .send()
            .await
            .expect("request");
        assert_eq!(on.status(), 200);
    }
}
