use chrono::{DateTime, Utc};
use tsr_collector::{
    models::{CaptureStatus, ScreenshotMeta, WindowSnapshot},
    storage::Store,
};

#[test]
fn persists_window_focus_events_in_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .insert_window_focus(
            &session_id,
            &WindowSnapshot {
                captured_at: ts("2026-05-23T09:00:00Z"),
                hwnd: 100,
                pid: 42,
                process_name: "Code.exe".to_string(),
                exe_path_hash: Some("abc123".to_string()),
                window_title: Some("main.rs".to_string()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();

    let rows = store.list_window_events(10).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, session_id);
    assert_eq!(rows[0].process_name, "Code.exe");
    assert_eq!(rows[0].window_title.as_deref(), Some("main.rs"));
    assert_eq!(rows[0].capture_status, CaptureStatus::Ok);
}

#[test]
fn lists_latest_window_focus_events_in_chronological_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for minute in 0..3 {
        store
            .insert_window_focus(
                &session_id,
                &WindowSnapshot {
                    captured_at: ts(&format!("2026-05-23T09:0{minute}:00Z")),
                    hwnd: minute + 100,
                    pid: minute as u32 + 42,
                    process_name: format!("App-{minute}"),
                    exe_path_hash: None,
                    window_title: Some(format!("Title {minute}")),
                    capture_status: CaptureStatus::Ok,
                },
            )
            .unwrap();
    }

    let rows = store.list_window_events(2).unwrap();

    assert_eq!(
        rows.iter()
            .map(|row| row.process_name.as_str())
            .collect::<Vec<_>>(),
        vec!["App-1", "App-2"]
    );
}

#[test]
fn persists_screenshot_metadata_for_session() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .insert_screenshot(
            &session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: ts("2026-05-23T09:01:00Z"),
                file_path: "2026-05-23/09-01.jpg".into(),
                width: 640,
                height: 360,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

    let rows = store.list_screenshots_by_date("2026-05-23", 10).unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].file_path, "2026-05-23/09-01.jpg");
    assert_eq!(rows[0].process_name.as_deref(), Some("Code.exe"));
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
