//! The Agent's modules: each switchable feature and the request paths it
//! owns. A module that is switched off in `agent.toml` (`[modules] disabled`)
//! answers `404`, and the background work that only it needs is not started.
//! Paths that belong to no module (health, info, events, jobs, modules) are
//! the always-on core.
//!
//! `/v1/system/capabilities` is core although it sits under `/v1/system`: it
//! only reports what the `power` and `update` modules allow, so it must keep
//! answering when `system` is off and must not depend on that module.

use std::{collections::HashSet, sync::Arc};

use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use deckox_protocol::{
    ModuleDefinition, ModuleManifest, module_infos, module_owner, validate_disabled,
};
use serde_json::json;

const DEFINITIONS: &[ModuleDefinition] = &[
    ModuleDefinition {
        id: "system",
        requires: &[],
        routes: &["/v1/system"],
    },
    ModuleDefinition {
        id: "power",
        requires: &[],
        routes: &["/v1/system/reboot"],
    },
    ModuleDefinition {
        id: "update",
        requires: &[],
        routes: &["/v1/system/update"],
    },
    ModuleDefinition {
        id: "storage",
        requires: &[],
        routes: &["/v1/storage"],
    },
    ModuleDefinition {
        id: "diagnostics",
        requires: &[],
        routes: &["/v1/diagnostics"],
    },
    ModuleDefinition {
        id: "backups",
        requires: &[],
        routes: &["/v1/backups"],
    },
    ModuleDefinition {
        id: "services",
        requires: &[],
        routes: &["/v1/services"],
    },
    ModuleDefinition {
        id: "schedules",
        requires: &["services"],
        routes: &["/v1/schedules"],
    },
    ModuleDefinition {
        id: "software",
        requires: &[],
        routes: &["/v1/software"],
    },
];

/// Paths inside a module's prefix that belong to no module.
const CORE_PATHS: &[&str] = &["/v1/system/capabilities"];

#[derive(Clone)]
pub struct ModuleRegistry {
    disabled: Arc<HashSet<String>>,
}

impl ModuleRegistry {
    /// Builds the registry from the `disabled` list, refusing a typo or a
    /// module whose dependency is switched off.
    pub fn new(disabled: &[String]) -> Result<Self, String> {
        Ok(Self {
            disabled: Arc::new(validate_disabled(DEFINITIONS, disabled)?),
        })
    }

    pub fn is_enabled(&self, id: &str) -> bool {
        !self.disabled.contains(id)
    }

    /// The module that owns `path`, if any (the longest prefix match).
    pub fn owner_of(path: &str) -> Option<&'static str> {
        if CORE_PATHS.contains(&path) {
            return None;
        }
        module_owner(DEFINITIONS, path)
    }

    pub fn manifest(&self) -> ModuleManifest {
        ModuleManifest {
            modules: module_infos(DEFINITIONS, &self.disabled),
        }
    }
}

/// Answers `404 module_disabled` for a path whose module is switched off.
pub async fn gate(
    State(registry): State<ModuleRegistry>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(module) = ModuleRegistry::owner_of(request.uri().path())
        && !registry.is_enabled(module)
    {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "code": "module_disabled",
                "message": format!("the {module} module is switched off on this host"),
            })),
        )
            .into_response();
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::ModuleRegistry;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn paths_belong_to_the_most_specific_module() {
        assert_eq!(ModuleRegistry::owner_of("/v1/system"), Some("system"));
        assert_eq!(
            ModuleRegistry::owner_of("/v1/system/metrics"),
            Some("system")
        );
        assert_eq!(ModuleRegistry::owner_of("/v1/system/reboot"), Some("power"));
        assert_eq!(
            ModuleRegistry::owner_of("/v1/system/update"),
            Some("update")
        );
        assert_eq!(
            ModuleRegistry::owner_of("/v1/services/a.service/logs"),
            Some("services")
        );
        assert_eq!(
            ModuleRegistry::owner_of("/v1/storage/disks"),
            Some("storage")
        );
        assert_eq!(
            ModuleRegistry::owner_of("/v1/software/installed"),
            Some("software")
        );
    }

    #[test]
    fn core_paths_and_lookalikes_belong_to_no_module() {
        for path in [
            "/v1/health",
            "/v1/info",
            "/v1/events",
            "/v1/jobs",
            "/v1/modules",
            "/v1/system/capabilities",
        ] {
            assert_eq!(ModuleRegistry::owner_of(path), None, "{path}");
        }
        assert_eq!(
            ModuleRegistry::owner_of("/v1/servicesx"),
            None,
            "a prefix must end at a path boundary"
        );
    }

    #[test]
    fn everything_is_on_unless_disabled() {
        let registry = ModuleRegistry::new(&[]).expect("valid");
        assert!(
            registry
                .manifest()
                .modules
                .iter()
                .all(|module| module.enabled)
        );

        let registry = ModuleRegistry::new(&strings(&["software", "power"])).expect("valid");
        assert!(!registry.is_enabled("software"));
        assert!(registry.is_enabled("services"));
        let manifest = registry.manifest();
        let software = manifest
            .modules
            .iter()
            .find(|module| module.id == "software")
            .expect("listed");
        assert!(!software.enabled);
    }

    #[test]
    fn a_typo_is_refused_with_the_list_of_modules() {
        let error = ModuleRegistry::new(&strings(&["sofware"]))
            .err()
            .expect("refused");
        assert!(error.contains("unknown module \"sofware\""), "{error}");
        assert!(error.contains("software"), "{error}");
    }

    #[test]
    fn a_module_cannot_stay_on_without_its_dependency() {
        let error = ModuleRegistry::new(&strings(&["services"]))
            .err()
            .expect("refused");
        assert!(error.contains("schedules"), "{error}");
        assert!(ModuleRegistry::new(&strings(&["services", "schedules"])).is_ok());
    }
}
