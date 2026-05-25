use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

use crate::models::{
    ActivityBucket, ActivityCategory, AttentionState, BucketEvidence, TimeEvent, TimeEventKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityBucketQuery {
    pub date: String,
    pub bucket_seconds: i64,
}

#[derive(Debug, Clone)]
struct BucketAccumulator {
    start_at: DateTime<Utc>,
    end_at: DateTime<Utc>,
    evidence: Vec<BucketEvidence>,
}

impl BucketAccumulator {
    fn new(start_at: DateTime<Utc>, bucket_seconds: i64) -> Self {
        Self {
            start_at,
            end_at: start_at + chrono::Duration::seconds(bucket_seconds),
            evidence: Vec::new(),
        }
    }
}

pub fn build_activity_buckets(
    events: &[TimeEvent],
    query: ActivityBucketQuery,
) -> Vec<ActivityBucket> {
    let Some((date_start, date_end)) = date_bounds(&query.date) else {
        return Vec::new();
    };
    let bucket_seconds = query.bucket_seconds.max(1);
    let mut buckets: BTreeMap<i64, BucketAccumulator> = BTreeMap::new();

    for event in events {
        let Some(duration_seconds) = event.duration_seconds else {
            continue;
        };
        if duration_seconds <= 0 {
            continue;
        }
        let Some(event_end) = event.ended_at else {
            continue;
        };
        if event_end <= event.started_at {
            continue;
        }

        let interval_start = event.started_at.max(date_start);
        let interval_end = event_end.min(date_end);
        if interval_end <= interval_start {
            continue;
        }

        let mut cursor = interval_start;
        while cursor < interval_end {
            let bucket_start = floor_to_bucket(cursor, bucket_seconds);
            let bucket_end = bucket_start + chrono::Duration::seconds(bucket_seconds);
            let slice_end = interval_end.min(bucket_end);
            let slice_seconds = slice_end.signed_duration_since(cursor).num_seconds().max(0);
            if slice_seconds > 0 {
                let entry = buckets
                    .entry(bucket_start.timestamp())
                    .or_insert_with(|| BucketAccumulator::new(bucket_start, bucket_seconds));
                entry.evidence.push(BucketEvidence {
                    event_id: event.id.clone(),
                    app: event.app.clone(),
                    title: event.title.clone(),
                    normalized_title: normalize_browser_title(&event.app, &event.title),
                    kind: event.kind.clone(),
                    started_at: cursor,
                    ended_at: slice_end,
                    duration_seconds: slice_seconds,
                });
            }
            cursor = slice_end;
        }
    }

    buckets
        .into_values()
        .filter_map(|bucket| build_bucket(bucket, bucket_seconds))
        .collect()
}

pub fn normalize_browser_title(process_name: &str, title: &str) -> String {
    let mut normalized = title.trim().to_string();
    if !is_browser_process(process_name) {
        return normalized;
    }

    for suffix in [
        " - Google Chrome",
        " - Microsoft Edge",
        " - 个人 - Microsoft Edge",
        " - Personal - Microsoft Edge",
    ] {
        if normalized.ends_with(suffix) {
            normalized.truncate(normalized.len() - suffix.len());
            normalized = normalized.trim().to_string();
        }
    }

    for marker in [" 和另外 ", " and "] {
        if let Some(index) = normalized.find(marker) {
            normalized.truncate(index);
            normalized = normalized.trim().to_string();
        }
    }

    normalized
        .trim_matches(|c: char| c == '-' || c.is_whitespace())
        .trim()
        .to_string()
}

fn build_bucket(mut bucket: BucketAccumulator, bucket_seconds: i64) -> Option<ActivityBucket> {
    bucket.evidence.sort_by(|left, right| {
        left.started_at
            .cmp(&right.started_at)
            .then_with(|| left.event_id.cmp(&right.event_id))
    });
    let dominant = bucket
        .evidence
        .iter()
        .max_by_key(|evidence| evidence.duration_seconds)?;
    let total_seconds: i64 = bucket
        .evidence
        .iter()
        .map(|evidence| evidence.duration_seconds)
        .sum();
    if total_seconds <= 0 {
        return None;
    }
    let switch_count = count_switches(&bucket.evidence);
    let category = categorize(dominant);
    let dominant_share = dominant.duration_seconds as f64 / total_seconds as f64;
    let attention_state = attention_state(&category, switch_count, dominant_share);

    Some(ActivityBucket {
        id: format!("bucket-{}", bucket.start_at.timestamp()),
        start_at: bucket.start_at,
        end_at: bucket.end_at,
        bucket_seconds,
        dominant_app: dominant.app.clone(),
        dominant_title: dominant.title.clone(),
        normalized_title: dominant.normalized_title.clone(),
        dominant_duration_seconds: dominant.duration_seconds,
        switch_count,
        project_id: None,
        project_name: None,
        activity_category: category,
        attention_state,
        confidence: confidence(dominant_share),
        evidence: bucket.evidence,
        visual_summary_id: None,
    })
}

fn count_switches(evidence: &[BucketEvidence]) -> usize {
    let active: Vec<_> = evidence
        .iter()
        .filter(|item| item.kind == TimeEventKind::ActiveWindow)
        .collect();
    active
        .windows(2)
        .filter(|pair| {
            pair[0].app != pair[1].app || pair[0].normalized_title != pair[1].normalized_title
        })
        .count()
}

fn categorize(evidence: &BucketEvidence) -> ActivityCategory {
    if evidence.kind == TimeEventKind::Lifecycle {
        return ActivityCategory::Idle;
    }

    let app = evidence.app.to_ascii_lowercase();
    let title = evidence.normalized_title.to_ascii_lowercase();
    let combined = format!("{app} {title}");

    if contains_any(&combined, &["code", "cursor", "cargo", "rust", "tsr"]) {
        ActivityCategory::Coding
    } else if contains_any(&combined, &["word", "docx", "writing", "文档"]) {
        ActivityCategory::Writing
    } else if contains_any(&combined, &["weixin", "wechat", "mail", "outlook", "微信"]) {
        ActivityCategory::Communication
    } else if is_browser_process(&evidence.app) {
        if title.is_empty() || contains_any(&title, &["youtube", "bilibili", "netflix"]) {
            ActivityCategory::Unknown
        } else {
            ActivityCategory::Research
        }
    } else {
        ActivityCategory::Unknown
    }
}

fn attention_state(
    category: &ActivityCategory,
    switch_count: usize,
    dominant_share: f64,
) -> AttentionState {
    if *category == ActivityCategory::Idle {
        return AttentionState::Away;
    }
    if switch_count <= 1 && dominant_share >= 0.85 {
        AttentionState::DeepFocus
    } else if switch_count <= 2 && dominant_share >= 0.75 {
        AttentionState::Steady
    } else if switch_count <= 5 || dominant_share >= 0.45 {
        AttentionState::LightSwitching
    } else {
        AttentionState::Fragmented
    }
}

fn confidence(dominant_share: f64) -> f64 {
    (dominant_share * 100.0).round() / 100.0
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn is_browser_process(process_name: &str) -> bool {
    let value = process_name.to_ascii_lowercase();
    value.contains("chrome") || value.contains("msedge") || value.contains("edge")
}

fn date_bounds(date: &str) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let start = Utc
        .from_local_datetime(&date.and_hms_opt(0, 0, 0)?)
        .single()?;
    Some((start, start + chrono::Duration::days(1)))
}

fn floor_to_bucket(value: DateTime<Utc>, bucket_seconds: i64) -> DateTime<Utc> {
    let timestamp = value.timestamp();
    let bucket_timestamp = timestamp - timestamp.rem_euclid(bucket_seconds);
    Utc.timestamp_opt(bucket_timestamp, 0)
        .single()
        .unwrap_or(value)
}
