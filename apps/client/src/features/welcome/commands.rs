//! 欢迎页的用例：选择目录、克隆、取消克隆、初始化仓库与切换入口分段。
//!
//! 方法体由原 `Actions` 里的闭包原样搬来，只把捕获的局部变量换成 `self.*`；
//! 打开仓库这类跨功能动作走 `SessionHandle` 的方法。

use super::state::{CloneOptions, InitFiles, RepoForm, WelcomeState};
use crate::features::SessionHandle;
use crate::features::settings::commands::SettingsCommands;
use crate::features::settings::state::Section;
use crate::i18n;
use domain::GitError;
use nsh_core::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(crate) struct WelcomeCommands {
    pub(crate) state: WelcomeState,
    pub(crate) session: SessionHandle,
    /// 设置页入口：左栏 Settings 与克隆的 Credentials 都用它。
    pub(crate) settings: SettingsCommands,
    /// 错误文案按当前语言取。
    pub(crate) language: Signal<SharedString>,
    /// 克隆的取消标志：后台轮询线程读它决定是否提前结束。
    pub(crate) cancel: Arc<AtomicBool>,
    pub(crate) askpass: crate::askpass::AskPassClient,
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

    /// 选一个目录并把路径写进调用方给的信号；三个表单的 Choose… 用它。
    fn pick_directory(set: Arc<dyn Fn(String) + 'static>) {
        let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
        spawn(async move {
            if let Ok(Some(paths)) = answer.await {
                if let Some(path) = paths.first() {
                    set(path.display().to_string());
                }
            }
        });
    }

    /// Open 表单的主操作：有值就校验并打开，没值先选目录，选完再校验打开。
    pub(crate) fn open_or_choose(&self) {
        if !self.state.open_path.get_untracked().trim().is_empty() {
            self.open_repository();
            return;
        }
        let open = self.clone();
        let answer = pick_paths(nsh_core::file_dialog::OpenDialog::folders());
        spawn(async move {
            let Ok(Some(paths)) = answer.await else {
                return;
            };
            let Some(chosen) = paths.first() else {
                return;
            };
            open.state.open_path.set(chosen.display().to_string());
            open.open_repository();
        });
    }

    /// 空列表里的扫描入口：扫描尚未接入，给出真实状态说明。
    pub(crate) fn scan_repositories(&self) {
        let language = self.language.get_untracked();
        crate::interactions::alert(
            i18n::t(language.as_str(), "welcome.scan"),
            i18n::t(language.as_str(), "welcome.scanUnavailable"),
            i18n::t(language.as_str(), "common.close"),
        );
    }

    /// Clone 表单的 Choose…：只填目标目录，不打开仓库。
    pub(crate) fn choose_clone_destination(&self) {
        let destination = self.state.clone_destination;
        Self::pick_directory(Arc::new(move |path| destination.set(path)));
    }

    /// Init 表单的 Choose…：填目录；仓库名仍为空时用目录名补上。
    pub(crate) fn choose_init_folder(&self) {
        let init_path = self.state.init_path;
        let init_name = self.state.init_name;
        Self::pick_directory(Arc::new(move |path| {
            if init_name.get_untracked().trim().is_empty() {
                let name = path
                    .trim_end_matches('/')
                    .rsplit('/')
                    .next()
                    .unwrap_or_default();
                init_name.set(name.to_string());
            }
            init_path.set(path);
        }));
    }

    /// 校验并打开 Open 表单的路径；失败留在欢迎页显示错误。
    pub(crate) fn open_repository(&self) {
        let open_path = self.state.open_path;
        let open_error = self.state.open_error;
        let session = self.session.clone();
        let language = self.language;
        let path = open_path.get_untracked();
        if path.trim().is_empty() {
            open_error.set(Some(i18n::t(
                language.get_untracked().as_str(),
                "welcome.missingPath",
            )));
            return;
        }
        match session.open_repo(path.trim()) {
            Ok(_) => {
                open_error.set(None);
                session.open_path(path.trim().to_string());
            }
            Err(error) => {
                open_error.set(Some(localized_error(
                    language.get_untracked().as_str(),
                    &error,
                )));
            }
        }
    }

    /// 从最近仓库列表打开，失败时留在欢迎页并把错误放回 Open 表单。
    pub(crate) fn open_recent(&self, path: String) {
        self.state.form.set(RepoForm::Open);
        self.state.open_path.set(path);
        self.state.open_error.set(None);
        self.open_repository();
    }

    /// 开始克隆（原 `clone_repository`）：逐 100ms 轮询子进程，可取消。
    pub(crate) fn clone_repository(&self) {
        // 按钮在克隆中会被禁用；这里再拦一次，避免键盘重复触发。
        if self.state.cloning.get_untracked() {
            return;
        }
        let clone_url = self.state.clone_url;
        let clone_destination = self.state.clone_destination;
        let clone_options = self.state.clone_options;
        let clone_error = self.state.clone_error;
        let cloning = self.state.cloning;
        let clone_progress = self.state.clone_progress;
        let session = self.session.clone();
        let language = self.language;
        let cancel = self.cancel.clone();
        let git_executable = session.current_git_executable();
        let askpass_client = self.askpass.clone();
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
        // 只有实际打开仓库才会进入最近打开历史。
        let open_after = chosen.open_after_clone;
        cloning.set(true);
        clone_progress.set(String::new());
        cancel.store(false, Ordering::Relaxed);
        clone_error.set(None);
        spawn(async move {
            // 启动后按 100ms 轮询：界面一直可响应，取消也能立刻生效。
            let askpass = match crate::askpass::AskPassSession::new(git_executable, askpass_client)
            {
                Ok(session) => session,
                Err(error) => {
                    cloning.set(false);
                    clone_error.set(Some(error.to_string()));
                    return;
                }
            };
            let mut run = match git::start_clone_with_executable(
                url.trim(),
                destination.trim(),
                &options,
                askpass.executable(),
            ) {
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
                    if askpass.was_cancelled() {
                        break Err(i18n::t(lang.as_str(), "welcome.cloneCancelled"));
                    }
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
            license: files.license,
            name: self.state.init_name.get_untracked(),
            branch: self.state.init_branch.get_untracked(),
        };
        match git::init_with_executable(
            path.trim(),
            &options,
            &self.session.current_git_executable(),
        ) {
            Ok(()) => session.open_path(path.trim().to_string()),
            Err(error) => init_error.set(Some(localized_error(
                language.get_untracked().as_str(),
                &error,
            ))),
        }
    }

    /// 从左栏移除一条最近仓库；不影响已打开的标签页。
    pub(crate) fn forget_recent(&self, path: String) {
        let store_dir = self.session.store_dir.clone();
        let recent = self.session.recent;
        let next = store::forget_recent(&store_dir, &recent.get_untracked(), path.as_str());
        recent.set(next);
    }

    /// 聚焦左栏搜索框（⌘F）。
    pub(crate) fn focus_search(&self) {
        self.state.search_ref.focus();
    }

    /// 打开设置页的指定分区。
    pub(crate) fn open_settings(&self, section: Section) {
        self.settings.go_settings_section(section);
    }

    /// Check for Updates：更新服务尚未接入，给出真实状态说明。
    pub(crate) fn check_updates(&self) {
        let language = self.language.get_untracked();
        crate::interactions::alert(
            i18n::t(language.as_str(), "welcome.checkUpdates"),
            i18n::t(language.as_str(), "welcome.updateUnavailable"),
            i18n::t(language.as_str(), "common.close"),
        );
    }

    /// 切换 Open / Clone / Init 分段（原 `set_repo_form`）。
    pub(crate) fn set_repo_form(&self, form: RepoForm) {
        self.state.form.set(form);
        // 错误跟着表单走：切换后旧表单的失败不能出现在新表单里。
        self.state.open_error.set(None);
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
        GitError::NotARepository { path } => {
            i18n::with_arguments(language, "welcome.error.notRepo", &[("path", path.clone())])
        }
        GitError::Unreadable { path, .. } => i18n::with_arguments(
            language,
            "welcome.error.unreadable",
            &[("path", path.clone())],
        ),
        GitError::CommandFailed { .. } => i18n::t(language, "welcome.error.commandFailed"),
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
