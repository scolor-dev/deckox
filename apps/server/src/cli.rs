//! `deckox-server <subcommand>` entry point for SSH-console administration
//! (password recovery, TOTP recovery). [`dispatch`] is the router; each
//! handler function below owns stdin/stdout I/O and calls into the
//! account-mutating service functions in `auth.rs`, which do the actual
//! read-modify-write against the account file. `dispatch` is also the only
//! place that turns an `Err` into a printed message and exit code, so
//! handlers stay free of scattered `process::exit` calls.
//!
//! To add a subcommand: add a match arm in [`dispatch`] and a handler
//! function that returns `Result<(), String>`.

use std::{
    env,
    io::{Read, Write},
    net::SocketAddr,
    path::PathBuf,
};

use deckox_protocol::SystemInfo;

use crate::{agent_client::AgentClient, audit::AuditLog, auth, request_context::RequestId};

pub async fn dispatch(argument: Option<&str>) -> bool {
    let result = match argument {
        Some("hash-password") => hash_password(),
        Some("reset-password") => reset_password().await,
        Some("disable-totp") => disable_totp().await,
        Some("access-url") => access_url().await,
        _ => return false,
    };
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(2);
    }
    true
}

fn hash_password() -> Result<(), String> {
    let password = read_stdin_password()?;
    println!("{}", auth::hash_password(&password)?);
    Ok(())
}

/// `reset-password` — run from an SSH-connected console when the admin
/// password is lost. Reads the new password from stdin, exactly like
/// `hash-password`, and updates only `password_hash`; TOTP settings are
/// left untouched (see [`disable_totp`] for that recovery path). The
/// running server keeps the old password in memory until restarted, so
/// this always ends with a restart reminder.
async fn reset_password() -> Result<(), String> {
    let account_path = require_account_path("password cannot be reset")?;
    let password = read_stdin_password()?;
    auth::reset_password_cli(&account_path, &password).await?;

    AuditLog::from_env()
        .record_cli("password_reset_cli", "success", None)
        .await;
    println!("Password updated. Restart deckox-server for the change to take effect:");
    println!("  sudo systemctl restart deckox-server");
    Ok(())
}

/// `disable-totp` — the last-resort recovery path when both the
/// authenticator app and every recovery code are lost. Interactive
/// confirmation guards against running it by accident, since it lowers the
/// account's security to password-only.
async fn disable_totp() -> Result<(), String> {
    let account_path = require_account_path("TOTP cannot be changed")?;
    let account = auth::load_admin_account(&account_path)?;
    if account.totp.is_none() {
        println!("Two-factor authentication is not enabled for the admin account.");
        return Ok(());
    }
    if !confirm("Disable two-factor authentication for the admin account? [y/N] ")? {
        println!("Cancelled.");
        return Ok(());
    }

    auth::disable_totp_cli(&account_path).await?;

    AuditLog::from_env()
        .record_cli("totp_disabled_cli", "success", None)
        .await;
    println!(
        "Two-factor authentication disabled. Restart deckox-server for the change to take effect:"
    );
    println!("  sudo systemctl restart deckox-server");
    Ok(())
}

/// `access-url` — prints the URL(s) this instance can be reached at from
/// elsewhere on the LAN, combining the configured listen port with the
/// LAN-facing IPv4 addresses Agent reports for the host's physical network
/// interfaces. Best-effort: if Agent can't be reached or reports no
/// address, still prints the port so there is something to go on.
async fn access_url() -> Result<(), String> {
    let listen_addr = env::var("DECKOX_LISTEN_ADDR")
        .unwrap_or_else(|_| crate::DEFAULT_LISTEN_ADDR.to_owned())
        .parse::<SocketAddr>()
        .map_err(|error| format!("invalid DECKOX_LISTEN_ADDR: {error}"))?;
    let socket_path = PathBuf::from(
        env::var("DECKOX_AGENT_SOCKET").unwrap_or_else(|_| crate::DEFAULT_AGENT_SOCKET.to_owned()),
    );
    let agent = AgentClient::new(socket_path);
    let request_id = RequestId(format!("cli-{}", hex::encode(rand::random::<[u8; 8]>())));

    match agent
        .get_json::<SystemInfo>("/v1/system", &request_id)
        .await
    {
        Ok(system) if !system.lan_addresses.is_empty() => {
            for address in system.lan_addresses {
                println!("http://{address}:{}", listen_addr.port());
            }
        }
        Ok(_) => {
            println!("No LAN-facing IPv4 address was found.");
            println!("Listening on port {}.", listen_addr.port());
        }
        Err(error) => {
            eprintln!("Could not reach deckox-agent to detect the LAN address: {error}");
            println!("Listening on port {}.", listen_addr.port());
        }
    }
    Ok(())
}

/// Resolves the account file path, or an error naming what was being
/// changed when the server is in the non-persistent
/// `DECKOX_ADMIN_PASSWORD_HASH` mode, in which there is no file to edit.
fn require_account_path(prefix: &str) -> Result<PathBuf, String> {
    auth::account_path_from_env().ok_or_else(|| {
        format!(
            "{prefix} while DECKOX_ADMIN_PASSWORD_HASH is configured; \
             remove it and use DECKOX_ADMIN_PASSWORD_HASH_FILE instead"
        )
    })
}

/// Reads up to 1024 bytes from stdin and trims a trailing newline — shared
/// by `hash-password` and `reset-password`.
fn read_stdin_password() -> Result<String, String> {
    let mut password = String::new();
    std::io::stdin()
        .take(1025)
        .read_to_string(&mut password)
        .map_err(|error| format!("failed to read password: {error}"))?;
    Ok(password.trim_end_matches(['\r', '\n']).to_owned())
}

/// Prints `prompt` and reads a y/N confirmation from stdin.
fn confirm(prompt: &str) -> Result<bool, String> {
    print!("{prompt}");
    std::io::stdout()
        .flush()
        .map_err(|error| format!("failed to write prompt: {error}"))?;
    let mut answer = String::new();
    std::io::stdin()
        .read_line(&mut answer)
        .map_err(|error| format!("failed to read confirmation: {error}"))?;
    Ok(matches!(answer.trim(), "y" | "Y" | "yes" | "YES"))
}
