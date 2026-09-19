//! `deckox-agent config <subcommand>` — edit `/etc/deckox/agent.toml` from an
//! SSH console without hand-editing the file. Each change goes through the
//! same table-aware writers the Web UI uses, so the other tables and their
//! comments survive. Run it as root (the file is owned by root); the running
//! Agent reads the file only at start, so changes need a restart.

use std::path::Path;

use crate::{
    config::{
        AgentConfig, read_file_config, repair_config, write_allowed_services,
        write_software_settings, write_system_settings,
    },
    services::{PROTECTED_SERVICES, validate_service_id},
    software::validate_package_name,
};

const USAGE: &str = "\
Usage: deckox-agent config <command>

  show                             print the settings stored in the file
  set allow_reboot <true|false>    allow the Web UI to reboot the host
  set allow_update <true|false>    allow the Web UI to update Deckox
  set auto_adopt <true|false>      manage software you installed yourself automatically
  allow-service <id>               allow start/stop/restart of a service (e.g. nginx.service)
  deny-service <id>                remove a service from the allowlist
  allow-software <name>            manage a package (e.g. docker-ce)
  deny-software <name>             stop managing a package and do not adopt it again
  repair                           rewrite the file in its standard layout, restoring
                                   missing tables (the old file is kept as agent.toml.bak)

Changes take effect after: sudo systemctl restart deckox-agent";

/// Handles `deckox-agent config ...`. Returns `false` when the command line is
/// not a `config` command, so `main` can go on to start the Agent.
pub fn dispatch(arguments: &[String]) -> bool {
    if arguments.first().map(String::as_str) != Some("config") {
        return false;
    }
    let rest: Vec<&str> = arguments[1..].iter().map(String::as_str).collect();
    match run(&AgentConfig::resolve_path(), &rest) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
    true
}

pub fn run(path: &Path, arguments: &[&str]) -> Result<String, String> {
    let restart = "Restart the Agent for this to take effect: sudo systemctl restart deckox-agent";
    let describe = |error: crate::error::AgentError| format!("{error:?}");

    match arguments {
        [] | ["help" | "--help" | "-h"] => Ok(USAGE.to_owned()),
        ["show"] => Ok(show(path).map_err(describe)?),
        ["set", key, value] => {
            let value = parse_bool(value)?;
            let mut config = read_file_config(path).map_err(describe)?;
            match *key {
                "allow_reboot" => {
                    config.system.allow_reboot = value;
                    write_system_settings(path, &config.system).map_err(describe)?;
                }
                "allow_update" => {
                    config.system.allow_update = value;
                    write_system_settings(path, &config.system).map_err(describe)?;
                }
                "auto_adopt" => {
                    config.software.auto_adopt = value;
                    write_software_settings(path, &config.software).map_err(describe)?;
                }
                other => return Err(format!("unknown setting: {other}\n\n{USAGE}")),
            }
            Ok(format!("{key} = {value}\n{restart}"))
        }
        ["allow-service", id] => {
            validate_service_id(id).map_err(describe)?;
            if PROTECTED_SERVICES.contains(id) {
                return Err("Deckox's own services cannot be added to the allowlist".to_owned());
            }
            let mut allowed = read_file_config(path).map_err(describe)?.services.allowed;
            if !allowed.iter().any(|existing| existing == id) {
                allowed.push((*id).to_owned());
            }
            write_allowed_services(path, &allowed).map_err(describe)?;
            Ok(format!("{id} added to the service allowlist\n{restart}"))
        }
        ["deny-service", id] => {
            let mut allowed = read_file_config(path).map_err(describe)?.services.allowed;
            allowed.retain(|existing| existing != id);
            write_allowed_services(path, &allowed).map_err(describe)?;
            Ok(format!(
                "{id} removed from the service allowlist\n{restart}"
            ))
        }
        ["allow-software", name] => {
            validate_package_name(name).map_err(describe)?;
            let mut software = read_file_config(path).map_err(describe)?.software;
            if !software.allowed.iter().any(|existing| existing == name) {
                software.allowed.push((*name).to_owned());
            }
            software.dismissed.retain(|existing| existing != name);
            write_software_settings(path, &software).map_err(describe)?;
            Ok(format!("{name} added to software management\n{restart}"))
        }
        ["deny-software", name] => {
            validate_package_name(name).map_err(describe)?;
            let mut software = read_file_config(path).map_err(describe)?.software;
            software.allowed.retain(|existing| existing != name);
            if !software.dismissed.iter().any(|existing| existing == name) {
                software.dismissed.push((*name).to_owned());
            }
            write_software_settings(path, &software).map_err(describe)?;
            Ok(format!(
                "{name} removed from software management\n{restart}"
            ))
        }
        ["repair"] => {
            repair_config(path).map_err(describe)?;
            Ok(format!(
                "Rewrote {} in its standard layout (previous file: {}).\n{restart}",
                path.display(),
                path.with_extension("toml.bak").display()
            ))
        }
        _ => Err(format!("unrecognized command\n\n{USAGE}")),
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "true" | "on" | "yes" | "1" => Ok(true),
        "false" | "off" | "no" | "0" => Ok(false),
        other => Err(format!("expected true or false, got: {other}")),
    }
}

fn show(path: &Path) -> Result<String, crate::error::AgentError> {
    let config = read_file_config(path)?;
    let list = |items: &[String]| {
        if items.is_empty() {
            "(none)".to_owned()
        } else {
            items.join(", ")
        }
    };
    Ok(format!(
        "{}\n\nallow_reboot: {}\nallow_update: {}\nauto_adopt: {}\nsoftware allowed: {}\nsoftware dismissed: {}\nservices allowed: {}",
        path.display(),
        config.system.allow_reboot,
        config.system.allow_update,
        config.software.auto_adopt,
        list(&config.software.allowed),
        list(&config.software.dismissed),
        list(&config.services.allowed),
    ))
}

#[cfg(test)]
mod tests {
    use super::run;

    fn temp_config(contents: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "deckox-config-cli-{}-{}",
            std::process::id(),
            contents.len()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("agent.toml");
        std::fs::write(&path, contents).expect("write config");
        path
    }

    #[test]
    fn set_edits_one_table_and_keeps_the_others() {
        let path = temp_config(include_str!("../../../packaging/config/agent.toml"));
        run(&path, &["allow-service", "nginx.service"]).expect("allow service");
        run(&path, &["allow-software", "docker-ce"]).expect("allow software");
        run(&path, &["set", "allow_reboot", "true"]).expect("set reboot");
        run(&path, &["set", "auto_adopt", "false"]).expect("set auto_adopt");

        let shown = run(&path, &["show"]).expect("show");
        assert!(shown.contains("allow_reboot: true"), "{shown}");
        assert!(shown.contains("allow_update: false"), "{shown}");
        assert!(shown.contains("auto_adopt: false"), "{shown}");
        assert!(shown.contains("software allowed: docker-ce"), "{shown}");
        assert!(shown.contains("services allowed: nginx.service"), "{shown}");
    }

    #[test]
    fn deny_software_dismisses_and_allow_lifts_it() {
        let path = temp_config("[software]\nallowed = [\"git\"]\n");
        run(&path, &["deny-software", "git"]).expect("deny");
        let shown = run(&path, &["show"]).expect("show");
        assert!(shown.contains("software allowed: (none)"), "{shown}");
        assert!(shown.contains("software dismissed: git"), "{shown}");
        run(&path, &["allow-software", "git"]).expect("allow");
        let shown = run(&path, &["show"]).expect("show");
        assert!(shown.contains("software dismissed: (none)"), "{shown}");
    }

    #[test]
    fn repair_restores_missing_tables_from_a_damaged_file() {
        let path = temp_config(
            "socket = \"/run/deckox/agent.sock\"\n\n[services]\nallowed = [\"docker.service\"]\n",
        );
        run(&path, &["repair"]).expect("repair");

        let shown = run(&path, &["show"]).expect("show");
        assert!(
            shown.contains("services allowed: docker.service"),
            "{shown}"
        );
        assert!(shown.contains("auto_adopt: true"), "{shown}");
        let content = std::fs::read_to_string(&path).expect("read");
        for header in ["[system]", "[software]", "[services]"] {
            assert_eq!(
                content.matches(&format!("\n{header}\n")).count(),
                1,
                "{content}"
            );
        }
        assert!(
            path.with_extension("toml.bak").exists(),
            "the old file is kept"
        );
    }

    #[test]
    fn rejects_bad_input() {
        let path = temp_config("");
        assert!(run(&path, &["set", "allow_reboot", "maybe"]).is_err());
        assert!(run(&path, &["set", "nonsense", "true"]).is_err());
        assert!(run(&path, &["allow-service", "deckox-agent.service"]).is_err());
        assert!(run(&path, &["allow-software", "bad name"]).is_err());
        assert!(run(&path, &["frobnicate"]).is_err());
    }
}
