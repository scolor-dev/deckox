use std::{
    os::unix::fs::{MetadataExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use deckox_protocol::{SshKeyList, SshKeySummary};
use sha2::{Digest, Sha256};
use tokio::{io::AsyncWriteExt, process::Command, sync::Mutex};

use crate::error::AgentError;

const START_MARKER: &str = "# deckox-managed:start";
const END_MARKER: &str = "# deckox-managed:end";
const MAX_AUTHORIZED_KEYS_BYTES: u64 = 1024 * 1024;
const MAX_PUBLIC_KEY_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub struct SshKeyManager {
    target: Option<Arc<TargetUser>>,
    write_lock: Arc<Mutex<()>>,
}

struct TargetUser {
    username: String,
    home: PathBuf,
    uid: u32,
    gid: u32,
}

#[derive(Clone)]
struct PublicKey {
    normalized: String,
    summary: SshKeySummary,
}

struct AuthorizedKeysDocument {
    before: Vec<String>,
    managed: Vec<PublicKey>,
    after: Vec<String>,
}

impl SshKeyManager {
    pub fn new(managed_user: Option<String>) -> Result<Self, AgentError> {
        let target = managed_user
            .map(|username| resolve_user(&username).map(Arc::new))
            .transpose()?;
        Ok(Self {
            target,
            write_lock: Arc::new(Mutex::new(())),
        })
    }

    pub async fn list(&self) -> Result<SshKeyList, AgentError> {
        let Some(target) = &self.target else {
            return Ok(SshKeyList {
                enabled: false,
                managed_user: None,
                keys: Vec::new(),
            });
        };
        let document = read_document(target).await?;
        Ok(SshKeyList {
            enabled: true,
            managed_user: Some(target.username.clone()),
            keys: document
                .managed
                .into_iter()
                .map(|key| key.summary)
                .collect(),
        })
    }

    pub async fn add(&self, public_key: &str) -> Result<SshKeySummary, AgentError> {
        let target = self.target()?;
        let key = parse_public_key(public_key)?;
        let _write_guard = self.write_lock.lock().await;
        let mut document = read_document(target).await?;
        if document
            .managed
            .iter()
            .any(|existing| existing.summary.fingerprint == key.summary.fingerprint)
        {
            return Err(AgentError::conflict("SSH public key is already managed"));
        }

        document.managed.push(key.clone());
        write_document(target, &document).await?;
        Ok(key.summary)
    }

    pub async fn remove(&self, key_id: &str) -> Result<SshKeySummary, AgentError> {
        if key_id.len() != 64 || !key_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AgentError::bad_request("invalid SSH key ID"));
        }
        let target = self.target()?;
        let _write_guard = self.write_lock.lock().await;
        let mut document = read_document(target).await?;
        let Some(index) = document
            .managed
            .iter()
            .position(|key| key.summary.id.eq_ignore_ascii_case(key_id))
        else {
            return Err(AgentError::not_found("managed SSH public key not found"));
        };

        let external_keys = document
            .before
            .iter()
            .chain(&document.after)
            .filter(|line| is_key_line(line))
            .count();
        if document.managed.len() + external_keys <= 1 {
            return Err(AgentError::conflict(
                "refusing to remove the last SSH public key",
            ));
        }

        let removed = document.managed.remove(index);
        write_document(target, &document).await?;
        Ok(removed.summary)
    }

    fn target(&self) -> Result<&TargetUser, AgentError> {
        self.target
            .as_deref()
            .ok_or_else(|| AgentError::unavailable("SSH public-key management is not configured"))
    }
}

fn resolve_user(username: &str) -> Result<TargetUser, AgentError> {
    if username == "root" || !valid_username(username) {
        return Err(AgentError::bad_request(
            "SSH managed_user must be a valid non-root Linux account",
        ));
    }
    let passwd = std::fs::read_to_string("/etc/passwd")
        .map_err(|error| AgentError::internal(format!("failed to read /etc/passwd: {error}")))?;
    let fields = passwd
        .lines()
        .find_map(|line| {
            let fields = line.split(':').collect::<Vec<_>>();
            (fields.first() == Some(&username) && fields.len() >= 7).then_some(fields)
        })
        .ok_or_else(|| AgentError::bad_request(format!("Linux user {username} does not exist")))?;
    let uid = fields[2]
        .parse::<u32>()
        .map_err(|_| AgentError::internal("managed user has an invalid UID"))?;
    let gid = fields[3]
        .parse::<u32>()
        .map_err(|_| AgentError::internal("managed user has an invalid GID"))?;
    if uid == 0 {
        return Err(AgentError::bad_request(
            "SSH public-key management for UID 0 is not allowed",
        ));
    }
    let home = PathBuf::from(fields[5]);
    if !home.is_absolute() || home == Path::new("/") {
        return Err(AgentError::bad_request(
            "managed user has an unsafe home directory",
        ));
    }
    Ok(TargetUser {
        username: username.to_owned(),
        home,
        uid,
        gid,
    })
}

fn valid_username(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 32
        && username
            .as_bytes()
            .first()
            .is_some_and(|byte| byte.is_ascii_lowercase() || *byte == b'_')
        && username.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

async fn read_document(target: &TargetUser) -> Result<AuthorizedKeysDocument, AgentError> {
    let ssh_directory = target.home.join(".ssh");
    match tokio::fs::symlink_metadata(&ssh_directory).await {
        Ok(metadata) => {
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(AgentError::forbidden("refusing unsafe .ssh directory"));
            }
            if metadata.uid() != target.uid {
                return Err(AgentError::forbidden(
                    ".ssh directory is not owned by the managed user",
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AuthorizedKeysDocument::empty());
        }
        Err(error) => {
            return Err(AgentError::internal(format!(
                "failed to inspect .ssh directory: {error}"
            )));
        }
    }

    let path = ssh_directory.join("authorized_keys");
    let metadata = match tokio::fs::symlink_metadata(&path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(AuthorizedKeysDocument::empty());
        }
        Err(error) => {
            return Err(AgentError::internal(format!(
                "failed to inspect authorized_keys: {error}"
            )));
        }
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(AgentError::forbidden(
            "refusing unsafe authorized_keys path",
        ));
    }
    if metadata.uid() != target.uid {
        return Err(AgentError::forbidden(
            "authorized_keys is not owned by the managed user",
        ));
    }
    if metadata.len() > MAX_AUTHORIZED_KEYS_BYTES {
        return Err(AgentError::bad_request("authorized_keys is too large"));
    }
    let content = tokio::fs::read_to_string(&path).await.map_err(|error| {
        AgentError::internal(format!("failed to read authorized_keys: {error}"))
    })?;
    AuthorizedKeysDocument::parse(&content)
}

impl AuthorizedKeysDocument {
    const fn empty() -> Self {
        Self {
            before: Vec::new(),
            managed: Vec::new(),
            after: Vec::new(),
        }
    }

    fn parse(content: &str) -> Result<Self, AgentError> {
        let lines = content.lines().map(str::to_owned).collect::<Vec<_>>();
        let starts = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.trim() == START_MARKER)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let ends = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.trim() == END_MARKER)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();

        match (starts.as_slice(), ends.as_slice()) {
            ([], []) => Ok(Self {
                before: lines,
                managed: Vec::new(),
                after: Vec::new(),
            }),
            ([start], [end]) if start < end => {
                let managed = lines[start + 1..*end]
                    .iter()
                    .filter(|line| is_key_line(line))
                    .map(|line| parse_public_key(line))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self {
                    before: lines[..*start].to_vec(),
                    managed,
                    after: lines[end + 1..].to_vec(),
                })
            }
            _ => Err(AgentError::conflict(
                "authorized_keys contains malformed Deckox markers",
            )),
        }
    }

    fn render(&self) -> String {
        let mut lines = self.before.clone();
        lines.push(START_MARKER.to_owned());
        lines.extend(self.managed.iter().map(|key| key.normalized.clone()));
        lines.push(END_MARKER.to_owned());
        lines.extend(self.after.clone());
        format!("{}\n", lines.join("\n"))
    }
}

fn parse_public_key(input: &str) -> Result<PublicKey, AgentError> {
    if input.is_empty() || input.len() > MAX_PUBLIC_KEY_BYTES || input.contains(['\r', '\n', '\0'])
    {
        return Err(AgentError::bad_request("invalid SSH public key"));
    }
    let mut fields = input.trim().splitn(3, char::is_whitespace);
    let key_type = fields.next().unwrap_or_default();
    let encoded = fields.next().unwrap_or_default();
    let comment = fields
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if !supported_key_type(key_type) || encoded.is_empty() {
        return Err(AgentError::bad_request(
            "unsupported or malformed SSH public key",
        ));
    }
    let blob = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| AgentError::bad_request("SSH public key is not valid base64"))?;
    let embedded_type = ssh_string(&blob)
        .ok_or_else(|| AgentError::bad_request("SSH public key blob is malformed"))?;
    if embedded_type != key_type.as_bytes() {
        return Err(AgentError::bad_request(
            "SSH public key type does not match its encoded data",
        ));
    }

    let fingerprint = format!("SHA256:{}", STANDARD_NO_PAD.encode(Sha256::digest(&blob)));
    let normalized = comment.map_or_else(
        || format!("{key_type} {encoded}"),
        |comment| format!("{key_type} {encoded} {comment}"),
    );
    let id = hex::encode(Sha256::digest(normalized.as_bytes()));
    Ok(PublicKey {
        normalized,
        summary: SshKeySummary {
            id,
            key_type: key_type.to_owned(),
            fingerprint,
            comment: comment.map(str::to_owned),
        },
    })
}

fn ssh_string(blob: &[u8]) -> Option<&[u8]> {
    let length = u32::from_be_bytes(blob.get(..4)?.try_into().ok()?) as usize;
    blob.get(4..4 + length)
}

fn supported_key_type(key_type: &str) -> bool {
    matches!(
        key_type,
        "ssh-ed25519"
            | "ssh-rsa"
            | "ecdsa-sha2-nistp256"
            | "ecdsa-sha2-nistp384"
            | "ecdsa-sha2-nistp521"
            | "sk-ssh-ed25519@openssh.com"
            | "sk-ecdsa-sha2-nistp256@openssh.com"
    )
}

fn is_key_line(line: &str) -> bool {
    let line = line.trim();
    !line.is_empty() && !line.starts_with('#')
}

async fn write_document(
    target: &TargetUser,
    document: &AuthorizedKeysDocument,
) -> Result<(), AgentError> {
    let ssh_directory = target.home.join(".ssh");
    match tokio::fs::symlink_metadata(&ssh_directory).await {
        Ok(metadata)
            if metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == target.uid =>
        {
            tokio::fs::set_permissions(&ssh_directory, std::fs::Permissions::from_mode(0o700))
                .await
                .map_err(|error| {
                    AgentError::internal(format!("failed to secure .ssh directory: {error}"))
                })?;
        }
        Ok(_) => return Err(AgentError::forbidden("refusing unsafe .ssh directory")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            tokio::fs::create_dir(&ssh_directory)
                .await
                .map_err(|error| {
                    AgentError::internal(format!("failed to create .ssh directory: {error}"))
                })?;
            tokio::fs::set_permissions(&ssh_directory, std::fs::Permissions::from_mode(0o700))
                .await
                .map_err(|error| {
                    AgentError::internal(format!("failed to secure .ssh directory: {error}"))
                })?;
            chown(&ssh_directory, target.uid, target.gid).await?;
        }
        Err(error) => {
            return Err(AgentError::internal(format!(
                "failed to inspect .ssh directory: {error}"
            )));
        }
    }

    let path = ssh_directory.join("authorized_keys");
    let temporary_path = ssh_directory.join(format!(
        ".authorized_keys.deckox.{}",
        hex::encode(rand::random::<[u8; 8]>())
    ));
    let result = async {
        let mut options = tokio::fs::OpenOptions::new();
        options.create_new(true).write(true).mode(0o600);
        let mut file = options.open(&temporary_path).await.map_err(|error| {
            AgentError::internal(format!("failed to create authorized_keys update: {error}"))
        })?;
        file.write_all(document.render().as_bytes())
            .await
            .map_err(|error| {
                AgentError::internal(format!("failed to write authorized_keys: {error}"))
            })?;
        file.sync_all().await.map_err(|error| {
            AgentError::internal(format!("failed to sync authorized_keys: {error}"))
        })?;
        drop(file);
        chown(&temporary_path, target.uid, target.gid).await?;
        tokio::fs::rename(&temporary_path, &path)
            .await
            .map_err(|error| {
                AgentError::internal(format!("failed to replace authorized_keys: {error}"))
            })
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&temporary_path).await;
    }
    result
}

async fn chown(path: &Path, uid: u32, gid: u32) -> Result<(), AgentError> {
    let owner = format!("{uid}:{gid}");
    let status = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new("chown")
            .arg("--")
            .arg(owner)
            .arg(path)
            .status(),
    )
    .await
    .map_err(|_| AgentError::internal("chown timed out"))?
    .map_err(|error| AgentError::internal(format!("failed to run chown: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(AgentError::internal("chown failed"))
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthorizedKeysDocument, END_MARKER, START_MARKER, parse_public_key};

    const ED25519_KEY: &str =
        "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA laptop";

    #[test]
    fn parses_key_and_calculates_fingerprint() {
        let key = parse_public_key(ED25519_KEY).expect("key should parse");
        assert_eq!(key.summary.key_type, "ssh-ed25519");
        assert_eq!(key.summary.comment.as_deref(), Some("laptop"));
        assert!(key.summary.fingerprint.starts_with("SHA256:"));
    }

    #[test]
    fn preserves_lines_outside_managed_block() {
        let input = format!("# existing\n{START_MARKER}\n{ED25519_KEY}\n{END_MARKER}\n# footer\n");
        let document = AuthorizedKeysDocument::parse(&input).expect("document should parse");
        assert_eq!(document.managed.len(), 1);
        assert_eq!(document.render(), input);
    }

    #[test]
    fn rejects_authorized_keys_options_for_new_keys() {
        let input = format!("from=\"10.0.0.0/8\" {ED25519_KEY}");
        assert!(parse_public_key(&input).is_err());
    }
}
