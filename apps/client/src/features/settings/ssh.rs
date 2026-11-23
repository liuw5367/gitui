//! Local OpenSSH key discovery and management, limited to the current user's `~/.ssh`.

use std::fs;
use std::path::{Path, PathBuf};

mod operations;
mod storage;

pub(crate) use operations::{generate_key, import_key, remove_key};
use storage::{
    checked_ssh_directory, parse_public_key, public_fingerprint, read_public_key_file,
    valid_private_path, validate_public_name,
};
#[cfg(all(test, unix))]
use storage::{ssh_keygen_command, unique_token, validate_key_name};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SshKeyEntry {
    pub(crate) name: String,
    pub(crate) path_display: String,
    pub(crate) fingerprint: String,
    pub(crate) public_name: String,
    pub(crate) private_name: Option<String>,
}

impl SshKeyEntry {
    pub(crate) fn removable(&self) -> bool {
        return self.private_name.is_some();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SshError {
    NoHome,
    InvalidName,
    InvalidComment,
    MissingFile,
    NotRegularFile,
    InvalidPublicKey,
    NotMatchingKey,
    NameAlreadyExists,
    KeyChanged,
    Cancelled,
    KeygenFailed,
    Symlink,
    DirectoryUnavailable,
    FileOperation,
}

impl SshError {
    pub(crate) fn message_key(self) -> &'static str {
        return match self {
            Self::NoHome => "settings.sshHomeUnavailable",
            Self::InvalidName => "settings.sshInvalidName",
            Self::InvalidComment => "settings.sshInvalidComment",
            Self::MissingFile => "settings.sshFileMissing",
            Self::NotRegularFile => "settings.sshNotPrivateKey",
            Self::InvalidPublicKey => "settings.sshInvalidPublicKey",
            Self::NotMatchingKey => "settings.sshKeyMismatch",
            Self::NameAlreadyExists => "settings.sshNameExists",
            Self::KeyChanged => "settings.sshKeyChanged",
            Self::Cancelled => "askpass.cancelled",
            Self::KeygenFailed => "settings.sshKeyOperationFailed",
            Self::Symlink => "settings.sshSymlinkRejected",
            Self::DirectoryUnavailable => "settings.sshDirectoryUnavailable",
            Self::FileOperation => "settings.sshFileOperationFailed",
        };
    }
}

/// The production key directory. Tests pass an isolated directory directly to each operation.
pub(crate) fn ssh_dir() -> Result<PathBuf, SshError> {
    let Some(home) = std::env::var_os("HOME") else {
        return Err(SshError::NoHome);
    };
    return Ok(PathBuf::from(home).join(".ssh"));
}

/// List ordinary public keys and their adjacent ordinary private-key files.
/// Standalone `.pub` files remain copyable, read-only entries.
pub(crate) fn read_keys() -> Vec<SshKeyEntry> {
    let Ok(directory) = ssh_dir() else {
        return Vec::new();
    };
    return scan_keys(&directory);
}

fn scan_keys(directory: &Path) -> Vec<SshKeyEntry> {
    let Ok(metadata) = fs::symlink_metadata(directory) else {
        return Vec::new();
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Vec::new();
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };

    let mut keys = Vec::new();
    for entry in entries.flatten() {
        let public_path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&public_path) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let Some(public_name) = public_path
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.ends_with(".pub") && !name.starts_with('.'))
        else {
            continue;
        };
        let Ok(public_text) = read_public_key_file(&public_path) else {
            continue;
        };
        let Ok((_, _)) = parse_public_key(public_text.as_str()) else {
            continue;
        };
        let Some(name) = public_name.strip_suffix(".pub") else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let private_path = directory.join(name);
        let private_exists = valid_private_path(name)
            && fs::symlink_metadata(&private_path)
                .is_ok_and(|metadata| !metadata.file_type().is_symlink() && metadata.is_file());
        let Some(fingerprint) = public_fingerprint(&public_path) else {
            continue;
        };
        keys.push(SshKeyEntry {
            name: name.to_string(),
            path_display: if private_exists {
                format!("~/.ssh/{name}")
            } else {
                format!("~/.ssh/{public_name}")
            },
            fingerprint,
            public_name: public_name.to_string(),
            private_name: private_exists.then(|| name.to_string()),
        });
    }
    keys.sort_by(|left, right| left.name.cmp(&right.name));
    return keys;
}

pub(crate) fn read_public_key(name: &str) -> Result<String, SshError> {
    validate_public_name(name)?;
    let directory = ssh_dir()?;
    return read_public_key_at(&directory, name);
}

fn read_public_key_at(directory: &Path, name: &str) -> Result<String, SshError> {
    validate_public_name(name)?;
    let directory = checked_ssh_directory(directory, false)?;
    let path = directory.join(name);
    let metadata = fs::symlink_metadata(&path).map_err(|_| SshError::MissingFile)?;
    if metadata.file_type().is_symlink() {
        return Err(SshError::Symlink);
    }
    if !metadata.is_file() {
        return Err(SshError::NotRegularFile);
    }
    let text = read_public_key_file(&path)?;
    if public_fingerprint(&path).is_none() {
        return Err(SshError::InvalidPublicKey);
    }
    return Ok(text);
}

#[cfg(all(test, unix))]
#[path = "ssh/tests.rs"]
mod tests;
