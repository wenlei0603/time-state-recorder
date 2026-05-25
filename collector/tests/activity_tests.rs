use chrono::{DateTime, Utc};
use tsr_collector::{
    activity::{ActivityBucketQuery, build_activity_buckets, normalize_browser_title},
    models::{ActivityCategory, AttentionState, TimeEvent, TimeEventKind},
};

#[test]
fn buckets_choose_dominant_activity_and_count_switches() {
    let events = vec![
        active_event(
            "raw-1",
            "Code",
            "main.rs",
            "2026-05-23T09:00:00Z",
            "2026-05-23T09:02:00Z",
        ),
        active_event(
            "raw-2",
            "Weixin.exe",
            "微信",
            "2026-05-23T09:02:00Z",
            "2026-05-23T09:02:30Z",
        ),
        active_event(
            "raw-3",
            "chrome.exe",
            "GitHub - Pull Request - Google Chrome",
            "2026-05-23T09:02:30Z",
            "2026-05-23T09:03:00Z",
        ),
    ];

    let buckets = build_activity_buckets(
        &events,
        ActivityBucketQuery {
            date: "2026-05-23".to_string(),
            bucket_seconds: 180,
        },
    );

    assert_eq!(buckets.len(), 1);
    assert_eq!(buckets[0].start_at, ts("2026-05-23T09:00:00Z"));
    assert_eq!(buckets[0].end_at, ts("2026-05-23T09:03:00Z"));
    assert_eq!(buckets[0].bucket_seconds, 180);
    assert_eq!(buckets[0].dominant_app, "Code");
    assert_eq!(buckets[0].dominant_title, "main.rs");
    assert_eq!(buckets[0].normalized_title, "main.rs");
    assert_eq!(buckets[0].dominant_duration_seconds, 120);
    assert_eq!(buckets[0].switch_count, 2);
    assert_eq!(buckets[0].activity_category, ActivityCategory::Coding);
    assert_eq!(buckets[0].attention_state, AttentionState::LightSwitching);
}

#[test]
fn normalizes_browser_titles() {
    assert_eq!(
        normalize_browser_title("chrome.exe", "GitHub - Pull Request - Google Chrome"),
        "GitHub - Pull Request"
    );
    assert_eq!(
        normalize_browser_title(
            "msedge.exe",
            "Time State Recorder 和另外 2 个页面 - 个人 - Microsoft Edge",
        ),
        "Time State Recorder"
    );
}

#[test]
fn does_not_infer_loafing_without_explicit_rule() {
    let events = vec![active_event(
        "raw-1",
        "chrome.exe",
        "YouTube - Google Chrome",
        "2026-05-23T09:00:00Z",
        "2026-05-23T09:03:00Z",
    )];

    let buckets = build_activity_buckets(
        &events,
        ActivityBucketQuery {
            date: "2026-05-23".to_string(),
            bucket_seconds: 180,
        },
    );

    assert_eq!(buckets[0].activity_category, ActivityCategory::Unknown);
}

fn active_event(id: &str, app: &str, title: &str, started_at: &str, ended_at: &str) -> TimeEvent {
    TimeEvent {
        id: id.to_string(),
        app: app.to_string(),
        title: title.to_string(),
        kind: TimeEventKind::ActiveWindow,
        status: None,
        session_id: "session-1".to_string(),
        started_at: ts(started_at),
        ended_at: Some(ts(ended_at)),
        duration_seconds: Some(
            ts(ended_at)
                .signed_duration_since(ts(started_at))
                .num_seconds(),
        ),
    }
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
