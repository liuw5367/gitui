//! History filters: draft state and its translation to Git log conditions.

use git::LogFilter;

/// Preset dates are localized by `filter.when.*`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum When {
    #[default]
    All,
    Today,
    Week,
    Month,
    Year,
    Custom,
}

impl When {
    pub const ALL: [When; 6] = [
        When::All,
        When::Today,
        When::Week,
        When::Month,
        When::Year,
        When::Custom,
    ];

    pub fn key(self) -> &'static str {
        return match self {
            When::All => "filter.when.all",
            When::Today => "filter.when.today",
            When::Week => "filter.when.week",
            When::Month => "filter.when.month",
            When::Year => "filter.when.year",
            When::Custom => "filter.when.custom",
        };
    }

    #[cfg(test)]
    pub fn from_key(key: &str) -> When {
        return match key {
            "today" => When::Today,
            "week" => When::Week,
            "month" => When::Month,
            "year" => When::Year,
            "custom" => When::Custom,
            _ => When::All,
        };
    }

    #[cfg(test)]
    pub fn id(self) -> &'static str {
        return match self {
            When::All => "all",
            When::Today => "today",
            When::Week => "week",
            When::Month => "month",
            When::Year => "year",
            When::Custom => "custom",
        };
    }
}

/// Values being edited in the filter popover. They only affect history after Apply.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FilterDraft {
    pub(crate) references: Vec<String>,
    pub(crate) authors: Vec<String>,
    pub(crate) when: When,
    pub(crate) since: String,
    pub(crate) until: String,
    pub(crate) path: String,
    pub(crate) message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FilterError {
    InvalidSince,
    InvalidUntil,
    ReversedRange,
}

/// Convert the draft into conditions. Git parses date-only values in the user's local timezone;
/// custom boundaries are explicit and inclusive to the second.
pub(crate) fn build_draft(draft: &FilterDraft) -> Result<LogFilter, FilterError> {
    let (since, until) = match draft.when {
        When::All => (None, None),
        When::Today => (Some("midnight".to_string()), None),
        When::Week => (Some("7 days ago".to_string()), None),
        When::Month => (Some("30 days ago".to_string()), None),
        When::Year => (Some("365 days ago".to_string()), None),
        When::Custom => {
            let since = clean(&draft.since);
            let until = clean(&draft.until);
            if since.as_deref().is_some_and(|date| !valid_date(date)) {
                return Err(FilterError::InvalidSince);
            }
            if until.as_deref().is_some_and(|date| !valid_date(date)) {
                return Err(FilterError::InvalidUntil);
            }
            if let (Some(since), Some(until)) = (since.as_deref(), until.as_deref())
                && days_since_epoch(since) > days_since_epoch(until)
            {
                return Err(FilterError::ReversedRange);
            }
            (
                since.map(|date| format!("{date} 00:00:00")),
                until.map(|date| format!("{date} 23:59:59")),
            )
        }
    };

    let mut references = clean_all(&draft.references);
    references.sort();
    references.dedup();
    let mut authors = clean_all(&draft.authors);
    authors.sort();
    authors.dedup();
    return Ok(LogFilter {
        since,
        until,
        references,
        authors,
        path: clean(&draft.path),
        message: draft.message.trim().to_string(),
        ..LogFilter::default()
    });
}

/// Kept for the existing quick-filter conversion callers and tests.
#[cfg(test)]
fn build(query: &str, reference: &str, author: &str, when: When, _now: i64) -> LogFilter {
    let reference = reference.trim();
    let author = author.trim();
    let since = match when {
        When::All | When::Custom => None,
        When::Today => Some("midnight".to_string()),
        When::Week => Some("7 days ago".to_string()),
        When::Month => Some("30 days ago".to_string()),
        When::Year => Some("365 days ago".to_string()),
    };
    return LogFilter {
        query: query.trim().to_string(),
        author: if author.is_empty() {
            None
        } else {
            Some(author.to_string())
        },
        since,
        reference: if reference.is_empty() {
            None
        } else {
            Some(reference.to_string())
        },
        ..LogFilter::default()
    };
}

fn clean(value: &str) -> Option<String> {
    let value = value.trim();
    return (!value.is_empty()).then(|| value.to_string());
}

fn clean_all(values: &[String]) -> Vec<String> {
    return values.iter().filter_map(|value| clean(value)).collect();
}

fn valid_date(value: &str) -> bool {
    let mut fields = value.split('-');
    let Some(year) = fields.next().and_then(|part| part.parse::<i64>().ok()) else {
        return false;
    };
    let Some(month) = fields.next().and_then(|part| part.parse::<i64>().ok()) else {
        return false;
    };
    let Some(day) = fields.next().and_then(|part| part.parse::<i64>().ok()) else {
        return false;
    };
    if fields.next().is_some() || year < 1 || !(1..=12).contains(&month) {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        2 if leap => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    return (1..=days).contains(&day) && value.len() == 10;
}

fn days_since_epoch(value: &str) -> i64 {
    let mut fields = value.split('-').filter_map(|part| part.parse::<i64>().ok());
    let Some(mut year) = fields.next() else {
        return 0;
    };
    let Some(month) = fields.next() else {
        return 0;
    };
    let Some(day) = fields.next() else {
        return 0;
    };
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * adjusted_month + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    return era * 146_097 + doe - 719_468;
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn week_preset_uses_a_git_relative_boundary() {
        let filter = build("fix", "main", "Mia Chen", When::Week, NOW);
        assert_eq!(filter.query, "fix");
        assert_eq!(filter.reference.as_deref(), Some("main"));
        assert_eq!(filter.author.as_deref(), Some("Mia Chen"));
        assert_eq!(filter.since.as_deref(), Some("7 days ago"));
    }

    #[test]
    fn when_presets_round_trip_through_their_ids() {
        for when in When::ALL {
            assert_eq!(When::from_key(when.id()), when);
        }
        assert_eq!(When::from_key("nonsense"), When::All);
    }
}
