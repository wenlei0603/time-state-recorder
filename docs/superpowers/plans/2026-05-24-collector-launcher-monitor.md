# Collector Launcher + Monitoring Dashboard — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add one-click launcher scripts and a real-time collector monitoring dashboard showing per-subsystem health and DB row counts.

**Architecture:** `Arc<Mutex<CollectorHealth>>` in `AppState` updated by each collector loop on success/error. Enhanced `/api/health` returns full subsystem status, uptime, and DB stats. WebUI polls every 5s and renders a monitoring panel replacing the old "Collector Connection" panel.

**Tech Stack:** Rust/Axum (models + shared health state + DB count queries), React/TypeScript (monitor component), Windows batch (launcher), npm scripts (launcher).

---

### Task 1: Launcher Scripts

**Files:**
- Create: `collector/start.bat`
- Modify: `package.json:6-10`

- [ ] **Step 1: Create `collector/start.bat`**

```batch
@echo off
if exist "target\release\tsr-collector.exe" (
    echo Starting collector (release)...
    target\release\tsr-collector.exe serve --db data/local.sqlite3 --addr 127.0.0.1:4317
) else if exist "target\debug\tsr-collector.exe" (
    echo Starting collector (debug)...
    target\debug\tsr-collector.exe serve --db data/local.sqlite3 --addr 127.0.0.1:4317
) else (
    echo Collector binary not found. Build first:
    echo   cargo build --release -p tsr-collector
    pause
)
```

- [ ] **Step 2: Add npm scripts to `package.json`**

Append two scripts to the `"scripts"` block:

```json
"collector": "cargo run -p tsr-collector -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317",
"collector:release": "cargo run -p tsr-collector --release -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317"
```

The existing scripts block starts:
```json
"dev": "vite --host 127.0.0.1",
"build": "tsc -b && vite build",
"test": "vitest run",
"test:watch": "vitest"
```

Change it to:
```json
"dev": "vite --host 127.0.0.1",
"build": "tsc -b && vite build",
"test": "vitest run",
"test:watch": "vitest",
"collector": "cargo run -p tsr-collector -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317",
"collector:release": "cargo run -p tsr-collector --release -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317"
```

- [ ] **Step 3: Commit**

```bash
git add collector/start.bat package.json
git commit -m "feat: add collector launcher scripts (start.bat + npm scripts)"
```

---

### Task 2: Rust Health Models

**Files:**
- Modify: `collector/src/models.rs:196-201` (append after `AppInputCount`)

- [ ] **Step 1: Append health structs to `collector/src/models.rs`**

Append after the `AppInputCount` struct (line 200):

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorHealth {
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub uptime_seconds: u64,
    pub version: String,
    pub window_collector: SubsystemHealth,
    pub input_collector: SubsystemHealth,
    pub screenshot_collector: SubsystemHealth,
    pub db_stats: DbStats,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubsystemHealth {
    pub status: String,
    pub last_event_at: Option<DateTime<Utc>>,
    pub error_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DbStats {
    pub window_events: usize,
    pub input_events: usize,
    pub text_segments: usize,
    pub screenshots: usize,
    pub blocker_hits: usize,
}
```

- [ ] **Step 2: Verify build compiles**

```bash
cargo check -p tsr-collector
```

- [ ] **Step 3: Commit**

```bash
git add collector/src/models.rs
git commit -m "feat: add CollectorHealth, SubsystemHealth, and DbStats models"
```

---

### Task 3: DB Row Count Methods in Storage

**Files:**
- Modify: `collector/src/storage.rs` (append new methods)

- [ ] **Step 1: Add `get_db_stats` method to `Store`**

Append after the `get_input_summary` method in `impl Store`. Find the closing `}` of `impl Store` and insert before it:

```rust
pub fn get_db_stats(&self) -> Result<crate::models::DbStats> {
    let window_events: usize = self
        .conn
        .query_row("SELECT COUNT(*) FROM window_events", [], |r| r.get(0))?;
    let input_events: usize = self
        .conn
        .query_row("SELECT COUNT(*) FROM input_events", [], |r| r.get(0))?;
    let text_segments: usize = self
        .conn
        .query_row("SELECT COUNT(*) FROM text_segments", [], |r| r.get(0))?;
    let screenshots: usize = self
        .conn
        .query_row("SELECT COUNT(*) FROM screenshot_thumbnails", [], |r| r.get(0))?;
    let blocker_hits: usize = self
        .conn
        .query_row("SELECT COUNT(*) FROM blocker_hits", [], |r| r.get(0))?;

    Ok(crate::models::DbStats {
        window_events,
        input_events,
        text_segments,
        screenshots,
        blocker_hits,
    })
}
```

- [ ] **Step 2: Verify build compiles**

```bash
cargo check -p tsr-collector
```

- [ ] **Step 3: Commit**

```bash
git add collector/src/storage.rs
git commit -m "feat: add get_db_stats with per-table row counts"
```

---

### Task 4: Health State + Loop Instrumentation

**Files:**
- Modify: `collector/src/api.rs:1-407` (multiple changes)
- Modify: `collector/src/input.rs:142-187` (signature + drain task)

- [ ] **Step 1: Add health field to `AppState` and update imports**

In `api.rs`, add `CollectorHealth`, `SubsystemHealth`, `DbStats` to the models import at line 25-27:

```rust
use crate::{
    blocker::BlockerEngine,
    input,
    interval::build_time_events,
    models::{
        BlockerHit, CollectorHealth, DbStats, ScreenshotMeta, ScreenshotSummary,
        SubsystemHealth, TimeEvent,
    },
    screenshot,
    storage::Store,
    window::sample_foreground_window,
};
```

Add `health: Arc<Mutex<CollectorHealth>>` to the `AppState` struct (after `idle_threshold_secs` at line 39):

```rust
pub struct AppState {
    store: Arc<Mutex<Store>>,
    blocker_engine: Arc<BlockerEngine>,
    screenshot_dir: Arc<PathBuf>,
    screenshot_interval_secs: Arc<u64>,
    idle_threshold_secs: Arc<u64>,
    health: Arc<Mutex<CollectorHealth>>,
}
```

- [ ] **Step 2: Initialize health in `default_state`**

In `default_state` (line 98), add the health initialization:

```rust
fn default_state(store: Store, blocker_config_path: Option<PathBuf>) -> AppState {
    let engine = blocker_config_path
        .as_deref()
        .and_then(|p| BlockerEngine::load(p).ok())
        .unwrap_or_else(BlockerEngine::empty);
    let now = Utc::now();
    AppState {
        store: Arc::new(Mutex::new(store)),
        blocker_engine: Arc::new(engine),
        screenshot_dir: Arc::new(PathBuf::from("data/screenshots")),
        screenshot_interval_secs: Arc::new(DEFAULT_SCREENSHOT_INTERVAL),
        idle_threshold_secs: Arc::new(DEFAULT_IDLE_THRESHOLD),
        health: Arc::new(Mutex::new(CollectorHealth {
            status: "ok".into(),
            started_at: now,
            uptime_seconds: 0,
            version: env!("CARGO_PKG_VERSION").into(),
            window_collector: SubsystemHealth {
                status: "not_started".into(),
                last_event_at: None,
                error_count: 0,
                last_error: None,
            },
            input_collector: SubsystemHealth {
                status: "not_started".into(),
                last_event_at: None,
                error_count: 0,
                last_error: None,
            },
            screenshot_collector: SubsystemHealth {
                status: "not_started".into(),
                last_event_at: None,
                error_count: 0,
                last_error: None,
            },
            db_stats: DbStats {
                window_events: 0,
                input_events: 0,
                text_segments: 0,
                screenshots: 0,
                blocker_hits: 0,
            },
        })),
    }
}
```

- [ ] **Step 3: Instrument window collector loop**

In `spawn_collector_loop`, add health updates:

```rust
fn spawn_collector_loop(state: AppState, session_id: String, poll_ms: u64) {
    tokio::spawn(async move {
        // Mark as running
        {
            let mut h = state.health.lock().unwrap();
            h.window_collector.status = "running".into();
        }

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
                                    if let Ok(mut h) = state.health.lock() {
                                        h.window_collector.last_event_at = Some(Utc::now());
                                        h.window_collector.error_count = 0;
                                        h.window_collector.last_error = None;
                                    }
                                }
                                Err(err) => {
                                    eprintln!("window event write failed: {err:#}");
                                    if let Ok(mut h) = state.health.lock() {
                                        h.window_collector.error_count += 1;
                                        h.window_collector.last_error = Some(format!("{err:#}"));
                                    }
                                }
                            }
                        } else {
                            eprintln!("window event write failed: store lock poisoned");
                            if let Ok(mut h) = state.health.lock() {
                                h.window_collector.error_count += 1;
                                h.window_collector.last_error =
                                    Some("store lock poisoned".into());
                            }
                        }
                    }
                }
                Err(err) => {
                    eprintln!("window sample failed: {err:#}");
                    if let Ok(mut h) = state.health.lock() {
                        h.window_collector.error_count += 1;
                        h.window_collector.last_error = Some(format!("{err:#}"));
                    }
                }
            }

            time::sleep(Duration::from_millis(poll_ms)).await;
        }
    });
}
```

- [ ] **Step 4: Instrument screenshot collector loop**

In `spawn_screenshot_loop`, mark as running at start, update last_event_at on success, error_count on failure:

In the `spawn_screenshot_loop` function, after `let screenshot_dir = ...` and before the `loop`, add:

```rust
{
    if let Ok(mut h) = state.health.lock() {
        h.screenshot_collector.status = "running".into();
    }
}
```

On successful insert (after the `insert_screenshot` call), add:

```rust
if let Ok(mut h) = state.health.lock() {
    h.screenshot_collector.last_event_at = Some(Utc::now());
    h.screenshot_collector.error_count = 0;
    h.screenshot_collector.last_error = None;
}
```

On each `eprintln!("screenshot ... failed: {e:#}");` (dir create, file write), add:

```rust
if let Ok(mut h) = state.health.lock() {
    h.screenshot_collector.error_count += 1;
    h.screenshot_collector.last_error = Some(format!("{e:#}"));
}
```

On capture returning `None` (`capture_thumbnail` line), add before `continue`:

```rust
if let Ok(mut h) = state.health.lock() {
    h.screenshot_collector.error_count += 1;
    h.screenshot_collector.last_error = Some("capture_thumbnail returned None".into());
}
```

- [ ] **Step 5: Update `spawn_input_collector` signature to accept health**

In `input.rs`, change the function signature (line 142):

```rust
use crate::models::{CollectorHealth, InputEvent, InputEventType, TextSegment};
```

Change `spawn_input_collector`:

```rust
pub fn spawn_input_collector(store: Arc<Mutex<Store>>, health: Arc<Mutex<CollectorHealth>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<InputSignal>();

    // Mark as running
    {
        let mut h = health.lock().unwrap();
        h.input_collector.status = "running".into();
    }

    std::thread::spawn(move || {
        platform::run_raw_input_loop(tx);
    });

    tokio::spawn(async move {
        let mut buffer: Option<SegmentBuffer> = None;

        loop {
            match tokio::time::timeout(Duration::from_secs(30), rx.recv()).await {
                Ok(Some(signal)) => {
                    if buffer.is_none() {
                        buffer = Some(SegmentBuffer::new(&signal));
                    }
                    let should_flush = buffer.as_mut().unwrap().ingest(signal);

                    if should_flush {
                        let (segment, events) = buffer.take().unwrap().flush();
                        if let Ok(mut store) = store.lock() {
                            let _ = store.insert_input_segment(&segment, &events);
                        }
                        if let Ok(mut h) = health.lock() {
                            h.input_collector.last_event_at = Some(Utc::now());
                            h.input_collector.error_count = 0;
                            h.input_collector.last_error = None;
                        }
                    }
                }
                Ok(None) => {
                    if let Some(buf) = buffer.take() {
                        let (segment, events) = buf.flush();
                        if let Ok(mut store) = store.lock() {
                            let _ = store.insert_input_segment(&segment, &events);
                        }
                    }
                    break;
                }
                Err(_elapsed) => {
                    if let Some(buf) = buffer.take() {
                        let (segment, events) = buf.flush();
                        if let Ok(mut store) = store.lock() {
                            let _ = store.insert_input_segment(&segment, &events);
                        }
                    }
                }
            }
        }
    });
}
```

- [ ] **Step 6: Update caller in `api.rs` `serve()`**

In `serve()` (line 140), change:

```rust
input::spawn_input_collector(state.store.clone());
```

to:

```rust
input::spawn_input_collector(state.store.clone(), state.health.clone());
```

- [ ] **Step 7: Replace `health()` handler**

Replace the existing `health` function (lines 258-260):

```rust
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let h = match state.health.lock() {
        Ok(h) => h,
        Err(_) => return internal_error("health lock poisoned"),
    };

    let uptime = (Utc::now() - h.started_at).num_seconds().max(0) as u64;

    // Compute overall status
    let overall = if h.window_collector.status == "error"
        || h.input_collector.status == "error"
        || h.screenshot_collector.status == "error"
    {
        "error"
    } else if h.window_collector.status == "not_started"
        || h.input_collector.status == "not_started"
        || h.screenshot_collector.status == "not_started"
    {
        "degraded"
    } else {
        "ok"
    };

    let db_stats = match state.store.lock() {
        Ok(store) => match store.get_db_stats() {
            Ok(stats) => stats,
            Err(_) => DbStats {
                window_events: 0,
                input_events: 0,
                text_segments: 0,
                screenshots: 0,
                blocker_hits: 0,
            },
        },
        Err(_) => DbStats {
            window_events: 0,
            input_events: 0,
            text_segments: 0,
            screenshots: 0,
            blocker_hits: 0,
        },
    };

    Json(CollectorHealth {
        status: overall.into(),
        uptime_seconds: uptime,
        db_stats,
        ..h.clone()
    })
    .into_response()
}
```

Remove the old `HealthResponse` struct (lines 53-57) since it's no longer used:

```rust
// Remove this:
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HealthResponse {
    status: &'static str,
}
```

Update the `use` imports to remove unused `Serialize` from the axum-qualified line — actually `Serialize` is still used by other response structs, keep it.

- [ ] **Step 8: Verify build compiles**

```bash
cargo check -p tsr-collector
```

Expected: compiles without errors. If Rust tests fail to compile due to `health` field missing in test `AppState` construction, proceed to Task 9 to fix.

- [ ] **Step 9: Commit**

```bash
git add collector/src/api.rs collector/src/input.rs
git commit -m "feat: add health state tracking to all collector loops"
```

---

### Task 5: TypeScript Health Types

**Files:**
- Modify: `src/types.ts` (append after `AppInputCount`)

- [ ] **Step 1: Append health types**

After `AppInputCount` (line 95):

```typescript
export type SubsystemHealth = {
  status: "running" | "error" | "not_started";
  lastEventAt?: string;
  errorCount: number;
  lastError?: string;
};

export type DbStats = {
  windowEvents: number;
  inputEvents: number;
  textSegments: number;
  screenshots: number;
  blockerHits: number;
};

export type CollectorHealth = {
  status: "ok" | "degraded" | "error";
  startedAt: string;
  uptimeSeconds: number;
  version: string;
  windowCollector: SubsystemHealth;
  inputCollector: SubsystemHealth;
  screenshotCollector: SubsystemHealth;
  dbStats: DbStats;
};
```

- [ ] **Step 2: Verify type check**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add health-related TypeScript types"
```

---

### Task 6: Health API Client

**Files:**
- Create: `src/lib/health.ts`

- [ ] **Step 1: Create `src/lib/health.ts`**

```typescript
import type { CollectorHealth, DbStats, SubsystemHealth } from "../types";

type Fetcher = (input: string) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchCollectorHealth(
  fetcher: Fetcher = fetch,
): Promise<CollectorHealth> {
  const response = await fetcher("/api/health");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body)) {
    throw new Error("Collector API returned an invalid health response");
  }

  return {
    status: readStatus(body, "status"),
    startedAt: readString(body, "startedAt"),
    uptimeSeconds: readNumber(body, "uptimeSeconds"),
    version: readString(body, "version"),
    windowCollector: readSubsystem(body, "windowCollector"),
    inputCollector: readSubsystem(body, "inputCollector"),
    screenshotCollector: readSubsystem(body, "screenshotCollector"),
    dbStats: readDbStats(body, "dbStats"),
  };
}

function readSubsystem(
  record: Record<string, unknown>,
  key: string,
): SubsystemHealth {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return {
    status: readSubsystemStatus(value, "status"),
    lastEventAt: readOptionalString(value, "lastEventAt"),
    errorCount: readNumber(value, "errorCount"),
    lastError: readOptionalString(value, "lastError"),
  };
}

function readDbStats(
  record: Record<string, unknown>,
  key: string,
): DbStats {
  const value = record[key];
  if (!isRecord(value)) {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return {
    windowEvents: readNumber(value, "windowEvents"),
    inputEvents: readNumber(value, "inputEvents"),
    textSegments: readNumber(value, "textSegments"),
    screenshots: readNumber(value, "screenshots"),
    blockerHits: readNumber(value, "blockerHits"),
  };
}

function readStatus(
  record: Record<string, unknown>,
  key: string,
): "ok" | "degraded" | "error" {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  if (value !== "ok" && value !== "degraded" && value !== "error") {
    throw new Error(`Collector API row has invalid ${key}: ${value}`);
  }
  return value;
}

function readSubsystemStatus(
  record: Record<string, unknown>,
  key: string,
): "running" | "error" | "not_started" {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  if (value !== "running" && value !== "error" && value !== "not_started") {
    throw new Error(`Collector API row has invalid ${key}: ${value}`);
  }
  return value;
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return value;
}

function readOptionalString(
  record: Record<string, unknown>,
  key: string,
): string | undefined {
  const value = record[key];
  if (value === null || value === undefined) {
    return undefined;
  }
  if (typeof value !== "string") {
    throw new Error(`Collector API row has invalid ${key}`);
  }
  return value;
}

function readNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`Collector API row is missing ${key}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
```

- [ ] **Step 2: Verify type check**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/health.ts
git commit -m "feat: add health API client with type-safe parsing"
```

---

### Task 7: CollectorMonitor Component

**Files:**
- Create: `src/CollectorMonitor.tsx`

- [ ] **Step 1: Create `src/CollectorMonitor.tsx`**

```typescript
import { Activity, AlertTriangle, Clock, Database, Monitor, Server } from "lucide-react";
import { useEffect, useState } from "react";
import { fetchCollectorHealth } from "./lib/health";
import type { CollectorHealth, SubsystemHealth } from "./types";

type MonitorStatus = "offline" | "connecting" | "connected";

function formatUptime(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function SubsystemRow({ name, health, icon }: {
  name: string;
  health: SubsystemHealth;
  icon: React.ReactNode;
}) {
  return (
    <div className="subsystemRow">
      <div className="subsystemIcon">{icon}</div>
      <div className="subsystemLabel">{name}</div>
      <span className={`healthPill ${health.status}`}>
        {health.status === "running" ? "Running" : health.status === "error" ? "Error" : "Not Started"}
      </span>
      {health.lastEventAt && (
        <span className="subsystemTime">Last: {formatTime(health.lastEventAt)}</span>
      )}
      {health.errorCount > 0 && (
        <span className="subsystemErrors">{health.errorCount} errors</span>
      )}
    </div>
  );
}

export function CollectorMonitor() {
  const [health, setHealth] = useState<CollectorHealth | null>(null);
  const [status, setStatus] = useState<MonitorStatus>("connecting");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;

    async function poll() {
      try {
        const h = await fetchCollectorHealth();
        if (active) {
          setHealth(h);
          setStatus("connected");
          setError(null);
        }
      } catch (err) {
        if (active) {
          setStatus("offline");
          setError(err instanceof Error ? err.message : String(err));
        }
      }
    }

    void poll();
    const interval = setInterval(() => void poll(), 5000);
    return () => {
      active = false;
      clearInterval(interval);
    };
  }, []);

  if (status === "connecting") {
    return (
      <div className="panel">
        <div className="panelHeader">
          <Server aria-hidden="true" size={20} />
          <h2>Collector Monitor</h2>
        </div>
        <p className="monitorConnecting">
          <Activity aria-hidden="true" size={16} />
          Connecting to collector...
        </p>
      </div>
    );
  }

  if (status === "offline") {
    return (
      <div className="panel">
        <div className="panelHeader">
          <Server aria-hidden="true" size={20} />
          <h2>Collector Monitor</h2>
        </div>
        <dl className="statusList">
          <div>
            <dt>Status</dt>
            <dd>
              <span className="healthPill error">Offline</span>
            </dd>
          </div>
        </dl>
        <p className="offlineHint">
          Collector not reachable. Run <code>start.bat</code> or{" "}
          <code>npm run collector</code> to start.
        </p>
        {error && (
          <p className="errors" role="status">
            {error}
          </p>
        )}
      </div>
    );
  }

  // Connected
  const anyError = health?.windowCollector.lastError
    || health?.inputCollector.lastError
    || health?.screenshotCollector.lastError;

  return (
    <div className="panel collectorMonitor">
      <div className="panelHeader">
        <Server aria-hidden="true" size={20} />
        <h2>Collector Monitor</h2>
      </div>

      <div className="healthGrid">
        <div className="healthOverview">
          <span className={`healthPill ${health!.status}`}>
            {health!.status === "ok" ? "Healthy" : health!.status === "degraded" ? "Degraded" : "Error"}
          </span>
          <span className="healthUptime">
            <Clock aria-hidden="true" size={14} />
            Uptime {formatUptime(health!.uptimeSeconds)}
          </span>
        </div>
        <div className="healthMeta">
          <span>v{health!.version}</span>
          <span>Started {formatTime(health!.startedAt)}</span>
        </div>
      </div>

      <div className="monitorSection">
        <div className="monitorSectionHeader">
          <Monitor aria-hidden="true" size={16} />
          <h3>Subsystems</h3>
        </div>
        <div className="subsystemList">
          <SubsystemRow
            name="Window Collector"
            health={health!.windowCollector}
            icon={<Activity aria-hidden="true" size={14} />}
          />
          <SubsystemRow
            name="Input Collector"
            health={health!.inputCollector}
            icon={<Activity aria-hidden="true" size={14} />}
          />
          <SubsystemRow
            name="Screenshot Collector"
            health={health!.screenshotCollector}
            icon={<Activity aria-hidden="true" size={14} />}
          />
        </div>
      </div>

      <div className="monitorSection">
        <div className="monitorSectionHeader">
          <Database aria-hidden="true" size={16} />
          <h3>Database</h3>
        </div>
        <div className="dbStatList">
          <div className="dbStatRow">
            <span>Window Events</span>
            <strong>{health!.dbStats.windowEvents.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Input Events</span>
            <strong>{health!.dbStats.inputEvents.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Text Segments</span>
            <strong>{health!.dbStats.textSegments.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Screenshots</span>
            <strong>{health!.dbStats.screenshots.toLocaleString()}</strong>
          </div>
          <div className="dbStatRow">
            <span>Blocker Hits</span>
            <strong>{health!.dbStats.blockerHits.toLocaleString()}</strong>
          </div>
        </div>
      </div>

      {anyError && (
        <div className="errorBanner">
          <AlertTriangle aria-hidden="true" size={16} />
          <div>
            {health?.windowCollector.lastError && (
              <p>Window: {health.windowCollector.lastError}</p>
            )}
            {health?.inputCollector.lastError && (
              <p>Input: {health.inputCollector.lastError}</p>
            )}
            {health?.screenshotCollector.lastError && (
              <p>Screenshot: {health.screenshotCollector.lastError}</p>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Verify type check**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add src/CollectorMonitor.tsx
git commit -m "feat: add CollectorMonitor component with subsystem health and DB stats"
```

---

### Task 8: Wire into App.tsx + CSS

**Files:**
- Modify: `src/App.tsx:143-177` (replace Collector Connection panel)
- Modify: `src/styles.css` (append new styles)

- [ ] **Step 1: Import CollectorMonitor in App.tsx**

Add import after the InputActivity import:

```typescript
import { CollectorMonitor } from "./CollectorMonitor";
```

Replace the icons import line to add `Monitor` — actually not needed since `Server` is already imported and we use it in `CollectorMonitor`. Keep existing imports as-is.

- [ ] **Step 2: Replace the Collector Connection panel**

Replace the existing panel (lines 143-177, the one with `<Server>` and `statusList`) with:

```tsx
            <CollectorMonitor />
```

So the workspace section becomes:

```tsx
          <section className="workspace">
            <div className="panel">
              <div className="panelHeader">
                <BarChart3 aria-hidden="true" size={20} />
                <h2>Application Time</h2>
              </div>
              <div className="bars">
                {appSummary.map((item) => (
                  <div className="barRow" key={item.app}>
                    <div className="barLabel">
                      <span>{item.app}</span>
                      <strong>{formatSeconds(item.totalSeconds)}</strong>
                    </div>
                    <div className="barTrack" aria-hidden="true">
                      <div
                        className="barFill"
                        style={{ width: `${(item.totalSeconds / largest) * 100}%` }}
                      />
                    </div>
                    <div className="barMeta">
                      <span>{item.eventCount} events</span>
                      <span>{Math.round(item.share * 100)}%</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>

            <CollectorMonitor />
          </section>
```

- [ ] **Step 3: Remove unused `refreshCollector` and related code from App.tsx**

The `CollectorMonitor` component handles its own auto-refresh. The `refreshCollector` function in App.tsx (lines 37-48) is still used by the topbar "Collector Data" button for fetching time events — keep it.

The `collectorStatus` and `collectorError` state (lines 20-21) are now unused in the Stats view since `CollectorMonitor` handles its own state. Remove lines 20-21:

```typescript
// Remove these lines:
const [collectorStatus, setCollectorStatus] = useState<CollectorStatus>("sample");
const [collectorError, setCollectorError] = useState<string | null>(null);
```

Remove the `CollectorStatus` type (line 15):
```typescript
// Remove this line:
type CollectorStatus = "sample" | "loading" | "connected" | "offline";
```

Update `refreshCollector` to use local state. Actually — the `refreshCollector` function updates `events` directly and uses `setEvents` to populate the stats view. The `collectorStatus` and `collectorError` are NOT truly unused — they're still used in `loadSample` and `refreshCollector`. Let me reconsider.

Actually, the Stats view no longer shows a "Collector Connection" panel with `collectorStatus` and `collectorError`. The CollectorMonitor handles all of that. But `loadSample` calls `setCollectorStatus("sample")` and `setCollectorError(null)`, and `refreshCollector` calls `setCollectorStatus("loading")` etc. These are just stale — the UI no longer renders them anywhere.

Keep the state and functions for now — removing them would break the topbar "Collector Data" button. The `collectorStatus` and `collectorError` are spread across `loadSample` + `refreshCollector` which update `events`. Keep everything except the panel replacement above. No deletions needed beyond the panel swap.

Also remove unused imports: `RefreshCw` is still used in topbar buttons, `RotateCcw` still used. `Server` is no longer used in App.tsx but is used in CollectorMonitor.tsx. Remove `Server` from App.tsx imports:

```typescript
import { BarChart3, Camera, Keyboard, RefreshCw, RotateCcw, TableProperties } from "lucide-react";
```

- [ ] **Step 4: Add CSS styles to `src/styles.css`**

Append before the first `@media` block:

```css
/* Collector Monitor */

.collectorMonitor {
  display: grid;
  gap: 14px;
}

.healthGrid {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 6px;
  background: #f3f6f2;
}

.healthOverview {
  display: flex;
  align-items: center;
  gap: 10px;
}

.healthUptime {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: #5f6d65;
  font-size: 0.82rem;
  font-weight: 600;
}

.healthMeta {
  display: flex;
  gap: 16px;
  color: #5f6d65;
  font-size: 0.78rem;
  font-weight: 600;
}

.monitorConnecting {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 8px 0 0;
  color: #5f6d65;
  font-size: 0.88rem;
}

.offlineHint {
  margin: 8px 0 0;
  padding: 10px 14px;
  border-radius: 6px;
  background: #f5dfdd;
  color: #8a312e;
  font-size: 0.82rem;
}

.offlineHint code {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  background: rgba(0, 0, 0, 0.06);
  padding: 1px 4px;
  border-radius: 3px;
}

.healthPill {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  padding: 3px 10px;
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
}

.healthPill.ok,
.healthPill.running {
  background: #d9ecdf;
  color: #1f6b3b;
}

.healthPill.degraded,
.healthPill.not_started {
  background: #fef3cf;
  color: #8a6d10;
}

.healthPill.error {
  background: #f5dfdd;
  color: #8a312e;
}

.monitorSection {
  border-top: 1px solid #e2e6e1;
  padding-top: 10px;
}

.monitorSectionHeader {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  color: #5f6d65;
}

.monitorSectionHeader h3 {
  margin: 0;
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
}

.subsystemList {
  display: grid;
  gap: 8px;
}

.subsystemRow {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 8px;
  border-radius: 6px;
  background: rgba(255, 255, 255, 0.7);
}

.subsystemIcon {
  color: #5f6d65;
  display: flex;
}

.subsystemLabel {
  flex: 1;
  font-size: 0.84rem;
  font-weight: 600;
  color: #17211b;
}

.subsystemTime {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.75rem;
  color: #5f6d65;
}

.subsystemErrors {
  font-size: 0.75rem;
  font-weight: 700;
  color: #b84a45;
}

.dbStatList {
  display: grid;
  gap: 6px;
}

.dbStatRow {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 8px;
  font-size: 0.84rem;
}

.dbStatRow strong {
  font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
  font-size: 0.82rem;
}

.errorBanner {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 10px 14px;
  border-radius: 6px;
  background: #f5dfdd;
  color: #8a312e;
  font-size: 0.8rem;
}

.errorBanner p {
  margin: 2px 0;
}
```

- [ ] **Step 5: Verify build and type check**

```bash
npx tsc --noEmit && npm run build
```

- [ ] **Step 6: Commit**

```bash
git add src/App.tsx src/styles.css
git commit -m "feat: replace Collector Connection panel with CollectorMonitor"
```

---

### Task 9: Tests

**Files:**
- Create: `src/lib/health.test.ts`
- Modify: `collector/tests/api_tests.rs` (append)

- [ ] **Step 1: Create `src/lib/health.test.ts`**

```typescript
import { describe, expect, it, vi } from "vitest";
import { fetchCollectorHealth } from "./health";

describe("fetchCollectorHealth", () => {
  it("loads collector health from /api/health", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        status: "ok",
        startedAt: "2026-05-24T10:00:00Z",
        uptimeSeconds: 3600,
        version: "0.1.0",
        windowCollector: {
          status: "running",
          lastEventAt: "2026-05-24T10:59:00Z",
          errorCount: 0,
        },
        inputCollector: {
          status: "running",
          lastEventAt: "2026-05-24T10:58:30Z",
          errorCount: 0,
        },
        screenshotCollector: {
          status: "error",
          lastEventAt: "2026-05-24T09:12:00Z",
          errorCount: 3,
          lastError: "capture_thumbnail returned None",
        },
        dbStats: {
          windowEvents: 452,
          inputEvents: 3421,
          textSegments: 87,
          screenshots: 128,
          blockerHits: 3,
        },
      }),
    });

    const health = await fetchCollectorHealth(fetcher);
    expect(health.status).toBe("ok");
    expect(health.uptimeSeconds).toBe(3600);
    expect(health.version).toBe("0.1.0");
    expect(health.windowCollector.status).toBe("running");
    expect(health.inputCollector.lastEventAt).toBe("2026-05-24T10:58:30Z");
    expect(health.screenshotCollector.status).toBe("error");
    expect(health.screenshotCollector.errorCount).toBe(3);
    expect(health.screenshotCollector.lastError).toBe("capture_thumbnail returned None");
    expect(health.dbStats.windowEvents).toBe(452);
    expect(health.dbStats.blockerHits).toBe(3);
    expect(fetcher).toHaveBeenCalledWith("/api/health");
  });

  it("reports API failures", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
    });

    await expect(fetchCollectorHealth(fetcher)).rejects.toThrow(
      "Collector API failed: 503 Service Unavailable",
    );
  });

  it("handles missing optional fields", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        status: "ok",
        startedAt: "2026-05-24T10:00:00Z",
        uptimeSeconds: 0,
        version: "0.1.0",
        windowCollector: {
          status: "not_started",
          errorCount: 0,
        },
        inputCollector: {
          status: "not_started",
          errorCount: 0,
        },
        screenshotCollector: {
          status: "not_started",
          errorCount: 0,
        },
        dbStats: {
          windowEvents: 0,
          inputEvents: 0,
          textSegments: 0,
          screenshots: 0,
          blockerHits: 0,
        },
      }),
    });

    const health = await fetchCollectorHealth(fetcher);
    expect(health.windowCollector.lastEventAt).toBeUndefined();
    expect(health.windowCollector.lastError).toBeUndefined();
  });
});
```

- [ ] **Step 2: Run TypeScript tests**

```bash
npx vitest run
```

Expected: 20 tests pass (17 existing + 3 new).

- [ ] **Step 3: Update Rust tests for health endpoint**

In `collector/tests/api_tests.rs`, add a test for the health endpoint. Append after the last test:

```rust
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
```

- [ ] **Step 4: Run Rust tests**

```bash
cargo test -p tsr-collector
```

Expected: all tests pass including the new health test.

- [ ] **Step 5: Commit**

```bash
git add src/lib/health.test.ts collector/tests/api_tests.rs
git commit -m "test: add health API client tests and Rust health endpoint test"
```

---

### Task 10: Update Documentation

**Files:**
- Modify: `README.md` (update architecture + API table + verification)
- Modify: `docs/infra.md` (update diagram + API + UI sections)

- [ ] **Step 1: Update README.md API endpoint table**

Add after the input-summary row in the API table:

```markdown
| `GET` | `/api/health` | — | Full collector health (status, uptime, subsystem states, DB row counts) |
```

Note: the health endpoint already exists at `/api/health` — its response changed. Update the existing row description.

Find the existing health row:
```markdown
| `GET` | `/api/health` | — | `{"status":"ok"}` |
```

Replace with:
```markdown
| `GET` | `/api/health` | — | Collector health (status, uptime, per-subsystem states, DB row counts) |
```

- [ ] **Step 2: Update verification checklist in README.md**

Add after the `serve` checklist item:

```markdown
- `/api/health` returns full `CollectorHealth` JSON with uptime, subsystem states, and DB row counts.
- WebUI CollectorMonitor shows offline state with launch instructions when collector is unreachable.
- WebUI CollectorMonitor auto-refreshes every 5 seconds with live subsystem health and DB stats.
```

- [ ] **Step 3: Update docs/infra.md API section**

In the API list, update the health entry:

```markdown
- `/api/health` — full collector health (status, uptime, per-subsystem state, DB row counts).
```

- [ ] **Step 4: Update docs/infra.md UI section**

Add after the existing views:

```markdown
- Collector Monitor replaces the old Collector Connection panel with real-time subsystem health and DB row counts.
```

- [ ] **Step 5: Commit**

```bash
git add README.md docs/infra.md
git commit -m "docs: update docs for collector monitoring and launcher"
```

---

## Summary

| Task | Files | What |
|------|-------|------|
| 1 | `collector/start.bat`, `package.json` | Launcher scripts |
| 2 | `collector/src/models.rs` | Health Rust models |
| 3 | `collector/src/storage.rs` | DB row count queries |
| 4 | `collector/src/api.rs`, `collector/src/input.rs` | Health state + loop instrumentation |
| 5 | `src/types.ts` | TypeScript health types |
| 6 | `src/lib/health.ts` | Health API client |
| 7 | `src/CollectorMonitor.tsx` | Monitor component |
| 8 | `src/App.tsx`, `src/styles.css` | Wire component + styles |
| 9 | `src/lib/health.test.ts`, `collector/tests/api_tests.rs` | Tests |
| 10 | `README.md`, `docs/infra.md` | Documentation |
