//! 历史筛选：把界面上的选项翻译成 `git log` 的条件。
//!
//! 时间预设以“天”为单位换算成 `--since`，用注入的 `now` 计算，便于测试。

use git::LogFilter;

/// 时间预设的标识，与 `locales` 的 `filter.when.*` 对应。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    All,
    Today,
    Week,
    Month,
    Year,
}

impl When {
    /// 界面上可选的顺序。
    pub const ALL: [When; 5] = [When::All, When::Today, When::Week, When::Month, When::Year];

    pub fn key(self) -> &'static str {
        return match self {
            When::All => "filter.when.all",
            When::Today => "filter.when.today",
            When::Week => "filter.when.week",
            When::Month => "filter.when.month",
            When::Year => "filter.when.year",
        };
    }

    /// 从标识还原；未知值按“全部”处理。
    pub fn from_key(key: &str) -> When {
        return match key {
            "today" => When::Today,
            "week" => When::Week,
            "month" => When::Month,
            "year" => When::Year,
            _ => When::All,
        };
    }

    /// 存储用的短标识。
    pub fn id(self) -> &'static str {
        return match self {
            When::All => "all",
            When::Today => "today",
            When::Week => "week",
            When::Month => "month",
            When::Year => "year",
        };
    }

    /// 距今天数；`None` 表示不限制。
    pub fn days(self) -> Option<i64> {
        return match self {
            When::All => None,
            When::Today => Some(1),
            When::Week => Some(7),
            When::Month => Some(30),
            When::Year => Some(365),
        };
    }
}

/// 组装 `LogFilter`。
///
/// - `reference`：只在该分支/标签上读取；空字符串表示当前 HEAD 的全部历史。
/// - `author`：空字符串表示不限。
/// - `when`：时间预设。
pub fn build(
    query: &str,
    reference: &str,
    author: &str,
    when: When,
    now: i64,
) -> LogFilter {
    let reference = reference.trim();
    let author = author.trim();
    return LogFilter {
        query: query.trim().to_string(),
        author: if author.is_empty() {
            None
        } else {
            Some(author.to_string())
        },
        since: when.days().map(|days| date_days_before(now, days)),
        until: None,
        reference: if reference.is_empty() {
            None
        } else {
            Some(reference.to_string())
        },
    };
}

/// `now` 之前 `days` 天的日期，`YYYY-MM-DD`。
fn date_days_before(now: i64, days: i64) -> String {
    let target = now - days * 86_400;
    let (year, month, day) = civil(target);
    return format!("{:04}-{:02}-{:02}", year, month, day);
}

/// UTC 秒转 (年, 月, 日)。
fn civil(timestamp: i64) -> (i64, i64, i64) {
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
    return (year, m, d);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 2026-11-14T00:00:00Z
    const NOW: i64 = 1_794_614_400;

    #[test]
    fn all_means_no_limits() {
        let filter = build("", "", "", When::All, NOW);
        assert_eq!(filter, LogFilter::default());
    }

    #[test]
    fn empty_strings_are_treated_as_unset() {
        let filter = build("  ", " ", " ", When::All, NOW);
        assert_eq!(filter.reference, None);
        assert_eq!(filter.author, None);
        assert_eq!(filter.query, "");
    }

    #[test]
    fn week_preset_subtracts_seven_days() {
        let filter = build("fix", "main", "Mia Chen", When::Week, NOW);
        assert_eq!(filter.query, "fix");
        assert_eq!(filter.reference.as_deref(), Some("main"));
        assert_eq!(filter.author.as_deref(), Some("Mia Chen"));
        assert_eq!(filter.since.as_deref(), Some("2026-11-07"));
    }

    #[test]
    fn when_presets_round_trip_through_their_ids() {
        for when in When::ALL {
            assert_eq!(When::from_key(when.id()), when);
        }
        assert_eq!(When::from_key("nonsense"), When::All);
    }

    #[test]
    fn preset_days_match_their_names() {
        assert_eq!(When::Today.days(), Some(1));
        assert_eq!(When::Week.days(), Some(7));
        assert_eq!(When::Month.days(), Some(30));
        assert_eq!(When::Year.days(), Some(365));
        assert_eq!(When::All.days(), None);
    }
}
