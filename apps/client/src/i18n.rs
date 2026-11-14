//! 双语：读取 `locales/*.json`，按当前语言取文案。
//!
//! 文件保持 rust-i18n 的嵌套结构，构建期由 `nsh` 读取；这里在运行期把它展平成
//! `"welcome.cloneTitle"` 形式的键，因此同一份文件既可供 `t!` 使用，也可供本模块
//! 使用。键缺失时回退到英文，再缺失则返回键本身，便于发现漏翻。
//!
//! 语言用应用内的 `Signal<SharedString>` 保存（`en` / `zh-Hans`），不走系统的
//! `set_locale`，这样设置页可以在运行期切换而无需重启。

use std::collections::HashMap;
use std::sync::LazyLock;

const EN: &str = include_str!("../locales/en.json");
const ZH_HANS: &str = include_str!("../locales/zh-Hans.json");

/// 界面语言标识：与 `locales` 文件名一致。默认英文。
pub const EN_US: &str = "en";
pub const ZH_HANS_LANG: &str = "zh-Hans";
/// 首次启动使用的语言。
pub const DEFAULT_LANGUAGE: &str = EN_US;

static EN_MAP: LazyLock<HashMap<String, String>> = LazyLock::new(|| flatten(EN));
static ZH_MAP: LazyLock<HashMap<String, String>> = LazyLock::new(|| flatten(ZH_HANS));

/// 取一条文案；`arguments` 以 `%{name}` 形式替换。
pub fn t(language: &str, key: &str) -> String {
    return with_arguments(language, key, &[]);
}

/// 取一条文案并替换 `%{name}` 占位符。
pub fn with_arguments(language: &str, key: &str, arguments: &[(&str, String)]) -> String {
    let map = if language == ZH_HANS_LANG {
        &*ZH_MAP
    } else {
        &*EN_MAP
    };
    let template = map
        .get(key)
        .or_else(|| EN_MAP.get(key))
        .cloned()
        .unwrap_or_else(|| key.to_string());
    if arguments.is_empty() {
        return template;
    }
    let mut text = template;
    for (name, value) in arguments {
        text = text.replace(&format!("%{{{}}}", name), value);
    }
    return text;
}

/// 语言切换按钮使用的下一语言。
pub fn toggle(language: &str) -> &'static str {
    if language == ZH_HANS_LANG {
        return EN_US;
    }
    return ZH_HANS_LANG;
}

fn flatten(json: &str) -> HashMap<String, String> {
    let value: serde_json::Value = serde_json::from_str(json).expect("locales parse");
    let mut out = HashMap::new();
    flatten_into("", &value, &mut out);
    return out;
}

fn flatten_into(prefix: &str, value: &serde_json::Value, out: &mut HashMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                let next = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_into(&next, child, out);
            }
        }
        serde_json::Value::String(text) => {
            out.insert(prefix.to_string(), text.clone());
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let next = format!("{}.{}", prefix, index);
                flatten_into(&next, item, out);
            }
        }
        _ => {}
    }
}

/// 相对时间：设计的提交行显示 `2h`、`Yesterday`、`Mon`、`Nov 6`。
///
/// 输入是 `git log` 的 `%at`（UTC 秒）。以 `now` 为基准计算，避免依赖系统
/// 语言；中文用“2 小时”“昨天”，英文用 `2h`、`Yesterday`。
pub fn relative_time(language: &str, timestamp: i64, now: i64) -> String {
    let seconds = (now - timestamp).max(0);
    let minutes = seconds / 60;
    if minutes < 1 {
        return t(language, "time.now");
    }
    if minutes < 60 {
        return with_arguments(
            language,
            "time.minutes",
            &[("count", minutes.to_string())],
        );
    }
    let hours = minutes / 60;
    if hours < 24 {
        return with_arguments(language, "time.hours", &[("count", hours.to_string())]);
    }
    let days = hours / 24;
    if days == 1 {
        return t(language, "time.yesterday");
    }
    if days < 7 {
        return with_arguments(language, "time.days", &[("count", days.to_string())]);
    }
    let (month, day, year) = civil_from_unix(timestamp);
    let month_name = t(language, &format!("time.months.{}", month.saturating_sub(1)));
    let is_this_year = {
        let (_, _, now_year) = civil_from_unix(now);
        year == now_year
    };
    if is_this_year {
        return with_arguments(
            language,
            "time.monthDay",
            &[("month", month_name), ("day", day.to_string())],
        );
    }
    return with_arguments(
        language,
        "time.monthDayYear",
        &[
            ("month", month_name),
            ("day", day.to_string()),
            ("year", year.to_string()),
        ],
    );
}

/// 把 UTC 秒换成 `(月, 日, 年)`。只用于展示，不追求时区精度：按本地时区偏移
/// 没有跨平台 API，这里用 UTC，跨日误差可忽略。
fn civil_from_unix(timestamp: i64) -> (i64, i64, i64) {
    let days = timestamp.div_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    return (m, d, year);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_languages_have_the_same_keys() {
        let mut english: Vec<&String> = EN_MAP.keys().collect();
        let mut chinese: Vec<&String> = ZH_MAP.keys().collect();
        english.sort();
        chinese.sort();
        assert_eq!(english, chinese, "中英文案键集合不一致");
    }

    #[test]
    fn no_translation_is_empty() {
        for (key, value) in EN_MAP.iter().chain(ZH_MAP.iter()) {
            assert!(!value.trim().is_empty(), "{} 是空文案", key);
        }
    }

    #[test]
    fn missing_key_falls_back_to_the_key() {
        assert_eq!(t("en", "does.not.exist"), "does.not.exist");
    }

    #[test]
    fn arguments_replace_placeholders() {
        let text = with_arguments("en", "history.commits", &[("count", "128".to_string())]);
        assert_eq!(text, "128 commits");
        let text = with_arguments("zh-Hans", "history.commits", &[("count", "128".to_string())]);
        assert_eq!(text, "128 次提交");
    }

    #[test]
    fn relative_time_covers_the_designs_cases() {
        let now = 1_800_000_000;
        assert_eq!(relative_time("en", now - 30, now), "now");
        assert_eq!(relative_time("en", now - 60 * 5, now), "5m");
        assert_eq!(relative_time("en", now - 3_600 * 2, now), "2h");
        assert_eq!(relative_time("en", now - 86_400, now), "Yesterday");
        assert_eq!(relative_time("zh-Hans", now - 3_600 * 2, now), "2 小时");
    }

    #[test]
    fn civil_dates_are_correct_for_known_instants() {
        // 2026-11-14T00:00:00Z
        assert_eq!(civil_from_unix(1_794_614_400), (11, 14, 2026));
    }
}
