use chrono::{DateTime, Utc};
use tsr_collector::{
    interval::build_time_events,
    models::{CaptureStatus, StoredWindowEvent},
};

#[test]
fn builds_intervals_from_ordered_window_focus_events() {
    let events = vec![
        event(1, "2026-05-23T09:00:00Z", "Code", "main.rs"),
        event(2, "2026-05-23T09:10:00Z", "Browser", "Docs"),
        event(3, "2026-05-23T09:25:00Z", "Code", "README.md"),
    ];

    let intervals = build_time_events(&events);

    assert_eq!(intervals.len(), 3);
    assert_eq!(intervals[0].id, "raw-1");
    assert_eq!(intervals[0].app, "Code");
    assert_eq!(intervals[0].title, "main.rs");
    assert_eq!(intervals[0].duration_seconds, Some(600));
    assert_eq!(intervals[1].duration_seconds, Some(900));
    assert_eq!(intervals[2].duration_seconds, None);
}

fn event(id: i64, ts: &str, app: &str, title: &str) -> StoredWindowEvent {
    StoredWindowEvent {
        raw_event_id: id,
        session_id: "session-1".to_string(),
        event_ts: DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&Utc),
        hwnd: id,
        pid: id as u32,
        process_name: app.to_string(),
        exe_path_hash: Some(format!("hash-{id}")),
        window_title: Some(title.to_string()),
        capture_status: CaptureStatus::Ok,
    }
}
