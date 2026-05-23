# Collector Launcher + Monitoring Dashboard — Design Spec

**Goal:** Provide a user-friendly way to launch the collector and monitor the health of all subsystems (window, input, screenshot) in real time from the WebUI.

**Architecture:** A `Arc<Mutex<CollectorHealth>>` shared in `AppState` that each collector loop updates on success/error. The `/api/health` endpoint reads this state and returns full subsystem status. The WebUI polls health every 5 seconds and renders a monitoring dashboard with per-subsystem status indicators, database row counts, uptime, and error logs.

**Tech Stack:** Rust/Axum (enhanced health endpoint), React/TypeScript (monitoring panel), Windows batch/PowerShell (launcher scripts), npm scripts (dev launcher).

---

## Part 1: Launcher Scripts

### Rust CLI (no changes)

The existing `tsr-collector serve` command is unchanged. Defaults: `--db data/local.sqlite3 --addr 127.0.0.1:4317 --poll-ms 1000 --blocker-config blocker_config.json`.

### Windows batch script (`collector/start.bat`)

Checks for release binary first, falls back to debug, otherwise instructs user to build.

```batch
@echo off
if exist "target\release\tsr-collector.exe" (
    echo Starting collector (release)...
    target\release\tsr-collector.exe serve --db data/local.sqlite3 --addr 127.0.0.1:4317
) else if exist "target\debug\tsr-collector.exe" (
    echo Starting collector (debug)...
    target\debug\tsr-collector.exe serve --db data/local.sqlite3 --addr 127.0.0.1:4317
) else (
    echo Collector binary not found. Build first: cargo build --release -p tsr-collector
    pause
)
```

### npm scripts (`package.json`)

```json
"collector": "cargo run -p tsr-collector -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317",
"collector:release": "cargo run -p tsr-collector --release -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317"
```

---

## Part 2: Enhanced `/api/health` Endpoint

### Rust models (`collector/src/models.rs`)

Three new structs:

```rust
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectorHealth {
    pub status: String,                  // "ok" | "degraded" | "error"
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
    pub status: String,           // "running" | "error" | "not_started"
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

### Health state management (`collector/src/api.rs`)

`AppState` gains a new field:

```rust
pub struct AppState {
    // ...existing...
    health: Arc<Mutex<CollectorHealth>>,
}
```

On startup (`serve()`), initialize `CollectorHealth` with `started_at: Utc::now()`, version from `CARGO_PKG_VERSION`, and all subsystems at `"not_started"`.

Each collector loop updates the shared health state:

**Window collector loop:**
- On successful `sample_foreground_window()`: set `window_collector.status = "running"`, update `last_event_at`
- On error: increment `error_count`, set `last_error` to the error message

**Input collector (drain task):**
- On successful segment flush: update `last_event_at`
- On Raw Input registration failure: set `status = "error"`, set `last_error`
- On drain error: increment `error_count`

**Screenshot collector loop:**
- On successful capture + write: update `last_event_at`
- On capture/write failure: increment `error_count`, set `last_error`
- On blocker skip: no change (normal operation)

The health handler computes `uptime_seconds` as `(Utc::now() - started_at).num_seconds()` and `status` as:
- `"error"` if any subsystem has `status == "error"`
- `"degraded"` if any subsystem has `status == "not_started"`
- `"ok"` otherwise

DB stats: query each table's row count with `SELECT COUNT(*) FROM ...`.

The health endpoint becomes:

```rust
async fn health(State(state): State<AppState>) -> impl IntoResponse {
    let health = state.health.lock().unwrap();
    let db_stats = compute_db_stats(&state.store.lock().unwrap());
    // merge and return
}
```

### Collectors must update health

**Window collector** — already has `eprintln!("window event write failed: {err:#}")`. Changed to also write to `health.window_collector`.

**Input collector** — the `spawn_input_collector` function in `input.rs` receives the `Arc<Mutex<CollectorHealth>>` and updates `input_collector`.

**Screenshot collector** — already has `eprintln!("screenshot ... failed: {e:#}")`. Changed to also write to `health.screenshot_collector`.

---

## Part 3: WebUI Collector Monitor

### TypeScript types (`src/types.ts`)

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

### API client (`src/lib/health.ts`)

```typescript
export async function fetchCollectorHealth(fetcher = fetch): Promise<CollectorHealth>
```

Calls `GET /api/health`, validates response shape with same `readString`/`readNumber`/`isRecord` pattern. Returns typed `CollectorHealth`.

### CollectorMonitor component (`src/CollectorMonitor.tsx`)

Replaces the existing "Collector Connection" panel in the Statistics view. Uses `useEffect` with a 5-second `setInterval` to poll `/api/health`.

Layout (see design in brainstorming). Key states:

- **Loading:** Spinner with "Connecting..."
- **Offline:** Red status pill "Offline", message "Collector not reachable. Run `start.bat` or `npm run collector` to start."
- **Connected:** Full monitor view with all sections

Subsystem rows show:
- Name label
- Status pill (green "Running" / yellow "Not started" / red "Error")
- Last event time (if available)
- Error count (if > 0)

DB stats: simple key-value rows with row counts.

Error banner at bottom: only if any subsystem has `last_error`, shows red bar with the error text.

### App.tsx changes

Replace the existing "Collector Connection" panel (the one with `Server` icon) with `<CollectorMonitor />`. The topbar refresh button still works for fetching time events (Statistics tab).

If monitor shows Offline, the Statistics and Input Activity tabs' "Live Data" buttons show error states — existing behavior unchanged.

### Styles (`src/styles.css`)

New CSS:
- `.collectorMonitor` — panel wrapper
- `.healthGrid` — grid layout for the health overview section
- `.healthPill` — status indicator pill (extends `.statusPill` with `running`/`error`/`not_started` variants)
- `.subsystemRow` — per-subsystem row with indicator + label + detail
- `.dbStatRow` — simple key-value row for DB stats
- `.errorBanner` — red error banner at bottom

---

## Verification Checklist

- `collector/start.bat` double-click launches collector in a terminal window
- `npm run collector` starts collector from dev environment
- `/api/health` returns full `CollectorHealth` JSON when collector is running
- WebUI CollectorMonitor shows "Offline" with launch instructions when collector not running
- WebUI CollectorMonitor shows per-subsystem status, uptime, DB row counts when connected
- Subsystem status turns red when a collector loop encounters errors
- 5-second auto-refresh updates the monitor
- `npm run build` and `npm test` pass
- Existing Rust tests still pass (`cargo test -p tsr-collector`)
