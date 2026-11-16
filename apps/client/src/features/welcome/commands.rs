//! 欢迎页的用例：选择目录、克隆、取消克隆、初始化仓库与切换入口分段。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 打开仓库这类跨功能动作走 `SessionHandle` 的方法。

use super::state::{CloneOptions, InitFiles, RepoForm, WelcomeState};
use crate::features::SessionHandle;
use crate::i18n;
use domain::GitError;
use nsh_core::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(crate) struct WelcomeCommands {
    pub(crate) state: WelcomeState,
    pub(crate) session: SessionHandle,
    /// 错误文案按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 克隆的取消标志：后台轮询线程读它决定是否提前结束。
    pub(crate) cancel: Arc<AtomicBool>,
}

impl WelcomeCommands {
    /// 让用户选一个目录并打开（原 `choose_folder`）。
    pub(crate) fn choose_folder(&self) {
        let session = self.session.clone();
        let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
        spawn(async move {
            if let Ok(Some(paths)) = answer.await {
                if let Some(path) = paths.first() {
                    session.open_path(path.display().to_string());
                }
            }
        });
    }

    /// 开始克隆（原 `clone_repository`）：逐 100ms 轮询子进程，可取消。
    pub(crate) fn clone_repository(&self) {
        let clone_url = self.state.clone_url;
        let clone_destination = self.state.clone_destination;
        let clone_options = self.state.clone_options;
        let clone_error = self.state.clone_error;
        let cloning = self.state.cloning;
        let clone_progress = self.state.clone_progress;
        let session = self.session.clone();
        let language = self.language;
        let cancel = self.cancel.clone();
        let url = clone_url.get_untracked();
        let destination = clone_destination.get_untracked();
        let chosen: CloneOptions = clone_options.get_untracked();
        let options = git::CloneOptions {
            branch: Some(chosen.branch.clone()).filter(|b| !b.trim().is_empty()),
            depth: chosen.depth,
            recurse_submodules: chosen.recurse_submodules,
        };
        if url.trim().is_empty() {
            clone_error.set(Some(i18n::t(
                language.get_untracked().as_str(),
                "welcome.missingUrl",
            )));
            return;
        }
        if destination.trim().is_empty() {
            clone_error.set(Some(i18n::t(
                language.get_untracked().as_str(),
                "welcome.missingDestination",
            )));
            return;
        }
        // 「克隆后打开」关闭时只记入最近仓库，不切走当前工作区。
        let open_after = chosen.open_after_clone;
        cloning.set(true);
        clone_progress.set(String::new());
        cancel.store(false, Ordering::Relaxed);
        clone_error.set(None);
        let recent = session.recent;
        let store_dir = session.store_dir.clone();
        spawn(async move {
            // 启动后按 100ms 轮询：界面一直可响应，取消也能立刻生效。
            let mut run = match git::start_clone(url.trim(), destination.trim(), &options) {
                Ok(run) => run,
                Err(error) => {
                    cloning.set(false);
                    clone_error.set(Some(localized_error(
                        language.get_untracked().as_str(),
                        &error,
                    )));
                    return;
                }
            };
            let result = loop {
                if cancel.load(Ordering::Relaxed) {
                    let _ = run.cancel();
                    let lang = language.get_untracked();
                    break Err(i18n::t(lang.as_str(), "welcome.cloneCancelled"));
                }
                if let Some(outcome) = run.try_finish() {
                    let lang = language.get_untracked();
                    break outcome.map_err(|error| localized_error(lang.as_str(), &error));
                }
                let progress = run.progress();
                if !progress.is_empty() {
                    clone_progress.set(progress);
                }
                timer(std::time::Duration::from_millis(100)).await;
            };
            cloning.set(false);
            clone_progress.set(String::new());
            match result {
                Ok(()) => {
                    if open_after {
                        session.open_path(destination.trim().to_string());
                    } else {
                        let path = destination.trim().to_string();
                        let list = recent.get_untracked();
                        // 复用既有的「记住最近仓库」入口，顺序与去重规则一致。
                        store::remember_recent(&store_dir, &list, path.as_str());
                        recent.set(store::load_recent(&store_dir));
                    }
                }
                Err(detail) => clone_error.set(Some(detail)),
            }
        });
    }

    /// 取消正在进行的克隆（原 `cancel_clone`）：后台轮询看到标志后结束子进程。
    pub(crate) fn cancel_clone(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// 初始化仓库并打开（原 `init_repository`）。
    pub(crate) fn init_repository(&self) {
        let init_path = self.state.init_path;
        let init_error = self.state.init_error;
        let init_files = self.state.init_files;
        let session = self.session.clone();
        let language = self.language;
        let path = init_path.get_untracked();
        if path.trim().is_empty() {
            init_error.set(Some(i18n::t(
                language.get_untracked().as_str(),
                "welcome.missingDirectory",
            )));
            return;
        }
        let files: InitFiles = init_files.get_untracked();
        let options = git::InitOptions {
            gitignore: files.gitignore,
            readme: files.readme,
        };
        match git::init_with(path.trim(), &options) {
            Ok(()) => session.open_path(path.trim().to_string()),
            Err(error) => init_error.set(Some(localized_error(
                language.get_untracked().as_str(),
                &error,
            ))),
        }
    }

    /// 切换 Open / Clone / Init 分段（原 `set_repo_form`）。
    pub(crate) fn set_repo_form(&self, form: RepoForm) {
        self.state.form.set(form);
        // 错误跟着表单走：切换后旧表单的失败不能出现在新表单里。
        self.state.clone_error.set(None);
        self.state.init_error.set(None);
    }
}

/// 把 Git 错误映射成当前语言的表单文案。
///
/// `GitError::detail()` 只有中文，不能直接给英文界面用；这里按错误类别取
/// i18n 文案，命令输出留在错误对象里供后续诊断。
pub(crate) fn localized_error(language: &str, error: &GitError) -> String {
    return match error {
        GitError::GitMissing => i18n::t(language, "welcome.error.gitMissing"),
        GitError::NotARepository { path } => i18n::with_arguments(
            language,
            "welcome.error.notRepo",
            &[("path", path.clone())],
        ),
        GitError::Unreadable { path, .. } => i18n::with_arguments(
            language,
            "welcome.error.unreadable",
            &[("path", path.clone())],
        ),
        GitError::CommandFailed { .. } => {
            i18n::t(language, "welcome.error.commandFailed")
        }
        GitError::Spawn { program, .. } => i18n::with_arguments(
            language,
            "welcome.error.spawn",
            &[("program", program.clone())],
        ),
        GitError::HookFailed { .. } => i18n::t(language, "welcome.error.hookFailed"),
        GitError::Parse { .. } => i18n::t(language, "welcome.error.parse"),
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_errors_use_localized_text() {
        assert_eq!(
            localized_error("en", &GitError::GitMissing),
            i18n::t("en", "welcome.error.gitMissing")
        );
        let not_repo = GitError::NotARepository {
            path: "/tmp/not-a-repo".to_string(),
        };
        let message = localized_error("en", &not_repo);
        assert!(message.contains("/tmp/not-a-repo"));
        assert!(!message.contains("不是 Git 仓库"));
    }
}
