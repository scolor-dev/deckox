use std::{
    fs::File,
    io::{Read, Write},
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    sync::Arc,
};

use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use deckox_protocol::{SshKeyList, SshKeySummary};
use rustix::{
    fs::{
        AtFlags, FileType, Gid, Mode, OFlags, Uid, fchmod, fchown, fstat, fsync, mkdirat, open,
        openat, renameat, unlinkat,
    },
    io::Errno,
};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;

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

#[derive(Clone)]
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
        let managed_user = target.username.clone();
        let target = Arc::clone(target);
        let document = tokio::task::spawn_blocking(move || read_document(&target))
            .await
            .map_err(|error| {
                AgentError::internal(format!("SSH key read task failed: {error}"))
            })??;
        Ok(SshKeyList {
            enabled: true,
            managed_user: Some(managed_user),
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
        let target = target.clone();
        tokio::task::spawn_blocking(move || {
            let directory = open_ssh_directory(&target, true)?.ok_or_else(|| {
                AgentError::internal("failed to create managed user's .ssh directory")
            })?;
            let mut document = read_document_from(&directory, target.uid)?;
            if document
                .managed
                .iter()
                .any(|existing| existing.summary.fingerprint == key.summary.fingerprint)
            {
                return Err(AgentError::conflict("SSH public key is already managed"));
            }

            document.managed.push(key.clone());
            write_document_to(&directory, &target, &document)?;
            Ok(key.summary)
        })
        .await
        .map_err(|error| AgentError::internal(format!("SSH key update task failed: {error}")))?
    }

    pub async fn remove(&self, key_id: &str) -> Result<SshKeySummary, AgentError> {
        if key_id.len() != 64 || !key_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AgentError::bad_request("invalid SSH key ID"));
        }
        let target = self.target()?;
        let _write_guard = self.write_lock.lock().await;
        let target = target.clone();
        let key_id = key_id.to_owned();
        tokio::task::spawn_blocking(move || {
            let Some(directory) = open_ssh_directory(&target, false)? else {
                return Err(AgentError::not_found("managed SSH public key not found"));
            };
            let mut document = read_document_from(&directory, target.uid)?;
            let Some(index) = document
                .managed
                .iter()
                .position(|key| key.summary.id.eq_ignore_ascii_case(&key_id))
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
            write_document_to(&directory, &target, &document)?;
            Ok(removed.summary)
        })
        .await
        .map_err(|error| AgentError::internal(format!("SSH key update task failed: {error}")))?
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

fn read_document(target: &TargetUser) -> Result<AuthorizedKeysDocument, AgentError> {
    let Some(directory) = open_ssh_directory(target, false)? else {
        return Ok(AuthorizedKeysDocument::empty());
    };
    read_document_from(&directory, target.uid)
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

fn open_ssh_directory(target: &TargetUser, create: bool) -> Result<Option<OwnedFd>, AgentError> {
    let home = open(
        &target.home,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| AgentError::forbidden(format!("failed to open home directory: {error}")))?;

    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut created = false;
    let directory = match openat(&home, ".ssh", flags, Mode::empty()) {
        Ok(directory) => directory,
        Err(Errno::NOENT) if !create => return Ok(None),
        Err(Errno::NOENT) => {
            match mkdirat(&home, ".ssh", Mode::from(0o700)) {
                Ok(()) => created = true,
                Err(Errno::EXIST) => {}
                Err(error) => {
                    return Err(AgentError::internal(format!(
                        "failed to create .ssh directory: {error}"
                    )));
                }
            }
            openat(&home, ".ssh", flags, Mode::empty()).map_err(|error| {
                AgentError::forbidden(format!("failed to safely open .ssh directory: {error}"))
            })?
        }
        Err(error) => {
            return Err(AgentError::forbidden(format!(
                "failed to safely open .ssh directory: {error}"
            )));
        }
    };
    let metadata = fstat(&directory)
        .map_err(|error| AgentError::internal(format!("failed to inspect .ssh: {error}")))?;
    if !FileType::from_raw_mode(metadata.st_mode).is_dir()
        || (!created && metadata.st_uid != target.uid)
    {
        return Err(AgentError::forbidden(
            ".ssh must be a directory owned by the managed user",
        ));
    }
    if created {
        fchown(
            &directory,
            Some(Uid::from_raw(target.uid)),
            Some(Gid::from_raw(target.gid)),
        )
        .map_err(|error| AgentError::internal(format!("failed to own .ssh directory: {error}")))?;
    }
    if create {
        fchmod(&directory, Mode::from(0o700)).map_err(|error| {
            AgentError::internal(format!("failed to secure .ssh directory: {error}"))
        })?;
    }
    Ok(Some(directory))
}

fn read_document_from(
    directory: &OwnedFd,
    expected_uid: u32,
) -> Result<AuthorizedKeysDocument, AgentError> {
    let descriptor = match openat(
        directory,
        "authorized_keys",
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(descriptor) => descriptor,
        Err(Errno::NOENT) => return Ok(AuthorizedKeysDocument::empty()),
        Err(error) => {
            return Err(AgentError::forbidden(format!(
                "failed to safely open authorized_keys: {error}"
            )));
        }
    };
    let metadata = fstat(&descriptor).map_err(|error| {
        AgentError::internal(format!("failed to inspect authorized_keys: {error}"))
    })?;
    if !FileType::from_raw_mode(metadata.st_mode).is_file() || metadata.st_uid != expected_uid {
        return Err(AgentError::forbidden(
            "authorized_keys must be a regular file owned by the managed user",
        ));
    }
    if u64::try_from(metadata.st_size).map_or(true, |size| size > MAX_AUTHORIZED_KEYS_BYTES) {
        return Err(AgentError::bad_request("authorized_keys is too large"));
    }

    let mut content = String::new();
    File::from(descriptor)
        .take(MAX_AUTHORIZED_KEYS_BYTES + 1)
        .read_to_string(&mut content)
        .map_err(|error| {
            AgentError::bad_request(format!("authorized_keys is not valid UTF-8: {error}"))
        })?;
    if content.len() as u64 > MAX_AUTHORIZED_KEYS_BYTES {
        return Err(AgentError::bad_request("authorized_keys is too large"));
    }
    AuthorizedKeysDocument::parse(&content)
}

fn write_document_to(
    directory: &OwnedFd,
    target: &TargetUser,
    document: &AuthorizedKeysDocument,
) -> Result<(), AgentError> {
    let temporary_name = format!(
        ".authorized_keys.deckox.{}",
        hex::encode(rand::random::<[u8; 8]>())
    );
    let result = (|| {
        let descriptor = openat(
            directory,
            temporary_name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from(0o600),
        )
        .map_err(|error| {
            AgentError::internal(format!("failed to create authorized_keys update: {error}"))
        })?;
        fchmod(&descriptor, Mode::from(0o600)).map_err(|error| {
            AgentError::internal(format!("failed to secure authorized_keys update: {error}"))
        })?;
        fchown(
            &descriptor,
            Some(Uid::from_raw(target.uid)),
            Some(Gid::from_raw(target.gid)),
        )
        .map_err(|error| {
            AgentError::internal(format!("failed to own authorized_keys update: {error}"))
        })?;
        let mut file = File::from(descriptor);
        file.write_all(document.render().as_bytes())
            .map_err(|error| {
                AgentError::internal(format!("failed to write authorized_keys: {error}"))
            })?;
        file.sync_all().map_err(|error| {
            AgentError::internal(format!("failed to sync authorized_keys: {error}"))
        })?;
        drop(file);
        renameat(
            directory,
            temporary_name.as_str(),
            directory,
            "authorized_keys",
        )
        .map_err(|error| {
            AgentError::internal(format!("failed to replace authorized_keys: {error}"))
        })?;
        fsync(directory).map_err(|error| {
            AgentError::internal(format!("failed to sync .ssh directory: {error}"))
        })
    })();
    if result.is_err() {
        let _ = unlinkat(directory, temporary_name.as_str(), AtFlags::empty());
    }
    result
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        os::unix::fs::{MetadataExt, PermissionsExt, symlink},
    };

    use super::{
        AuthorizedKeysDocument, END_MARKER, START_MARKER, TargetUser, open_ssh_directory,
        parse_public_key, read_document_from, write_document_to,
    };

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

    #[test]
    fn writes_authorized_keys_through_directory_descriptor() {
        let (root, target) = temporary_target();
        let directory = open_ssh_directory(&target, true)
            .expect("directory should open")
            .expect("directory should exist");
        let key = parse_public_key(ED25519_KEY).expect("key should parse");
        let document = AuthorizedKeysDocument {
            before: vec!["# existing".to_owned()],
            managed: vec![key],
            after: Vec::new(),
        };

        write_document_to(&directory, &target, &document).expect("document should write");
        let loaded = read_document_from(&directory, target.uid).expect("document should read");
        assert_eq!(loaded.managed.len(), 1);
        let metadata = fs::metadata(target.home.join(".ssh/authorized_keys"))
            .expect("authorized_keys should exist");
        assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
        assert_eq!(metadata.uid(), target.uid);

        fs::remove_dir_all(root).expect("temporary directory should be removed");
    }

    #[test]
    fn rejects_symlinked_ssh_directory() {
        let (root, target) = temporary_target();
        let redirected = root.join("redirected");
        fs::create_dir(&redirected).expect("redirect target should be created");
        symlink(&redirected, target.home.join(".ssh")).expect("symlink should be created");

        assert!(open_ssh_directory(&target, false).is_err());

        fs::remove_dir_all(root).expect("temporary directory should be removed");
    }

    fn temporary_target() -> (std::path::PathBuf, TargetUser) {
        let root = std::env::temp_dir().join(format!(
            "deckox-ssh-test-{}",
            hex::encode(rand::random::<[u8; 8]>())
        ));
        let home = root.join("home");
        fs::create_dir_all(&home).expect("temporary home should be created");
        let metadata = fs::metadata(&home).expect("temporary home should be readable");
        let target = TargetUser {
            username: "test-user".to_owned(),
            home,
            uid: metadata.uid(),
            gid: metadata.gid(),
        };
        (root, target)
    }
}
