use super::SshError;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

const MAX_PUBLIC_KEY_BYTES: u64 = 64 * 1024;

pub(super) fn checked_ssh_directory(directory: &Path) -> Result<PathBuf, SshError> {
    let metadata = fs::symlink_metadata(directory).map_err(|_| SshError::DirectoryUnavailable)?;
    if metadata.file_type().is_symlink() {
        return Err(SshError::Symlink);
    }
    if !metadata.is_dir() {
        return Err(SshError::DirectoryUnavailable);
    }
    let canonical = directory
        .canonicalize()
        .map_err(|_| SshError::DirectoryUnavailable)?;
    let parent = directory
        .parent()
        .ok_or(SshError::DirectoryUnavailable)?
        .canonicalize()
        .map_err(|_| SshError::DirectoryUnavailable)?;
    if canonical.parent() != Some(parent.as_path()) {
        return Err(SshError::Symlink);
    }
    return Ok(canonical);
}

pub(super) fn valid_private_path(name: &str) -> bool {
    if name.is_empty()
        || name.len() > 128
        || name.starts_with('.')
        || name.ends_with(".pub")
        || name.chars().any(char::is_control)
        || Path::new(name).file_name().and_then(|part| part.to_str()) != Some(name)
        || matches!(
            name,
            "config" | "known_hosts" | "known_hosts.old" | "authorized_keys"
        )
    {
        return false;
    }
    return true;
}

pub(super) fn validate_public_name(name: &str) -> Result<(), SshError> {
    let Some(stem) = name.strip_suffix(".pub") else {
        return Err(SshError::InvalidName);
    };
    if stem.is_empty()
        || name.len() > 133
        || name.chars().any(char::is_control)
        || Path::new(name).file_name().and_then(|part| part.to_str()) != Some(name)
    {
        return Err(SshError::InvalidName);
    }
    return Ok(());
}

fn check_regular_file(path: &Path) -> Result<(), SshError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| SshError::MissingFile)?;
    if metadata.file_type().is_symlink() {
        return Err(SshError::Symlink);
    }
    if !metadata.is_file() {
        return Err(SshError::NotRegularFile);
    }
    return Ok(());
}

pub(super) fn read_public_key_file(path: &Path) -> Result<String, SshError> {
    check_regular_file(path)?;
    let metadata = fs::metadata(path).map_err(|_| SshError::FileOperation)?;
    if metadata.len() > MAX_PUBLIC_KEY_BYTES {
        return Err(SshError::InvalidPublicKey);
    }
    let mut file = File::open(path).map_err(|_| SshError::FileOperation)?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|_| SshError::InvalidPublicKey)?;
    parse_public_key(text.as_str())?;
    return Ok(text);
}

pub(super) fn parse_public_key(text: &str) -> Result<(String, String), SshError> {
    let mut nonempty = text.lines().filter(|line| !line.trim().is_empty());
    let Some(line) = nonempty.next() else {
        return Err(SshError::InvalidPublicKey);
    };
    if nonempty.next().is_some() {
        return Err(SshError::InvalidPublicKey);
    }
    let mut fields = line.split_whitespace();
    let Some(key_type) = fields.next() else {
        return Err(SshError::InvalidPublicKey);
    };
    let Some(key_data) = fields.next() else {
        return Err(SshError::InvalidPublicKey);
    };
    if !(key_type.starts_with("ssh-")
        || key_type.starts_with("ecdsa-")
        || key_type.starts_with("sk-"))
    {
        return Err(SshError::InvalidPublicKey);
    }
    return Ok((key_type.to_string(), key_data.to_string()));
}

pub(super) fn public_fingerprint(path: &Path) -> Option<String> {
    let output = ssh_keygen_command().args(["-lf"]).arg(path).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    return text
        .split_whitespace()
        .nth(1)
        .filter(|value| value.starts_with("SHA256:"))
        .map(str::to_string);
}

pub(super) fn ssh_keygen_command() -> Command {
    let system_path = Path::new("/usr/bin/ssh-keygen");
    if system_path.is_file() {
        return Command::new(system_path);
    }
    return Command::new("ssh-keygen");
}
