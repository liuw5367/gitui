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
/// 应用数据目录名（`~/.config/<name>`）。
pub const STORAGE_DIR: &str = "gitui";
/// 外观偏好，与 `nsh.json` 的 `appearance` 同义。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ThemeMode {
    /// 跟随系统。
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeMode {
    /// 存储用的短标识。
    pub fn id(self) -> &'static str {
        return match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        };
    }

    /// 从标识还原；未知值按跟随系统处理。
    pub fn from_id(id: &str) -> ThemeMode {
        return match id {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
        };
    }

    /// 切换到下一个模式（跟随系统 → 浅色 → 深色）。
    pub fn next(self) -> ThemeMode {
        return match self {
            ThemeMode::System => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
        };
    }

    /// 对应 NSH 的外观值。
    pub fn appearance(self) -> nsh_core::platform::Appearance {
        return match self {
            ThemeMode::System => nsh_core::platform::Appearance::System,
            ThemeMode::Light => nsh_core::platform::Appearance::Light,
            ThemeMode::Dark => nsh_core::platform::Appearance::Dark,
        };
    }
}

/// 窗口标题里显示的完整标识：`名称 版本 (构建号)`。
pub fn version_line() -> String {
    return format!("{} {} ({})", PRODUCT_NAME, VERSION, BUILD);
}
