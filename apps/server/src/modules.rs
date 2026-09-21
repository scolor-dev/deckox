//! The Server's own switchable modules — the parts that live on the Server
//! rather than being passed through to the Agent. Switch them off with
//! `DECKOX_DISABLED_MODULES=audit,notifications` (comma-separated); a module
//! that is off answers `404 module_disabled` and starts no background work.
//! The Agent's modules are switched in `agent.toml` and reach the Web through
//! `GET /api/v1/modules`.

use std::{collections::HashSet, env, sync::Arc};

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
        let disabled: Vec<String> = env::var("DECKOX_DISABLED_MODULES")
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect();
        Self::new(&disabled)
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
    use super::ModuleRegistry;

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
