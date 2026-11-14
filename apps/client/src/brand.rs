//! 产品身份：名称、版本、构建号与存储目录的唯一来源。
//!
//! 产品名尚未最终确定，代码里不直接写字符串；改这里（以及 `nsh.json` 的
//! `name`/`id`，CLI 需要它们）即可整体改名。`nsh.json` 无法读取 Rust 常量，
//! 是唯一的重复点，README 里有对应说明。

/// 产品名（工作名）。
pub const PRODUCT_NAME: &str = "gitui";
/// 版本号，取自 Cargo 包版本。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// 构建号；打包时由 `scripts/release-macos.sh` 覆盖 `nsh.json` 的 `build`。
pub const BUILD: &str = "1";
/// 应用支持目录名（`~/Library/Application Support/<name>`）。
pub const STORAGE_DIR: &str = "gitclient";
/// 窗口标题里显示的完整标识：`名称 版本 (构建号)`。
pub fn version_line() -> String {
    return format!("{} {} ({})", PRODUCT_NAME, VERSION, BUILD);
}
