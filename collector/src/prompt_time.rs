use chrono::{DateTime, FixedOffset, Utc};

const MINIMAX_PROMPT_UTC_OFFSET_SECONDS: i32 = 8 * 60 * 60;

pub(crate) fn minimax_prompt_timestamp(value: DateTime<Utc>) -> String {
    let offset = FixedOffset::east_opt(MINIMAX_PROMPT_UTC_OFFSET_SECONDS)
        .expect("UTC+8 offset must be valid");
    value.with_timezone(&offset).to_rfc3339()
}
