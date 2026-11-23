//! 应用层共用操作：被两个以上功能调用的写路径（全部暂存、同步、提交身份与引用刷新）。

use crate::app::session::{Sync, SyncOptions, Tab};
use crate::features::changes::ops::refresh_changes;
use crate::features::sync::ops::sync_with;
use git::Repo;

/// 暂存或取消暂存全部。
pub(crate) fn stage_all_or_none(tab: &mut Tab, stage: bool) {
    let repo = match tab.open_repo() {
        Ok(repo) => repo,
        Err(error) => {
            tab.error = Some(error.detail());
            return;
        }
    };
    let result = if stage {
        repo.stage_all()
    } else {
        repo.unstage_all()
    };
    match result {
        Ok(()) => refresh_changes(tab),
        Err(error) => tab.error = Some(error.detail()),
    }
}

/// 执行一次同步，并把结果留在标签页上（供操作条显示）。
pub(crate) fn sync(tab: &mut Tab, kind: Sync) {
    return sync_with(tab, kind, &SyncOptions::default());
}

/// 仓库级提交身份（`user.name` / `user.email`）。
pub(crate) fn identity(tab: &Tab) -> (String, String) {
    let Ok(repo) = tab.open_repo() else {
        return (String::new(), String::new());
    };
    let name = repo.config_get_local("user.name").unwrap_or_default();
    let email = repo.config_get_local("user.email").unwrap_or_default();
    return (name, email);
}

/// 刷新引用与其附属信息（远端 URL）。所有改引用的动作都走这里。
pub(crate) fn refresh_refs(tab: &mut Tab, repo: &Repo) {
    tab.refs = repo.refs().unwrap_or_default();
    tab.remote_urls = repo.remote_urls().unwrap_or_default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::session::load_tab;
    use crate::features::refs::ops::clear_identity;
    use crate::features::search::ops::set_identity;
    use crate::test_support::TempRepo;
    use git::LogFilter;

    #[test]
    fn identity_reads_writes_and_clears_repo_config() {
        let repo = TempRepo::new("identity");
        let mut tab = load_tab(&repo.path.display().to_string(), &LogFilter::default());

        // 测试仓库自带仓库级身份（fixture 里设的）。
        assert_eq!(
            identity(&tab),
            ("Test".to_string(), "test@example.com".to_string())
        );

        set_identity(&tab, "Ada", "ada@example.com").expect("set identity");
        assert_eq!(
            identity(&tab),
            ("Ada".to_string(), "ada@example.com".to_string())
        );

        // 清掉后回到「没有仓库级身份」，与全局配置无关。
        clear_identity(&tab).expect("clear identity");
        assert_eq!(identity(&tab), (String::new(), String::new()));

        // 再清一次不应报错（键已不存在）。
        clear_identity(&tab).expect("clear twice");
        tab.summary = "x".to_string();
    }
}
