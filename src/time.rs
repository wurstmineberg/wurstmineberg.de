use {
    std::{
        fmt,
        str::FromStr,
    },
    chrono::prelude::*,
    chrono_tz::Europe,
    rocket::response::content::RawHtml,
    rocket_util::html,
    serde_with::{
        DeserializeFromStr,
        SerializeDisplay,
    },
};

#[derive(Debug, Clone, Copy, DeserializeFromStr, SerializeDisplay)]
pub(crate) enum DateWithOptionalTime {
    DateTime(DateTime<Utc>),
    Date(NaiveDate),
}

impl DateWithOptionalTime {
    pub(crate) fn sort_key(&self) -> DateTime<Utc> {
        match *self {
            Self::DateTime(datetime) => datetime,
            Self::Date(date) => date.and_hms_opt(0, 0, 0).expect("wrong hardcoded datetime").and_utc(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("failed to parse date with optional time")]
pub(crate) struct DateWithOptionalTimeParseError {
    rfc3339: chrono::ParseError,
    legacy_utc: chrono::ParseError,
    date_only: chrono::ParseError,
}

impl FromStr for DateWithOptionalTime {
    type Err = DateWithOptionalTimeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match DateTime::parse_from_rfc3339(s) {
            Ok(datetime) => Ok(Self::DateTime(datetime.into())),
            Err(rfc3339) => match NaiveDateTime::parse_from_str(s, "%F %T") {
                Ok(datetime) => Ok(Self::DateTime(datetime.and_utc())),
                Err(legacy_utc) => match NaiveDate::parse_from_str(s, "%F") {
                    Ok(date) => Ok(Self::Date(date)),
                    Err(date_only) => Err(DateWithOptionalTimeParseError { rfc3339, legacy_utc, date_only }),
                },
            },
        }
    }
}

impl fmt::Display for DateWithOptionalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DateTime(datetime) => datetime.to_rfc3339_opts(SecondsFormat::AutoSi, true).fmt(f),
            Self::Date(date) => date.fmt(f),
        }
    }
}

pub(crate) struct DateTimeFormat {
    pub(crate) long: bool,
    pub(crate) running_text: bool,
}

pub(crate) fn format_date<Z: TimeZone>(date: DateTime<Z>) -> RawHtml<String>
where Z::Offset: fmt::Display {
    html! {
        span(class = "date", data_timestamp = date.timestamp_millis()) {
            : date.format("%B %-d, %Y").to_string();
        }
    }
}

pub(crate) fn format_date_naive(date: NaiveDate) -> RawHtml<String> {
    html! {
        span : date.format("%B %-d, %Y").to_string();
    }
}

pub(crate) fn format_datetime<Z: TimeZone>(datetime: DateTime<Z>, format: DateTimeFormat) -> RawHtml<String> {
    let utc = datetime.to_utc();
    let berlin = datetime.with_timezone(&Europe::Berlin);
    let berlin_same_date = berlin.date_naive() == utc.date_naive();
    let berlin = berlin.format(if berlin_same_date { "%H:%M %Z" } else { "%A %H:%M %Z" }).to_string();
    html! {
        //TODO once https://github.com/WentTheFox/SledgeHammerTime is out of beta and https://github.com/WentTheFox/SledgeHammerTime/issues/2 is fixed, format as a link, e.g. https://hammertime.cyou/?t=1723402800.000
        span(class = "datetime", data_timestamp = datetime.timestamp_millis(), data_long = format.long.to_string()) {
            : utc.format("%A, %B %-d, %Y, %H:%M UTC").to_string();
            @if format.running_text {
                : " (";
            } else {
                : " • ";
            }
            : berlin;
            @if format.running_text {
                : ")";
            }
        }
    }
}
