//! 测试共用的小工具：一个带首个提交的临时 Git 仓库（`TempRepo`）。

use std::path::PathBuf;

pub(crate) struct TempRepo {
    pub(crate) path: PathBuf,
}

impl TempRepo {
    pub(crate) fn new(name: &str) -> TempRepo {
        let unique = format!(
            "nsh-app-model-test-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        );
        let path = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&path).expect("temp dir");
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&path)
                .args(args)
                .status()
                .expect("git");
            assert!(status.success(), "git {:?}", args);
        };
        run(&["init", "-q", "-b", "main"]);
        run(&["config", "user.name", "Test"]);
        run(&["config", "user.email", "test@example.com"]);
        std::fs::write(path.join("a.txt"), "one\n").expect("write");
        run(&["add", "-A"]);
        run(&["commit", "-q", "-m", "first"]);
        return TempRepo { path };
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}
