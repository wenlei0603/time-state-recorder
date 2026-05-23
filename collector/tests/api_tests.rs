use std::net::SocketAddr;

use axum::http::StatusCode;
use chrono::{DateTime, Utc};
use tsr_collector::{
    api,
    models::{CaptureStatus, WindowSnapshot},
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
    assert_eq!(body["version"], "0.1.0");
    assert!(body["windowCollector"]["status"].as_str().is_some());
    assert!(body["dbStats"]["windowEvents"].as_u64().is_some());

    server.abort();
}
