use super::*;
use std::os::unix::fs::PermissionsExt;
use std::sync::mpsc::Receiver;

struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("gitui-ssh-test-{}", unique_token()));
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

fn askpass_client() -> (
    crate::askpass::AskPassClient,
    Receiver<crate::askpass::UiEvent>,
) {
    let (_ui, client, receiver) = crate::askpass::channel();
    return (client, receiver);
}

fn create_plain_key(directory: &Path, name: &str) {
    let path = directory.join(name);
    let output = ssh_keygen_command()
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
    assert_eq!(pair.private_name.as_deref(), Some("id_ed25519"));
    assert!(pair.fingerprint.starts_with("SHA256:"));
    let standalone = entries
        .iter()
        .find(|entry| entry.name == "deployment")
        .unwrap();
    assert_eq!(standalone.private_name, None);
    assert_eq!(standalone.public_name, "deployment.pub");
}

#[test]
fn imports_private_key_without_overwriting_and_sets_secure_file_modes() {
    let runtime = nsh_core::element::reactive::Runtime::default();
    let _entered = runtime.enter();
    let scratch = Scratch::new();
    let source_dir = scratch.path().join("source");
    fs::create_dir(&source_dir).unwrap();
    let directory = scratch.ssh_dir();
    create_plain_key(&source_dir, "work_key");
    let source_private = source_dir.join("work_key");
    let source_public = source_dir.join("work_key.pub");
    let (askpass, _receiver) = askpass_client();

    let entry = import_key(&source_private, &directory, askpass.clone()).unwrap();
    assert_eq!(entry.name, "work_key");
    assert_eq!(
        fs::read(directory.join("work_key")).unwrap(),
        fs::read(&source_private).unwrap()
    );
    assert_eq!(
        fs::read(directory.join("work_key.pub")).unwrap(),
        fs::read(&source_public).unwrap()
    );
    assert_eq!(
        fs::metadata(directory.join("work_key"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(directory.join("work_key.pub"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
    assert_eq!(
        import_key(&source_private, &directory, askpass).unwrap_err(),
        SshError::NameAlreadyExists
    );
}

#[test]
fn derives_missing_public_key_and_rejects_a_mismatched_adjacent_key() {
    let runtime = nsh_core::element::reactive::Runtime::default();
    let _entered = runtime.enter();
    let scratch = Scratch::new();
    let source_dir = scratch.path().join("source");
    fs::create_dir(&source_dir).unwrap();
    let directory = scratch.ssh_dir();
    let (askpass, _receiver) = askpass_client();

    create_plain_key(&source_dir, "missing_public");
    fs::remove_file(source_dir.join("missing_public.pub")).unwrap();
    let derived = import_key(
        &source_dir.join("missing_public"),
        &directory,
        askpass.clone(),
    )
    .expect("derive adjacent public key");
    assert!(derived.fingerprint.starts_with("SHA256:"));
    assert!(directory.join("missing_public.pub").is_file());

    create_plain_key(&source_dir, "mismatch");
    create_plain_key(&source_dir, "other");
    fs::copy(
        source_dir.join("other.pub"),
        source_dir.join("mismatch.pub"),
    )
    .unwrap();
    assert_eq!(
        import_key(&source_dir.join("mismatch"), &directory, askpass).unwrap_err(),
        SshError::NotMatchingKey
    );
    assert!(!directory.join("mismatch").exists());
    assert!(!directory.join("mismatch.pub").exists());
}

#[test]
fn remove_requires_the_listed_fingerprint_and_deletes_the_pair() {
    let scratch = Scratch::new();
    let directory = scratch.ssh_dir();
    create_plain_key(&directory, "remove_me");
    let entry = scan_keys(&directory).pop().unwrap();
    assert_eq!(
        remove_key(&directory, "remove_me", "SHA256:stale"),
        Err(SshError::KeyChanged)
    );
    assert!(directory.join("remove_me").exists());

    remove_key(&directory, "remove_me", entry.fingerprint.as_str()).unwrap();
    assert!(!directory.join("remove_me").exists());
    assert!(!directory.join("remove_me.pub").exists());
}

#[test]
fn rejects_symlink_sources_directories_and_key_names() {
    let runtime = nsh_core::element::reactive::Runtime::default();
    let _entered = runtime.enter();
    let scratch = Scratch::new();
    let directory = scratch.ssh_dir();
    let source_dir = scratch.path().join("source");
    fs::create_dir(&source_dir).unwrap();
    create_plain_key(&source_dir, "linked_key");
    std::os::unix::fs::symlink(
        source_dir.join("linked_key"),
        scratch.path().join("private-link"),
    )
    .unwrap();
    let (askpass, _receiver) = askpass_client();
    assert_eq!(
        import_key(&scratch.path().join("private-link"), &directory, askpass).unwrap_err(),
        SshError::Symlink
    );
    std::os::unix::fs::symlink(source_dir.join("linked_key"), directory.join("linked_key"))
        .unwrap();
    fs::copy(
        source_dir.join("linked_key.pub"),
        directory.join("linked_key.pub"),
    )
    .unwrap();
    let public_fingerprint = public_fingerprint(&directory.join("linked_key.pub")).unwrap();
    assert_eq!(
        remove_key(&directory, "linked_key", public_fingerprint.as_str()),
        Err(SshError::Symlink)
    );
    assert_eq!(validate_key_name("../outside"), Err(SshError::InvalidName));
    assert_eq!(
        validate_key_name("id_ed25519.pub"),
        Err(SshError::InvalidName)
    );

    let link_directory = scratch.path().join("ssh-link");
    std::os::unix::fs::symlink(&directory, &link_directory).unwrap();
    assert_eq!(
        checked_ssh_directory(&link_directory, false),
        Err(SshError::Symlink)
    );
}
