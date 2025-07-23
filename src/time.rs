use {
    chrono::prelude::*,
    chrono_tz::Europe,
    rocket::response::content::RawHtml,
    rocket_util::html,
};

pub(crate) struct DateTimeFormat {
    pub(crate) long: bool,
    pub(crate) running_text: bool,
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
