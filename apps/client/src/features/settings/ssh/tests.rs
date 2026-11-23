use super::*;
use std::time::{SystemTime, UNIX_EPOCH};

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("gitui-ssh-test-{}-{nonce:x}", std::process::id()));
        fs::create_dir(&path).expect("create test directory");
        return Self(path);
    }

    fn path(&self) -> &Path {
        return self.0.as_path();
    }

    fn ssh_dir(&self) -> PathBuf {
        let path = self.0.join("ssh");
        fs::create_dir(&path).expect("create ssh test directory");
        return path;
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn create_plain_key(directory: &Path, name: &str) {
    let path = directory.join(name);
    let output = storage::ssh_keygen_command()
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "test"])
        .arg("-f")
        .arg(path)
        .output()
        .expect("run ssh-keygen fixture");
    assert!(
        output.status.success(),
        "ssh-keygen failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn scans_pairs_and_standalone_public_keys_but_skips_symlinks_and_invalid_files() {
    let scratch = Scratch::new();
    let directory = scratch.ssh_dir();
    create_plain_key(&directory, "id_ed25519");
    fs::copy(
        directory.join("id_ed25519.pub"),
        directory.join("deployment.pub"),
    )
    .unwrap();
    fs::write(directory.join("invalid.pub"), "not a public key\n").unwrap();
    fs::write(directory.join("config"), "Host *\n").unwrap();
    std::os::unix::fs::symlink(
        directory.join("id_ed25519.pub"),
        directory.join("linked.pub"),
    )
    .unwrap();
    assert_eq!(
        read_public_key_at(&directory, "linked.pub"),
        Err(SshError::Symlink)
    );

    let entries = scan_keys(&directory);
    assert_eq!(entries.len(), 2);
    let pair = entries
        .iter()
        .find(|entry| entry.name == "id_ed25519")
        .unwrap();
    assert_eq!(pair.path_display, "~/.ssh/id_ed25519");
    assert!(pair.fingerprint.starts_with("SHA256:"));
    let standalone = entries
        .iter()
        .find(|entry| entry.name == "deployment")
        .unwrap();
    assert_eq!(standalone.path_display, "~/.ssh/deployment.pub");
    assert_eq!(standalone.public_name, "deployment.pub");
}

#[test]
fn public_key_read_rejects_symlinked_ssh_directories() {
    let scratch = Scratch::new();
    let directory = scratch.ssh_dir();
    create_plain_key(&directory, "id_ed25519");
    let link_directory = scratch.path().join("ssh-link");
    std::os::unix::fs::symlink(&directory, &link_directory).unwrap();
    assert_eq!(
        checked_ssh_directory(&link_directory),
        Err(SshError::Symlink)
    );
}
