use super::storage::*;
use super::{SshError, SshKeyEntry};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};

pub(crate) fn import_key(
    source: &Path,
    directory: &Path,
    askpass: crate::askpass::AskPassClient,
) -> Result<SshKeyEntry, SshError> {
    let directory = checked_ssh_directory(directory, true)?;
    let source_metadata = fs::symlink_metadata(source).map_err(|_| SshError::MissingFile)?;
    if source_metadata.file_type().is_symlink() {
        return Err(SshError::Symlink);
    }
    if !source_metadata.is_file() {
        return Err(SshError::NotRegularFile);
    }
    let source_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or(SshError::InvalidName)?;
    validate_key_name(source_name)?;
    if source_name.ends_with(".pub") {
        return Err(SshError::NotRegularFile);
    }
    ensure_targets_free(&directory, source_name)?;

    let derived_public = derive_public_key(source, askpass)?;
    let source_public_path = source.with_file_name(format!("{source_name}.pub"));
    let public_text = match fs::symlink_metadata(&source_public_path) {
        Ok(metadata) if metadata.file_type().is_symlink() => return Err(SshError::Symlink),
        Ok(metadata) if !metadata.is_file() => return Err(SshError::NotRegularFile),
        Ok(_) => {
            let text = read_public_key_file(&source_public_path)?;
            if parse_public_key(text.as_str())? != parse_public_key(derived_public.as_str())? {
                return Err(SshError::NotMatchingKey);
            }
            text
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => derived_public,
        Err(_) => return Err(SshError::FileOperation),
    };

    install_pair(&directory, source_name, source, public_text.as_bytes())
}

/// Generate an Ed25519 key. `ssh-keygen` asks for an optional passphrase through AskPass.
pub(crate) fn generate_key(
    name: &str,
    comment: &str,
    directory: &Path,
    askpass: crate::askpass::AskPassClient,
) -> Result<SshKeyEntry, SshError> {
    validate_key_name(name)?;
    let directory = checked_ssh_directory(directory, true)?;
    ensure_targets_free(&directory, name)?;
    let temporary_directory = create_private_temp_directory(&directory, "generate")?;
    let temporary_private = temporary_directory.join(name);

    let mut command = ssh_keygen_command();
    command.arg("-q").arg("-t").arg("ed25519").arg("-f");
    command.arg(&temporary_private);
    let comment = comment.trim();
    if !comment.is_empty() {
        if comment.chars().any(char::is_control) {
            let _ = fs::remove_dir_all(&temporary_directory);
            return Err(SshError::InvalidComment);
        }
        command.arg("-C").arg(comment);
    }
    let output = run_with_askpass(command, askpass);
    let result = match output {
        Ok(_) => {
            let public_path = temporary_private.with_file_name(format!("{name}.pub"));
            match read_public_key_file(&public_path).and_then(|public_text| {
                let (key_type, _) = parse_public_key(public_text.as_str())?;
                if key_type != "ssh-ed25519" {
                    return Err(SshError::InvalidPublicKey);
                }
                return install_pair(&directory, name, &temporary_private, public_text.as_bytes());
            }) {
                Ok(entry) => Ok(entry),
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    };
    let _ = fs::remove_dir_all(&temporary_directory);
    return result;
}

/// Remove a managed pair after the caller has obtained explicit confirmation.
pub(crate) fn remove_key(
    directory: &Path,
    name: &str,
    expected_fingerprint: &str,
) -> Result<(), SshError> {
    validate_key_name(name)?;
    let directory = checked_ssh_directory(directory, false)?;
    let private_path = directory.join(name);
    let public_path = directory.join(format!("{name}.pub"));
    check_regular_file(&private_path)?;
    check_regular_file(&public_path)?;
    if public_fingerprint(&public_path).as_deref() != Some(expected_fingerprint) {
        return Err(SshError::KeyChanged);
    }

    let quarantine = create_private_temp_directory(&directory, "remove")?;
    let quarantined_private = quarantine.join(name);
    let quarantined_public = quarantine.join(format!("{name}.pub"));
    fs::rename(&private_path, &quarantined_private).map_err(|_| SshError::FileOperation)?;
    if fs::rename(&public_path, &quarantined_public).is_err() {
        let _ = fs::rename(&quarantined_private, &private_path);
        let _ = fs::remove_dir(&quarantine);
        return Err(SshError::FileOperation);
    }
    if fs::remove_file(&quarantined_private).is_err() {
        let _ = fs::rename(&quarantined_private, &private_path);
        let _ = fs::rename(&quarantined_public, &public_path);
        let _ = fs::remove_dir(&quarantine);
        return Err(SshError::FileOperation);
    }
    if fs::remove_file(&quarantined_public).is_err() {
        let _ = fs::rename(&quarantined_public, &public_path);
        let _ = fs::remove_dir(&quarantine);
        return Err(SshError::FileOperation);
    }
    fs::remove_dir(&quarantine).map_err(|_| SshError::FileOperation)?;
    return Ok(());
}

fn derive_public_key(
    private_path: &Path,
    askpass: crate::askpass::AskPassClient,
) -> Result<String, SshError> {
    let mut command = ssh_keygen_command();
    command.arg("-y").arg("-f").arg(private_path);
    let output = run_with_askpass(command, askpass)?;
    let text = String::from_utf8(output.stdout).map_err(|_| SshError::InvalidPublicKey)?;
    parse_public_key(text.as_str())?;
    return Ok(text);
}

fn run_with_askpass(
    mut command: Command,
    askpass: crate::askpass::AskPassClient,
) -> Result<Output, SshError> {
    let session = crate::askpass::AskPassSession::new(git::GitExecutable::system(), askpass)
        .map_err(|_| SshError::FileOperation)?;
    session.apply_env(&mut command);
    let output = command.output().map_err(|_| SshError::KeygenFailed)?;
    let cancelled = session.was_cancelled();
    drop(session);
    if cancelled {
        return Err(SshError::Cancelled);
    }
    if !output.status.success() {
        return Err(SshError::KeygenFailed);
    }
    return Ok(output);
}

fn install_pair(
    directory: &Path,
    name: &str,
    private_source: &Path,
    public_text: &[u8],
) -> Result<SshKeyEntry, SshError> {
    validate_key_name(name)?;
    ensure_targets_free(directory, name)?;
    let (temporary_private, mut private_file) = create_private_temp_file(directory, "private")?;
    if let Err(error) = copy_private_file(private_source, &temporary_private, &mut private_file) {
        let _ = fs::remove_file(&temporary_private);
        return Err(error);
    }
    drop(private_file);
    let (temporary_public, mut public_file) = create_public_temp_file(directory, "public")?;
    if public_file.write_all(public_text).is_err() || public_file.sync_all().is_err() {
        let _ = fs::remove_file(&temporary_private);
        let _ = fs::remove_file(&temporary_public);
        return Err(SshError::FileOperation);
    }
    drop(public_file);
    let Some(fingerprint) = public_fingerprint(&temporary_public) else {
        let _ = fs::remove_file(&temporary_private);
        let _ = fs::remove_file(&temporary_public);
        return Err(SshError::InvalidPublicKey);
    };

    let private_path = directory.join(name);
    let public_name = format!("{name}.pub");
    let public_path = directory.join(&public_name);
    if fs::hard_link(&temporary_private, &private_path).is_err() {
        let _ = fs::remove_file(&temporary_private);
        let _ = fs::remove_file(&temporary_public);
        return Err(classify_target_error(&private_path));
    }
    if fs::hard_link(&temporary_public, &public_path).is_err() {
        let _ = fs::remove_file(&private_path);
        let _ = fs::remove_file(&temporary_private);
        let _ = fs::remove_file(&temporary_public);
        return Err(classify_target_error(&public_path));
    }
    let _ = fs::remove_file(&temporary_private);
    let _ = fs::remove_file(&temporary_public);
    return Ok(SshKeyEntry {
        name: name.to_string(),
        path_display: format!("~/.ssh/{name}"),
        fingerprint,
        public_name: public_name.to_string(),
        private_name: Some(name.to_string()),
    });
}
