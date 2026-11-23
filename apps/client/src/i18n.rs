//! 双语：读取 `locales/*.json`，按当前语言取文案。
//!
//! 文件保持 rust-i18n 的嵌套结构，构建期由 `nsh` 读取；这里在运行期把它展平成
//! `"welcome.cloneTitle"` 形式的键，因此同一份文件既可供 `t!` 使用，也可供本模块
//! 使用。键缺失时回退到英文，再缺失则返回键本身，便于发现漏翻。
//!
//! 语言用应用内的 `Signal<SharedString>` 保存（`en` / `zh-Hans`），不走系统的
//! `set_locale`，这样设置页可以在运行期切换而无需重启。

use chrono::{Datelike, Local, TimeZone, Timelike};
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
/// 输入是 `git log` 的 `%at`（UTC 秒）。持续时间按 Unix 秒计算，日历日期按系统本地
/// 时区计算；中文用“2 小时”“昨天”，英文用 `2h`、`Yesterday`。
/// 日期显示方式（设计板 23 的 Date format）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateStyle {
    Relative,
    Absolute,
    /// 跟随系统：这里退化成「绝对」，因为读系统区域的完整格式需要平台 API。
    System,
    Custom,
}

impl DateStyle {
    pub fn id(self) -> &'static str {
        return match self {
            DateStyle::Relative => "relative",
            DateStyle::Absolute => "absolute",
            DateStyle::System => "system",
            DateStyle::Custom => "custom",
        };
    }

    pub fn from_id(id: &str) -> DateStyle {
        return match id {
            "absolute" => DateStyle::Absolute,
            "system" => DateStyle::System,
            "custom" => DateStyle::Custom,
            _ => DateStyle::Relative,
        };
    }
}

/// 按设置格式化一个时间戳。
///
/// `pattern` 只在 `Custom` 时使用，支持 `%Y %m %d %e %b %B %H %M`，未知占位原样
/// 保留，方便用户看出自己写错了哪一段。绝对与自定义格式按系统本地时区显示。
pub fn format_date(
    language: &str,
    style: DateStyle,
    pattern: &str,
    timestamp: i64,
    now: i64,
) -> String {
    return match style {
        DateStyle::Relative => relative_time(language, timestamp, now),
        DateStyle::Absolute => absolute_time(language, timestamp),
        DateStyle::System => absolute_time(language, timestamp),
        DateStyle::Custom => custom_time(language, pattern, timestamp),
    };
}

/// 绝对时间：`2026-11-14 09:05`，按系统本地时区显示。
pub fn absolute_time(language: &str, timestamp: i64) -> String {
    let (month, day, year, hour, minute) = local_components(timestamp);
    let _ = language;
    return format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}");
}

/// 自定义 pattern。
pub fn custom_time(language: &str, pattern: &str, timestamp: i64) -> String {
    let (month, day, year, hour, minute) = local_components(timestamp);
    let month_name = t(
        language,
        &format!("time.months.{}", month.saturating_sub(1)),
    );
    let mut out = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '%' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('Y') => out.push_str(&format!("{year:04}")),
            Some('m') => out.push_str(&format!("{month:02}")),
            Some('d') => out.push_str(&format!("{day:02}")),
            Some('e') => out.push_str(&format!("{day}")),
            Some('b') => out.push_str(month_name.as_str()),
            Some('B') => out.push_str(month_name.as_str()),
            Some('H') => out.push_str(&format!("{hour:02}")),
            Some('M') => out.push_str(&format!("{minute:02}")),
            // 不认识的占位原样留着，用户能看出写错了。
            Some(other) => {
                out.push('%');
                out.push(other);
            }
            None => out.push('%'),
        }
    }
    return out;
}

/// 按系统本地时区拆分 Unix 秒；超出 chrono 表示范围时回退到 UTC 拆分。
fn local_components(timestamp: i64) -> (i64, i64, i64, i64, i64) {
    if let Some(datetime) = local_datetime(timestamp) {
        return (
            i64::from(datetime.month()),
            i64::from(datetime.day()),
            i64::from(datetime.year()),
            i64::from(datetime.hour()),
            i64::from(datetime.minute()),
        );
    }
    let (month, day, year) = civil_from_unix(timestamp);
    let (hour, minute) = clock_from_unix(timestamp);
    return (month, day, year, hour, minute);
}

fn local_datetime(timestamp: i64) -> Option<chrono::DateTime<Local>> {
    return Local.timestamp_opt(timestamp, 0).single();
}

fn local_day_difference(timestamp: i64, now: i64) -> Option<i64> {
    let timestamp = local_datetime(timestamp)?;
    let now = local_datetime(now)?;
    return Some((now.date_naive() - timestamp.date_naive()).num_days());
}

/// 一天里的时分（UTC 秒），仅供超出 chrono 日期范围时回退。
fn clock_from_unix(timestamp: i64) -> (i64, i64) {
    let seconds = timestamp.rem_euclid(86_400);
    return (seconds / 3_600, (seconds % 3_600) / 60);
}

pub fn relative_time(language: &str, timestamp: i64, now: i64) -> String {
    let seconds = (now - timestamp).max(0);
    let minutes = seconds / 60;
    if minutes < 1 {
        return t(language, "time.now");
    }
    if minutes < 60 {
        return with_arguments(language, "time.minutes", &[("count", minutes.to_string())]);
    }
    let hours = minutes / 60;
    if hours < 24 {
        return with_arguments(language, "time.hours", &[("count", hours.to_string())]);
    }
    let days = local_day_difference(timestamp, now)
        .unwrap_or_else(|| seconds / 86_400)
        .max(0);
    if days == 1 {
        return t(language, "time.yesterday");
    }
    if days == 0 {
        return with_arguments(language, "time.hours", &[("count", hours.to_string())]);
    }
    if days < 7 {
        return with_arguments(language, "time.days", &[("count", days.to_string())]);
    }
    let (month, day, year, _, _) = local_components(timestamp);
    let month_name = t(
        language,
        &format!("time.months.{}", month.saturating_sub(1)),
    );
    let is_this_year = {
        let (_, _, now_year, _, _) = local_components(now);
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

/// 把 UTC 秒换成 `(月, 日, 年)`，仅供超出 chrono 日期范围时回退。
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
    fn history_filter_date_heading_is_localized() {
        assert_eq!(t("en", "filter.whenLabel"), "Date");
        assert_eq!(t("zh-Hans", "filter.whenLabel"), "日期");
    }

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
        let text = with_arguments(
            "zh-Hans",
            "history.commits",
            &[("count", "128".to_string())],
        );
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
    fn utc_fallback_civil_dates_are_correct_for_known_instants() {
        // 2026-11-14T00:00:00Z
        assert_eq!(civil_from_unix(1_794_614_400), (11, 14, 2026));
    }

    #[test]
    fn date_styles_round_trip_and_format() {
        // 2026-11-14 09:05:00 UTC
        let stamp = 1_794_647_100;
        for style in [
            DateStyle::Relative,
            DateStyle::Absolute,
            DateStyle::System,
            DateStyle::Custom,
        ] {
            assert_eq!(DateStyle::from_id(style.id()), style);
        }
        assert_eq!(
            DateStyle::from_id("nonsense"),
            DateStyle::Relative,
            "未知值退回相对时间"
        );

        let absolute = format_date("en", DateStyle::Absolute, "", stamp, stamp + 60);
        let local = local_datetime(stamp).expect("known timestamp is representable");
        assert_eq!(absolute, local.format("%Y-%m-%d %H:%M").to_string());
        assert!(absolute.contains(':'), "绝对时间应带时分：{absolute}");

        let custom = format_date("en", DateStyle::Custom, "%Y/%m/%d %H:%M", stamp, stamp);
        assert_eq!(custom, local.format("%Y/%m/%d %H:%M").to_string());

        let named = format_date("en", DateStyle::Custom, "%b %e, %Y", stamp, stamp);
        assert!(named.contains(&local.year().to_string()), "{named}");
        assert!(named.contains(&local.day().to_string()), "{named}");
    }

    #[test]
    fn unknown_pattern_placeholders_are_kept() {
        let stamp = 1_794_647_100;
        assert_eq!(
            format_date("en", DateStyle::Custom, "%Y %q", stamp, stamp),
            "2026 %q",
            "认不出的占位应原样保留"
        );
        assert_eq!(
            format_date("en", DateStyle::Custom, "100%%", stamp, stamp),
            "100%%"
        );
    }
}
