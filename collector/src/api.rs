use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use anyhow::Result;
use axum::{
    Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::time;

use crate::{
    interval::build_time_events, models::TimeEvent, storage::Store,
    window::sample_foreground_window,
};

#[derive(Clone)]
pub struct AppState {
    store: Arc<Mutex<Store>>,
}

#[derive(Debug, Deserialize)]
struct LimitQuery {
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeEventsResponse {
    events: Vec<TimeEvent>,
}

pub fn router(store: Store) -> Router {
    router_from_state(AppState {
        store: Arc::new(Mutex::new(store)),
    })
}

fn router_from_state(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/window-events", get(window_events))
        .route("/api/time-events", get(time_events))
        .with_state(state)
}

pub async fn serve(store: Store, addr: SocketAddr, poll_ms: u64) -> Result<()> {
    anyhow::ensure!(poll_ms >= 100, "poll_ms must be at least 100");
    let session_id = store.create_session(env!("CARGO_PKG_VERSION"), "default")?;
    let state = AppState {
        store: Arc::new(Mutex::new(store)),
    };
    spawn_collector_loop(state.clone(), session_id, poll_ms);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router_from_state(state)).await?;
    Ok(())
}

fn spawn_collector_loop(state: AppState, session_id: String, poll_ms: u64) {
    tokio::spawn(async move {
        let mut last_identity: Option<(i64, u32, Option<String>)> = None;
        loop {
            match sample_foreground_window() {
                Ok(snapshot) => {
                    let identity = (snapshot.hwnd, snapshot.pid, snapshot.window_title.clone());
                    if last_identity.as_ref() != Some(&identity) {
                        if let Ok(mut store) = state.store.lock() {
                            match store.insert_window_focus(&session_id, &snapshot) {
                                Ok(_) => {
                                    last_identity = Some(identity);
                                }
                                Err(err) => {
                                    eprintln!("window event write failed: {err:#}");
                                }
                            }
                        } else {
                            eprintln!("window event write failed: store lock poisoned");
                        }
                    }
                }
                Err(err) => {
                    eprintln!("window sample failed: {err:#}");
                }
            }

            time::sleep(Duration::from_millis(poll_ms)).await;
        }
    });
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn window_events(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_window_events(limit) {
        Ok(events) => Json(events).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn time_events(
    State(state): State<AppState>,
    Query(query): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_window_events(limit) {
        Ok(window_events) => Json(TimeEventsResponse {
            events: build_time_events(&window_events),
        })
        .into_response(),
        Err(err) => internal_error(err),
    }
}

fn internal_error(message: impl std::fmt::Display) -> axum::response::Response {
    (StatusCode::INTERNAL_SERVER_ERROR, message.to_string()).into_response()
}
