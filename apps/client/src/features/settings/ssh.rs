//! `~/.ssh` 的只读读取：文件名、路径与公钥指纹。
//!
//! 只读目录项和 `.pub` 的指纹，不读取私钥内容、不生成、不删除、不上传。

/// 返回 `(文件名, 显示路径, 公钥指纹)`；没有对应 `.pub` 时指纹为空。
pub(crate) fn read_keys() -> Vec<(String, String, String)> {
    let Some(home) = std::env::var_os("HOME") else {
        return Vec::new();
    };
    let dir = std::path::PathBuf::from(home).join(".ssh");
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut keys: Vec<(String, String, String)> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.')
            || name.ends_with(".pub")
            || matches!(name, "config" | "known_hosts" | "known_hosts.old" | "authorized_keys")
        {
            continue;
        }
        let fingerprint = public_fingerprint(path.as_os_str().to_string_lossy().as_ref());
        keys.push((
            name.to_string(),
            format!("~/.ssh/{name}"),
            fingerprint,
        ));
    }
    keys.sort_by(|left, right| left.0.cmp(&right.0));
    return keys;
}

fn public_fingerprint(private_path: &str) -> String {
    let pub_path = format!("{private_path}.pub");
    let Ok(output) = std::process::Command::new("ssh-keygen")
        .args(["-lf", pub_path.as_str()])
        .output()
    else {
        return String::new();
    };
    if !output.status.success() {
        return String::new();
    }
    let text = String::from_utf8_lossy(&output.stdout);
    return text
        .split_whitespace()
        .nth(1)
        .unwrap_or_default()
        .to_string();
}
