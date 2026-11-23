use super::SshError;
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_PUBLIC_KEY_BYTES: u64 = 64 * 1024;

pub(super) fn ensure_targets_free(directory: &Path, name: &str) -> Result<(), SshError> {
    let private_path = directory.join(name);
    let public_path = directory.join(format!("{name}.pub"));
    if exists_no_follow(&private_path)? || exists_no_follow(&public_path)? {
        return Err(SshError::NameAlreadyExists);
    }
    return Ok(());
}

fn exists_no_follow(path: &Path) -> Result<bool, SshError> {
    return match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(SshError::FileOperation),
    };
}

pub(super) fn checked_ssh_directory(directory: &Path, create: bool) -> Result<PathBuf, SshError> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() {
                return Err(SshError::Symlink);
            }
            if !metadata.is_dir() {
                return Err(SshError::DirectoryUnavailable);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound && create => {
            fs::create_dir(directory).map_err(|_| SshError::DirectoryUnavailable)?;
            if let Err(error) = set_mode(directory, 0o700) {
                let _ = fs::remove_dir(directory);
                return Err(error);
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(SshError::DirectoryUnavailable);
        }
        Err(_) => return Err(SshError::DirectoryUnavailable),
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

pub(super) fn validate_key_name(name: &str) -> Result<(), SshError> {
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
        return Err(SshError::InvalidName);
    }
    return Ok(());
}

pub(super) fn valid_private_path(name: &str) -> bool {
    return validate_key_name(name).is_ok();
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

pub(super) fn check_regular_file(path: &Path) -> Result<(), SshError> {
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

pub(super) fn copy_private_file(
    source: &Path,
    destination: &Path,
    target: &mut File,
) -> Result<(), SshError> {
    check_regular_file(source)?;
    let mut source = File::open(source).map_err(|_| SshError::FileOperation)?;
    std::io::copy(&mut source, target).map_err(|_| SshError::FileOperation)?;
    target.sync_all().map_err(|_| SshError::FileOperation)?;
    set_mode(destination, 0o600)
}

pub(super) fn create_private_temp_file(
    directory: &Path,
    kind: &str,
) -> Result<(PathBuf, File), SshError> {
    for attempt in 0..64_u64 {
        let path = directory.join(format!(".gitui-{kind}-{}-{attempt:x}", unique_token()));
        match create_file_with_mode(&path, 0o600) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(SshError::FileOperation),
        }
    }
    return Err(SshError::FileOperation);
}

pub(super) fn create_public_temp_file(
    directory: &Path,
    kind: &str,
) -> Result<(PathBuf, File), SshError> {
    let (path, file) = create_private_temp_file(directory, kind)?;
    if let Err(error) = set_mode(&path, 0o644) {
        drop(file);
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    return Ok((path, file));
}

pub(super) fn create_private_temp_directory(
    directory: &Path,
    kind: &str,
) -> Result<PathBuf, SshError> {
    for attempt in 0..64_u64 {
        let path = directory.join(format!(".gitui-{kind}-{}-{attempt:x}", unique_token()));
        match fs::create_dir(&path) {
            Ok(()) => {
                if let Err(error) = set_mode(&path, 0o700) {
                    let _ = fs::remove_dir(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(SshError::FileOperation),
        }
    }
    return Err(SshError::FileOperation);
}

fn create_file_with_mode(path: &Path, mode: u32) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(mode);
    }
    let file = options.open(path)?;
    #[cfg(unix)]
    if let Err(_) = set_mode(path, mode) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(std::io::Error::other("could not set file mode"));
    }
    return Ok(file);
}

fn set_mode(path: &Path, mode: u32) -> Result<(), SshError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| SshError::FileOperation)?;
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        return Err(SshError::FileOperation);
    }
    return Ok(());
}

pub(super) fn classify_target_error(path: &Path) -> SshError {
    return if matches!(fs::symlink_metadata(path), Ok(_)) {
        SshError::NameAlreadyExists
    } else {
        SshError::FileOperation
    };
}

pub(super) fn unique_token() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    return format!("{}-{timestamp:x}", std::process::id());
}
