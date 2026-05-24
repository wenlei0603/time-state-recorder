use chrono::{DateTime, Utc};
use tsr_collector::{
    interval::{build_time_events, build_time_events_with_lifecycle},
    models::{CaptureStatus, LifecycleEvent, LifecycleType, StoredWindowEvent, TimeEventKind},
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

#[test]
fn cuts_active_window_interval_at_lock_and_adds_locked_interval() {
    let window_events = vec![
        event_with_session(1, "session-1", "2026-05-23T09:00:00Z", "Code", "main.rs"),
        event_with_session(2, "session-1", "2026-05-23T09:25:00Z", "Browser", "Docs"),
    ];
    let lifecycle_events = vec![
        lifecycle(
            10,
            "session-1",
            "2026-05-23T09:05:00Z",
            LifecycleType::WindowsLock,
        ),
        lifecycle(
            11,
            "session-1",
            "2026-05-23T09:20:00Z",
            LifecycleType::WindowsUnlock,
        ),
    ];

    let intervals = build_time_events_with_lifecycle(&window_events, &lifecycle_events);

    assert_eq!(intervals.len(), 3);
    assert_eq!(intervals[0].id, "raw-1");
    assert_eq!(intervals[0].duration_seconds, Some(300));
    assert_eq!(intervals[1].id, "lifecycle-10");
    assert_eq!(intervals[1].kind, TimeEventKind::Lifecycle);
    assert_eq!(intervals[1].status.as_deref(), Some("windows_lock"));
    assert_eq!(intervals[1].duration_seconds, Some(900));
    assert_eq!(intervals[2].id, "raw-2");
}

#[test]
fn does_not_bridge_window_intervals_between_sessions() {
    let window_events = vec![
        event_with_session(1, "session-1", "2026-05-23T09:00:00Z", "Code", "main.rs"),
        event_with_session(2, "session-2", "2026-05-23T10:00:00Z", "Code", "README.md"),
    ];

    let intervals = build_time_events_with_lifecycle(&window_events, &[]);

    assert_eq!(intervals.len(), 2);
    assert_eq!(intervals[0].duration_seconds, None);
    assert_eq!(intervals[1].duration_seconds, None);
}

fn event(id: i64, ts: &str, app: &str, title: &str) -> StoredWindowEvent {
    event_with_session(id, "session-1", ts, app, title)
}

fn event_with_session(
    id: i64,
    session_id: &str,
    ts: &str,
    app: &str,
    title: &str,
) -> StoredWindowEvent {
    StoredWindowEvent {
        raw_event_id: id,
        session_id: session_id.to_string(),
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

fn lifecycle(id: i64, session_id: &str, ts: &str, lifecycle_type: LifecycleType) -> LifecycleEvent {
    LifecycleEvent {
        raw_event_id: id,
        session_id: session_id.to_string(),
        event_ts: DateTime::parse_from_rfc3339(ts)
            .unwrap()
            .with_timezone(&Utc),
        lifecycle_type,
        reason: None,
        active_session_id: Some(session_id.to_string()),
        payload: serde_json::json!({}),
    }
}
