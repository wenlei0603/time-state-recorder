use chrono::{DateTime, Utc};
use rusqlite::params;
use tsr_collector::{
    interval::build_time_events_with_lifecycle,
    models::{
        ActivityCategory, CaptureStatus, HighResScreenshotMeta, LifecycleType, ScreenshotMeta,
        VisualSummary, WindowSnapshot,
    },
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

#[test]
fn persists_high_res_screenshot_metadata_by_date_without_skip_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, capture_status) in [
        ("2026-05-23T09:05:00Z", "2026-05-23/09-05.jpg", "ok"),
        ("2026-05-23T09:10:00Z", "", "idle"),
    ] {
        store
            .insert_high_res_screenshot(
                &session_id,
                &HighResScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 1920 } else { 0 },
                    height: if capture_status == "ok" { 1080 } else { 0 },
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
    }

    let rows = store
        .list_high_res_screenshots_by_date("2026-05-23", 10)
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].file_path, "2026-05-23/09-05.jpg");
    assert_eq!(rows[0].width, 1920);
    assert_eq!(rows[0].height, 1080);
    assert_eq!(rows[0].capture_status, "ok");
}

#[test]
fn visual_summaries_round_trip_by_date() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    let screenshot_id = store
        .insert_screenshot(
            &session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: ts("2026-05-23T09:01:00Z"),
                file_path: "2026-05-23/09-01.jpg".into(),
                width: 1280,
                height: 720,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

    let inserted = store
        .insert_visual_summary(&VisualSummary {
            id: 0,
            screenshot_id,
            captured_at: ts("2026-05-23T09:01:00Z"),
            model_provider: "local_stub".into(),
            model_name: "metadata-v1".into(),
            prompt_version: "visual-summary-v1".into(),
            summary_text: "Code editor focused on main.rs".into(),
            activity_category: ActivityCategory::Coding,
            project_hints: vec!["Time State Recorder".into()],
            visible_apps: vec!["Code.exe".into()],
            visible_text_hints: vec!["main.rs".into()],
            risk_flags: vec![],
            confidence: 0.65,
            created_at: ts("2026-05-23T09:02:00Z"),
            error: None,
        })
        .unwrap();

    let rows = store
        .list_visual_summaries_by_date("2026-05-23", 10)
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, inserted);
    assert_eq!(rows[0].screenshot_id, screenshot_id);
    assert_eq!(rows[0].model_provider, "local_stub");
    assert_eq!(rows[0].activity_category, ActivityCategory::Coding);
    assert_eq!(rows[0].project_hints, vec!["Time State Recorder"]);
    assert_eq!(rows[0].visible_apps, vec!["Code.exe"]);
    assert_eq!(rows[0].visible_text_hints, vec!["main.rs"]);
    assert_eq!(rows[0].summary_text, "Code editor focused on main.rs");
}

#[test]
fn screenshot_queries_support_utc_day_windows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, capture_status) in [
        ("2026-06-03T15:59:00Z", "outside-before.jpg", "ok"),
        ("2026-06-03T16:00:00Z", "inside-start.jpg", "ok"),
        ("2026-06-04T15:59:00Z", "inside-end.jpg", "ok"),
        ("2026-06-04T16:00:00Z", "outside-after.jpg", "ok"),
        ("2026-06-04T10:00:00Z", "", "idle"),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 640 } else { 0 },
                    height: if capture_status == "ok" { 360 } else { 0 },
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
        store
            .insert_high_res_screenshot(
                &session_id,
                &HighResScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 1440 } else { 0 },
                    height: if capture_status == "ok" { 900 } else { 0 },
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
    }

    let start = ts("2026-06-03T16:00:00Z");
    let end = ts("2026-06-04T16:00:00Z");

    let screenshots = store.list_screenshots_between(start, end, 10).unwrap();
    assert_eq!(
        screenshots
            .iter()
            .map(|item| item.file_path.as_str())
            .collect::<Vec<_>>(),
        vec!["inside-start.jpg", "inside-end.jpg"]
    );

    let high_res = store
        .list_high_res_screenshots_between(start, end, 10)
        .unwrap();
    assert_eq!(high_res.len(), 2);
    assert_eq!(high_res[0].file_path, "inside-start.jpg");

    let summary = store
        .get_screenshot_summary_between("2026-06-04", start, end)
        .unwrap();
    assert_eq!(summary.date, "2026-06-04");
    assert_eq!(summary.total_screenshots, 2);
    assert_eq!(summary.skipped_reasons.len(), 1);
    assert_eq!(summary.skipped_reasons[0].reason, "idle");
}

#[test]
fn visual_summary_queries_support_utc_day_windows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    let mut screenshot_ids = Vec::new();
    for (captured_at, file_path) in [
        ("2026-06-03T15:59:00Z", "outside-before.jpg"),
        ("2026-06-03T16:00:00Z", "inside-start.jpg"),
        ("2026-06-04T15:59:00Z", "inside-end.jpg"),
        ("2026-06-04T16:00:00Z", "outside-after.jpg"),
    ] {
        let screenshot_id = store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: 1280,
                    height: 720,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: "ok".into(),
                },
            )
            .unwrap();
        screenshot_ids.push((screenshot_id, captured_at));
    }

    for (screenshot_id, captured_at) in screenshot_ids {
        store
            .insert_visual_summary(&VisualSummary {
                id: 0,
                screenshot_id,
                captured_at: ts(captured_at),
                model_provider: "local_stub".into(),
                model_name: "metadata-v1".into(),
                prompt_version: "visual-summary-v1".into(),
                summary_text: format!("Summary at {captured_at}"),
                activity_category: ActivityCategory::Coding,
                project_hints: vec!["Time State Recorder".into()],
                visible_apps: vec!["Code.exe".into()],
                visible_text_hints: vec!["main.rs".into()],
                risk_flags: vec![],
                confidence: 0.65,
                created_at: ts(captured_at),
                error: None,
            })
            .unwrap();
    }

    let rows = store
        .list_visual_summaries_between(ts("2026-06-03T16:00:00Z"), ts("2026-06-04T16:00:00Z"), 10)
        .unwrap();

    assert_eq!(rows.len(), 2);
    assert!(rows[0].summary_text.contains("2026-06-03T16:00:00Z"));
    assert!(rows[1].summary_text.contains("2026-06-04T15:59:00Z"));
}

#[test]
fn persists_lifecycle_events_in_order() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:05:00Z"),
            LifecycleType::WindowsLock,
            Some("manual_lock"),
            serde_json::json!({"source": "test"}),
        )
        .unwrap();
    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:20:00Z"),
            LifecycleType::WindowsUnlock,
            None,
            serde_json::json!({}),
        )
        .unwrap();

    let rows = store.list_lifecycle_events(10).unwrap();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].session_id, session_id);
    assert_eq!(rows[0].lifecycle_type, LifecycleType::WindowsLock);
    assert_eq!(rows[0].reason.as_deref(), Some("manual_lock"));
    assert_eq!(rows[0].payload["source"], "test");
    assert_eq!(rows[1].lifecycle_type, LifecycleType::WindowsUnlock);
}

#[test]
fn closes_stale_sessions_as_abnormal_stop_with_collector_gap() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let stale_session_id = store.create_session("0.1.0", "test-config").unwrap();

    let closed = store
        .close_stale_sessions(ts("2026-05-23T10:00:00Z"), "abnormal_stop")
        .unwrap();

    assert_eq!(closed, vec![stale_session_id.clone()]);
    let rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].session_id, stale_session_id);
    assert_eq!(rows[0].lifecycle_type, LifecycleType::CollectorGap);
    assert_eq!(rows[0].reason.as_deref(), Some("abnormal_stop"));
}

#[test]
fn closes_stale_sessions_at_last_recorded_event_boundary() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let stale_session_id = store.create_session("0.1.0", "test-config").unwrap();
    store
        .insert_window_focus(
            &stale_session_id,
            &WindowSnapshot {
                captured_at: ts("2026-05-23T18:00:00Z"),
                hwnd: 100,
                pid: 42,
                process_name: "Code.exe".to_string(),
                exe_path_hash: None,
                window_title: Some("main.rs".to_string()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();

    store
        .close_stale_sessions(ts("2026-05-24T09:00:00Z"), "abnormal_stop")
        .unwrap();

    let lifecycle_rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(lifecycle_rows[0].event_ts, ts("2026-05-23T18:00:00Z"));

    let window_rows = store.list_window_events(10).unwrap();
    let intervals = build_time_events_with_lifecycle(&window_rows, &lifecycle_rows);
    assert_eq!(intervals[0].duration_seconds, Some(0));
}

#[test]
fn close_session_records_one_terminal_transition() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .close_session(&session_id, ts("2026-05-23T10:00:00Z"), "completed")
        .unwrap();
    assert!(
        store
            .close_session(&session_id, ts("2026-05-23T10:01:00Z"), "completed")
            .is_err()
    );

    let lifecycle_rows = store.list_lifecycle_events(10).unwrap();
    assert_eq!(lifecycle_rows.len(), 1);
    assert_eq!(lifecycle_rows[0].lifecycle_type, LifecycleType::SessionStop);
}

#[test]
fn rejects_session_scoped_writes_after_session_close() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    store
        .close_session(&session_id, ts("2026-05-23T10:00:00Z"), "completed")
        .unwrap();

    assert!(
        store
            .insert_window_focus(
                &session_id,
                &WindowSnapshot {
                    captured_at: ts("2026-05-23T10:00:01Z"),
                    hwnd: 100,
                    pid: 42,
                    process_name: "Code.exe".to_string(),
                    exe_path_hash: None,
                    window_title: Some("late.rs".to_string()),
                    capture_status: CaptureStatus::Ok,
                },
            )
            .is_err()
    );

    assert!(
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts("2026-05-23T10:00:02Z"),
                    file_path: "2026-05-23/10-00.jpg".into(),
                    width: 640,
                    height: 360,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("late.rs".into()),
                    capture_status: "ok".into(),
                },
            )
            .is_err()
    );

    assert!(
        store
            .insert_lifecycle_event(
                &session_id,
                ts("2026-05-23T10:00:03Z"),
                LifecycleType::WindowsLock,
                None,
                serde_json::json!({}),
            )
            .is_err()
    );
}

#[test]
fn rejects_unknown_lifecycle_types_from_storage() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("unknown-lifecycle.sqlite3");
    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    drop(store);

    let conn = rusqlite::Connection::open(&db_path).unwrap();
    conn.execute(
        r#"
        INSERT INTO raw_events
          (session_id, event_ts, event_type, source, target_window_id, payload_json)
        VALUES (?1, ?2, 'lifecycle', 'test', NULL, '{}')
        "#,
        params![session_id, "2026-05-23T09:00:00Z"],
    )
    .unwrap();
    let raw_event_id = conn.last_insert_rowid();
    conn.execute(
        r#"
        INSERT INTO lifecycle_events
          (raw_event_id, lifecycle_type, reason, active_session_id, payload_json)
        VALUES (?1, 'future_shutdown', NULL, ?2, '{}')
        "#,
        params![raw_event_id, session_id],
    )
    .unwrap();
    drop(conn);

    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();

    assert!(store.list_lifecycle_events(10).is_err());
}

#[test]
fn screenshot_summary_counts_skipped_reasons() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (minute, status) in [
        ("00", "ok"),
        ("01", "idle"),
        ("02", "blocked"),
        ("03", "capture_unavailable"),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(&format!("2026-05-24T09:{minute}:00Z")),
                    file_path: format!("2026-05-24/09-{minute}.jpg"),
                    width: 640,
                    height: 360,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: status.into(),
                },
            )
            .unwrap();
    }

    let summary = store.get_screenshot_summary("2026-05-24").unwrap();

    assert_eq!(summary.total_screenshots, 1);
    assert_eq!(summary.skipped_reasons.len(), 3);
    assert_eq!(summary.skipped_reasons[0].reason, "blocked");
    assert_eq!(summary.skipped_reasons[0].count, 1);
    assert_eq!(summary.skipped_reasons[1].reason, "capture_unavailable");
    assert_eq!(summary.skipped_reasons[2].reason, "idle");
}

#[test]
fn screenshot_reads_and_success_summary_ignore_skip_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, process_name, window_title, capture_status) in [
        (
            "2026-05-24T09:00:00Z",
            "2026-05-24/09-00.jpg",
            Some("Code.exe"),
            Some("main.rs"),
            "ok",
        ),
        ("2026-05-24T10:00:00Z", "", None, None, "idle"),
        (
            "2026-05-24T11:00:00Z",
            "",
            Some("Secret.exe"),
            Some("Sensitive window"),
            "blocked",
        ),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: if capture_status == "ok" { 640 } else { 0 },
                    height: if capture_status == "ok" { 360 } else { 0 },
                    process_name: process_name.map(String::from),
                    window_title: window_title.map(String::from),
                    capture_status: capture_status.into(),
                },
            )
            .unwrap();
    }

    let screenshots = store.list_screenshots_by_date("2026-05-24", 10).unwrap();
    assert_eq!(screenshots.len(), 1);
    assert_eq!(screenshots[0].capture_status, "ok");

    let summary = store.get_screenshot_summary("2026-05-24").unwrap();
    assert_eq!(summary.total_screenshots, 1);
    assert_eq!(summary.hours_covered, 1);
    assert_eq!(summary.top_apps.len(), 1);
    assert_eq!(summary.top_apps[0].process_name, "Code.exe");
    assert_eq!(summary.skipped_reasons.len(), 2);
    assert_eq!(summary.skipped_reasons[0].reason, "blocked");
    assert_eq!(summary.skipped_reasons[0].count, 1);
    assert_eq!(summary.skipped_reasons[1].reason, "idle");
    assert_eq!(summary.skipped_reasons[1].count, 1);

    let stats = store.get_db_stats().unwrap();
    assert_eq!(stats.screenshots, 1);
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}
