use std::{net::SocketAddr, time::Duration};

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use tsr_collector::{
    api,
    models::{
        ActivityCategory, CaptureStatus, HighResScreenshotMeta, LifecycleType, ScreenshotMeta,
        VisualSummary, WindowSnapshot,
    },
    storage::Store,
};

#[tokio::test]
async fn serves_time_events_as_json() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:00:00Z",
        100,
        "Code",
        "main.rs",
    );
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:10:00Z",
        200,
        "Browser",
        "Docs",
    );

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/time-events"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["events"][0]["app"], "Code");
    assert_eq!(body["events"][0]["durationSeconds"], 600);

    server.abort();
}

#[tokio::test]
async fn serves_lifecycle_events_as_json() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:05:00Z"),
            LifecycleType::WindowsLock,
            Some("manual_lock"),
            serde_json::json!({}),
        )
        .unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/lifecycle-events"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["events"][0]["lifecycleType"], "windows_lock");
    assert_eq!(body["events"][0]["reason"], "manual_lock");
    assert_eq!(body["events"][0]["sessionId"], session_id);

    server.abort();
}

#[tokio::test]
async fn serves_time_events_with_lifecycle_boundaries() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:00:00Z",
        100,
        "Code",
        "main.rs",
    );
    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:05:00Z"),
            LifecycleType::WindowsLock,
            None,
            serde_json::json!({}),
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
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:25:00Z",
        200,
        "Browser",
        "Docs",
    );

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/time-events"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["events"][0]["id"], "raw-1");
    assert_eq!(body["events"][0]["kind"], "active_window");
    assert_eq!(body["events"][0]["durationSeconds"], 300);
    assert_eq!(body["events"][1]["id"], "lifecycle-2");
    assert_eq!(body["events"][1]["kind"], "lifecycle");
    assert_eq!(body["events"][1]["status"], "windows_lock");
    assert_eq!(body["events"][1]["durationSeconds"], 900);

    server.abort();
}

#[tokio::test]
async fn time_events_limit_applies_after_merging_lifecycle_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:00:00Z",
        100,
        "Code",
        "main.rs",
    );
    store
        .insert_lifecycle_event(
            &session_id,
            ts("2026-05-23T09:05:00Z"),
            LifecycleType::WindowsLock,
            None,
            serde_json::json!({}),
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
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:25:00Z",
        200,
        "Browser",
        "Docs",
    );

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/time-events?limit=2"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["events"].as_array().unwrap().len(), 2);

    server.abort();
}

#[tokio::test]
async fn serves_activity_buckets_for_date() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:00:00Z",
        100,
        "Code",
        "main.rs",
    );
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:02:00Z",
        200,
        "chrome.exe",
        "GitHub - Pull Request - Google Chrome",
    );
    insert(
        &mut store,
        &session_id,
        "2026-05-23T09:04:00Z",
        300,
        "Code",
        "lib.rs",
    );

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!(
        "http://{addr}/api/activity-buckets?date=2026-05-23&bucketSeconds=180"
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["date"], "2026-05-23");
    assert_eq!(body["bucketSeconds"], 180);
    let buckets = body["buckets"].as_array().unwrap();
    assert!(buckets.len() >= 2);
    assert_eq!(buckets[0]["dominantApp"], "Code");
    assert_eq!(buckets[0]["normalizedTitle"], "main.rs");
    assert_eq!(buckets[0]["bucketSeconds"], 180);

    server.abort();
}

#[tokio::test]
async fn rejects_invalid_activity_bucket_date() {
    let store = Store::open_memory().unwrap();
    store.init().unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!(
        "http://{addr}/api/activity-buckets?date=not-a-date"
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    server.abort();
}

#[tokio::test]
async fn serve_bind_failure_does_not_close_open_sessions() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("bind-failure.sqlite3");
    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    let occupied_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let occupied_addr = occupied_listener.local_addr().unwrap();

    let result = api::serve(store, occupied_addr, 100, None).await;

    assert!(result.is_err());
    drop(occupied_listener);

    let mut store = Store::open(&db_path).unwrap();
    store.init().unwrap();
    assert!(store.list_lifecycle_events(10).unwrap().is_empty());
    assert_eq!(
        store
            .close_stale_sessions(ts("2026-05-23T10:00:00Z"), "abnormal_stop")
            .unwrap(),
        vec![session_id]
    );
}

#[tokio::test]
async fn serve_shutdown_endpoint_records_service_stop() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("shutdown.sqlite3");
    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();

    let port_probe = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = port_probe.local_addr().unwrap();
    drop(port_probe);

    let server = tokio::spawn(async move { api::serve(store, addr, 100, None).await });
    wait_for_health(addr).await;

    let response = reqwest::Client::new()
        .post(format!("http://{addr}/api/shutdown"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .unwrap()
        .unwrap()
        .unwrap();

    let store = Store::open(&db_path).unwrap();
    store.init().unwrap();
    let lifecycle_rows = store.list_lifecycle_events(10).unwrap();
    assert!(
        lifecycle_rows
            .iter()
            .any(|row| row.lifecycle_type == LifecycleType::SessionStop
                && row.reason.as_deref() == Some("service_stop"))
    );
}

#[tokio::test]
async fn does_not_allow_cross_origin_reads() {
    let store = Store::open_memory().unwrap();
    store.init().unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::Client::new()
        .get(format!("http://{addr}/api/time-events"))
        .header("Origin", "https://example.test")
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response
            .headers()
            .get("access-control-allow-origin")
            .is_none()
    );

    server.abort();
}

#[tokio::test]
async fn serves_input_events_as_json() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let _session_id = store.create_session("0.1.0", "test-config").unwrap();

    let segment = tsr_collector::models::TextSegment {
        id: "seg-test-1".into(),
        started_at: DateTime::parse_from_rfc3339("2026-05-23T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ended_at: Some(
            DateTime::parse_from_rfc3339("2026-05-23T09:00:05Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
        text_content: "fn main() {}\n".into(),
        key_count: 14,
        backspace_count: 1,
        delete_count: 0,
        foreground_hwnd: 1111,
        foreground_pid: 100,
        process_name: Some("Code".into()),
        window_title: Some("main.rs".into()),
    };
    let events = vec![
        tsr_collector::models::InputEvent {
            id: 0,
            event_ts: DateTime::parse_from_rfc3339("2026-05-23T09:00:01Z")
                .unwrap()
                .with_timezone(&Utc),
            event_type: tsr_collector::models::InputEventType::KeyDown,
            vk_code: 70,
            scan_code: 33,
            character: Some("f".into()),
            segment_id: "seg-test-1".into(),
            foreground_hwnd: 1111,
            foreground_pid: 100,
            process_name: Some("Code".into()),
            window_title: Some("main.rs".into()),
        },
        tsr_collector::models::InputEvent {
            id: 0,
            event_ts: DateTime::parse_from_rfc3339("2026-05-23T09:00:02Z")
                .unwrap()
                .with_timezone(&Utc),
            event_type: tsr_collector::models::InputEventType::KeyUp,
            vk_code: 70,
            scan_code: 33,
            character: None,
            segment_id: "seg-test-1".into(),
            foreground_hwnd: 1111,
            foreground_pid: 100,
            process_name: Some("Code".into()),
            window_title: Some("main.rs".into()),
        },
    ];
    store.insert_input_segment(&segment, &events).unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/input-events"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["events"][0]["eventType"], "keydown");
    assert_eq!(body["events"][0]["character"], "f");
    assert_eq!(body["events"][0]["vkCode"], 70);
    assert_eq!(body["events"][0]["segmentId"], "seg-test-1");
    assert_eq!(body["events"][1]["eventType"], "keyup");

    server.abort();
}

#[tokio::test]
async fn serves_input_summary_as_json() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let _session_id = store.create_session("0.1.0", "test-config").unwrap();

    let segment = tsr_collector::models::TextSegment {
        id: "seg-sum-1".into(),
        started_at: DateTime::parse_from_rfc3339("2026-05-23T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ended_at: Some(
            DateTime::parse_from_rfc3339("2026-05-23T09:00:05Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
        text_content: "hello\n".into(),
        key_count: 6,
        backspace_count: 0,
        delete_count: 0,
        foreground_hwnd: 1111,
        foreground_pid: 100,
        process_name: Some("Code".into()),
        window_title: Some("main.rs".into()),
    };
    let events = vec![tsr_collector::models::InputEvent {
        id: 0,
        event_ts: DateTime::parse_from_rfc3339("2026-05-23T09:00:01Z")
            .unwrap()
            .with_timezone(&Utc),
        event_type: tsr_collector::models::InputEventType::KeyDown,
        vk_code: 72,
        scan_code: 35,
        character: Some("h".into()),
        segment_id: "seg-sum-1".into(),
        foreground_hwnd: 1111,
        foreground_pid: 100,
        process_name: Some("Code".into()),
        window_title: Some("main.rs".into()),
    }];
    store.insert_input_segment(&segment, &events).unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/input-summary?date=2026-05-23"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["date"], "2026-05-23");
    assert_eq!(body["totalEvents"], 1);
    assert_eq!(body["keydownCount"], 1);
    assert_eq!(body["keyupCount"], 0);
    assert_eq!(body["segmentCount"], 1);
    assert_eq!(body["totalChars"], 6);
    assert_eq!(body["topApps"][0]["processName"], "Code");
    assert_eq!(body["topApps"][0]["charCount"], 6);

    server.abort();
}

#[tokio::test]
async fn serves_text_segments_as_json() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let _session_id = store.create_session("0.1.0", "test-config").unwrap();

    let segment = tsr_collector::models::TextSegment {
        id: "seg-text-1".into(),
        started_at: DateTime::parse_from_rfc3339("2026-05-23T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc),
        ended_at: None,
        text_content: "cargo run\n".into(),
        key_count: 10,
        backspace_count: 0,
        delete_count: 0,
        foreground_hwnd: 2222,
        foreground_pid: 200,
        process_name: Some("WindowsTerminal".into()),
        window_title: Some("PowerShell".into()),
    };
    store.insert_input_segment(&segment, &[]).unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/text-segments?date=2026-05-23"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["segments"][0]["id"], "seg-text-1");
    assert_eq!(body["segments"][0]["textContent"], "cargo run\n");
    assert_eq!(body["segments"][0]["keyCount"], 10);
    assert_eq!(body["segments"][0]["processName"], "WindowsTerminal");
    assert_eq!(body["segments"][0]["endedAt"], serde_json::Value::Null);

    server.abort();
}

#[tokio::test]
async fn serves_health_with_collector_stability_details() {
    let store = Store::open_memory().unwrap();
    store.init().unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/health"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["windowCollector"]["mode"], "polling");
    assert_eq!(
        body["windowCollector"]["lastCaptureStatus"],
        serde_json::Value::Null
    );
    assert_eq!(
        body["screenshotCollector"]["lastSkipReason"],
        serde_json::Value::Null
    );

    server.abort();
}

#[tokio::test]
async fn serves_screenshots_without_skip_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, capture_status) in [
        ("2026-05-24T09:00:00Z", "2026-05-24/09-00.jpg", "ok"),
        ("2026-05-24T10:00:00Z", "", "idle"),
        ("2026-05-24T11:00:00Z", "", "blocked"),
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
    }

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/screenshots?date=2026-05-24"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let screenshots = body["screenshots"].as_array().unwrap();
    assert_eq!(screenshots.len(), 1);
    assert_eq!(screenshots[0]["captureStatus"], "ok");
    assert_eq!(screenshots[0]["filePath"], "2026-05-24/09-00.jpg");

    server.abort();
}

#[tokio::test]
async fn serves_screenshots_for_browser_local_date_window() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path) in [
        ("2026-06-03T15:59:00Z", "outside-before.jpg"),
        ("2026-06-03T16:00:00Z", "inside-start.jpg"),
        ("2026-06-04T15:59:00Z", "inside-end.jpg"),
        ("2026-06-04T16:00:00Z", "outside-after.jpg"),
    ] {
        store
            .insert_screenshot(
                &session_id,
                &ScreenshotMeta {
                    id: 0,
                    captured_at: ts(captured_at),
                    file_path: file_path.into(),
                    width: 640,
                    height: 360,
                    process_name: Some("Code.exe".into()),
                    window_title: Some("main.rs".into()),
                    capture_status: "ok".into(),
                },
            )
            .unwrap();
    }

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!(
        "http://{addr}/api/screenshots?date=2026-06-04&tzOffsetMinutes=-480"
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let screenshots = body["screenshots"].as_array().unwrap();
    assert_eq!(screenshots.len(), 2);
    assert_eq!(screenshots[0]["filePath"], "inside-start.jpg");
    assert_eq!(screenshots[1]["filePath"], "inside-end.jpg");

    server.abort();
}

#[tokio::test]
async fn serves_visual_summaries_for_date() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    let screenshot_id = store
        .insert_screenshot(
            &session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: ts("2026-05-24T10:00:00Z"),
                file_path: "2026-05-24/10-00.jpg".into(),
                width: 1280,
                height: 720,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();
    store
        .insert_visual_summary(&VisualSummary {
            id: 0,
            screenshot_id,
            captured_at: ts("2026-05-24T10:00:00Z"),
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
            created_at: ts("2026-05-24T10:01:00Z"),
            error: None,
        })
        .unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!(
        "http://{addr}/api/visual-summaries?date=2026-05-24"
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["summaries"][0]["screenshotId"], screenshot_id);
    assert_eq!(body["summaries"][0]["modelProvider"], "local_stub");
    assert_eq!(body["summaries"][0]["activityCategory"], "coding");
    assert_eq!(body["summaries"][0]["visibleApps"][0], "Code.exe");

    server.abort();
}

#[tokio::test]
async fn serves_high_res_screenshots_without_skip_rows() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();

    for (captured_at, file_path, capture_status) in [
        ("2026-05-24T10:05:00Z", "2026-05-24/10-05.jpg", "ok"),
        ("2026-05-24T10:10:00Z", "", "idle"),
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

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!(
        "http://{addr}/api/high-res-screenshots?date=2026-05-24"
    ))
    .await
    .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    let screenshots = body["screenshots"].as_array().unwrap();
    assert_eq!(screenshots.len(), 1);
    assert_eq!(screenshots[0]["filePath"], "2026-05-24/10-05.jpg");
    assert_eq!(screenshots[0]["width"], 1920);
    assert_eq!(screenshots[0]["height"], 1080);
    assert_eq!(screenshots[0]["captureStatus"], "ok");

    server.abort();
}

#[tokio::test]
async fn analyzes_screenshot_with_local_stub() {
    let mut store = Store::open_memory().unwrap();
    store.init().unwrap();
    let session_id = store.create_session("0.1.0", "test-config").unwrap();
    let screenshot_id = store
        .insert_screenshot(
            &session_id,
            &ScreenshotMeta {
                id: 0,
                captured_at: ts("2026-05-24T10:00:00Z"),
                file_path: "2026-05-24/10-00.jpg".into(),
                width: 1280,
                height: 720,
                process_name: Some("Code.exe".into()),
                window_title: Some("main.rs".into()),
                capture_status: "ok".into(),
            },
        )
        .unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::Client::new()
        .post(format!(
            "http://{addr}/api/screenshots/{screenshot_id}/analyze"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["summary"]["screenshotId"], screenshot_id);
    assert_eq!(body["summary"]["modelProvider"], "local_stub");
    assert_eq!(body["summary"]["modelName"], "metadata-v1");
    assert_eq!(body["summary"]["activityCategory"], "coding");
    assert!(
        body["summary"]["summaryText"]
            .as_str()
            .unwrap()
            .contains("Code.exe")
    );

    server.abort();
}

fn insert(store: &mut Store, session_id: &str, ts: &str, hwnd: i64, app: &str, title: &str) {
    store
        .insert_window_focus(
            session_id,
            &WindowSnapshot {
                captured_at: DateTime::parse_from_rfc3339(ts)
                    .unwrap()
                    .with_timezone(&Utc),
                hwnd,
                pid: hwnd as u32,
                process_name: app.to_string(),
                exe_path_hash: Some(format!("hash-{hwnd}")),
                window_title: Some(title.to_string()),
                capture_status: CaptureStatus::Ok,
            },
        )
        .unwrap();
}

fn ts(value: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(value)
        .unwrap()
        .with_timezone(&Utc)
}

async fn wait_for_health(addr: SocketAddr) {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while tokio::time::Instant::now() < deadline {
        if let Ok(response) = client.get(format!("http://{addr}/api/health")).send().await {
            if response.status() == StatusCode::OK {
                return;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("collector health endpoint did not become ready");
}

#[tokio::test]
async fn serves_health_with_subsystem_status() {
    let store = Store::open_memory().unwrap();
    store.init().unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/health"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["status"].as_str().is_some());
    assert!(body["uptimeSeconds"].as_u64().is_some());
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["windowCollector"]["status"].as_str().is_some());
    assert!(body["dbStats"]["windowEvents"].as_u64().is_some());

    server.abort();
}
