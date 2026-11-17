//! 日期显示格式的全局设置（设计板 23 的 Date format）。
//!
//! 时间到处都要显示（历史、reflog、检查器、Agent 日志、欢迎页），逐个视图串
//! `Settings` 只会把签名撑大。这里放一份进程内设置：设置页改了就调用
//! [`set_style`] / [`set_pattern`]，渲染时统一走 [`format`]。
//!
//! 不涉及仓库状态，也不做持久化——持久化在 `store::Settings`，启动时读出来写进这里。

use crate::i18n;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};

/// 当前使用的风格（存 `DateStyle::id` 的编码）。
static STYLE: AtomicU8 = AtomicU8::new(0);
/// 自定义 pattern。`Mutex` 比 `RwLock` 简单，冲突只在启动与设置改动时发生。
static PATTERN: Mutex<String> = Mutex::new(String::new());

fn encode(style: i18n::DateStyle) -> u8 {
    return match style {
        i18n::DateStyle::Relative => 0,
        i18n::DateStyle::Absolute => 1,
        i18n::DateStyle::System => 2,
        i18n::DateStyle::Custom => 3,
    };
}

fn decode(value: u8) -> i18n::DateStyle {
    return match value {
        1 => i18n::DateStyle::Absolute,
        2 => i18n::DateStyle::System,
        3 => i18n::DateStyle::Custom,
        _ => i18n::DateStyle::Relative,
    };
}

/// 从持久化的字符串装上当前设置。
pub fn load(style_id: &str, pattern: &str) {
    STYLE.store(
        encode(i18n::DateStyle::from_id(style_id)),
        Ordering::Relaxed,
    );
    set_pattern(pattern);
}

/// 切换风格。
pub fn set_style(style: i18n::DateStyle) {
    STYLE.store(encode(style), Ordering::Relaxed);
}

/// 当前风格。
pub fn style() -> i18n::DateStyle {
    return decode(STYLE.load(Ordering::Relaxed));
}

/// 设置自定义 pattern。
pub fn set_pattern(pattern: &str) {
    if let Ok(mut slot) = PATTERN.lock() {
        *slot = pattern.to_string();
    }
}

/// 当前 pattern（拿不到锁时返回空串，渲染退回默认格式）。
pub fn pattern() -> String {
    return PATTERN.lock().map(|slot| slot.clone()).unwrap_or_default();
}

/// 按当前设置格式化时间戳。
pub fn format(language: &str, timestamp: i64, now: i64) -> String {
    return i18n::format_date(language, style(), pattern().as_str(), timestamp, now);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_and_pattern_round_trip() {
        // 测试之间共享全局状态，因此每一步都显式设置再看结果。
        load("absolute", "%Y-%m-%d");
        assert_eq!(style(), i18n::DateStyle::Absolute);
        assert_eq!(pattern(), "%Y-%m-%d");

        set_style(i18n::DateStyle::Custom);
        set_pattern("%d/%m/%Y");
        let stamp = 1_794_647_100;
        assert_eq!(format("en", stamp, stamp), "14/11/2026");

        // 未知 id 退回相对时间。
        load("nonsense", "");
        assert_eq!(style(), i18n::DateStyle::Relative);
    }
}
