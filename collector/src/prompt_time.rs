use chrono::{DateTime, FixedOffset, Utc};

const OWNER_LOCAL_UTC_OFFSET_SECONDS: i32 = 8 * 60 * 60;
const OWNER_LOCAL_TIMEZONE_LABEL: &str = "Asia/Shanghai UTC+8";

pub(crate) fn minimax_prompt_timestamp(value: DateTime<Utc>) -> String {
    human_report_timestamp(value)
}

pub(crate) fn human_report_timestamp(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&owner_local_offset())
        .format("%Y-%m-%dT%H:%M:%S%:z")
        .to_string()
}

pub(crate) fn human_report_clock(value: DateTime<Utc>) -> String {
    value
        .with_timezone(&owner_local_offset())
        .format("%H:%M")
        .to_string()
}

pub(crate) fn human_report_range(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
    format!("{}-{}", human_report_clock(start), human_report_clock(end))
}

pub(crate) fn human_report_timezone_label() -> &'static str {
    OWNER_LOCAL_TIMEZONE_LABEL
}

fn owner_local_offset() -> FixedOffset {
    FixedOffset::east_opt(OWNER_LOCAL_UTC_OFFSET_SECONDS).expect("UTC+8 offset must be valid")
}

#[cfg(test)]
mod prompt_time_tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn human_report_times_use_owner_utc_plus_eight_clock() {
        let start = Utc.with_ymd_and_hms(2026, 6, 7, 2, 0, 0).single().unwrap();
        let end = Utc.with_ymd_and_hms(2026, 6, 7, 7, 0, 0).single().unwrap();

        assert_eq!(human_report_timestamp(start), "2026-06-07T10:00:00+08:00");
        assert_eq!(human_report_clock(start), "10:00");
        assert_eq!(human_report_range(start, end), "10:00-15:00");
        assert_eq!(human_report_timezone_label(), "Asia/Shanghai UTC+8");
        assert_eq!(minimax_prompt_timestamp(start), human_report_timestamp(start));
    }
}
