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

    let app = api::router(store);
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

    let app = api::router(store);
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
