# Time State Recorder MVP

Time State Recorder is a local-first, Windows-first project for understanding how time is spent on a computer. The long-term product is a recorder that stores raw activity events locally, derives rollups, and presents useful summaries without sending private activity data to a cloud service.

This repository now contains: a Rust/SQLite Windows foreground-window collector, a periodic screenshot capture system with idle detection, a blocker/filter engine for privacy-aware capture, a local JSON API, and a React/TypeScript WebUI with statistics and daily timeline views.

## Current MVP Scope

The MVP answers two questions:

> 1. Given active-window events, what applications were active, for how long, and what are the basic descriptive statistics of those durations?
> 2. What does the screen look like minute-by-minute throughout the day, and which apps are visible?

Included:

- **Feature 1** — Active window/process monitoring via polling `GetForegroundWindow`.
- **Feature 3** — Periodic screenshot thumbnails with idle detection and file-system storage.
- **Blocker system** — JSON-configurable app/window/title blocklist shared by all capture types.
- Sample the real Windows foreground window with a Rust CLI.
- Capture screen thumbnails every 60 seconds (only when user is actively using the machine).
- Store `window_focus` raw events, screenshot metadata, and blocker hits in SQLite.
- Serve collector data over local REST JSON endpoints.
- Serve screenshot images as static files for the WebUI.
- Load built-in `feature1` and `feature3` sample datasets for demo and regression checks.
- Validate required fields before analysis.
- Calculate count, mean, median, standard deviation, min, max, quartiles, and total duration.
- Group active duration by application/process and display cards or tables in the WebUI.
- View a daily screenshot timeline with per-app breakdown.
- Run fully locally through `tsr-collector` and the browser/dev server.

Not included yet:

- `SetWinEventHook` event-driven collection.
- Tray app or installed background service.
- Keyboard/input monitoring (Feature 2 — text capture).
- OCR or high-resolution screenshot archival.
- Cloud sync or multi-device support.

## Feature 1 Definition

Feature 1 is active window/process monitoring.

In the full recorder, Feature 1 observes foreground-window changes on Windows and writes raw `window_focus` events containing timestamp, process name, PID, window title or redacted title, executable hash, and capture status.

The Rust collector polls `GetForegroundWindow` at a configurable interval (default 1000ms). The WebUI can fall back to built-in sample rows when the collector is offline.

## Feature 3 Definition

Feature 3 is periodic screenshot tracking.

The screenshot collector captures a thumbnail of the primary monitor every 60 seconds, but only when the user is "active" (keyboard or mouse input within the last 2 minutes, detected via `GetLastInputInfo`). Each capture is:

- Resized to max 640px width, preserving aspect ratio.
- Encoded as JPEG and stored at `data/screenshots/YYYY-MM-DD/HH-MM.jpg`.
- Metadata (timestamp, file path, dimensions, foreground app/window) is stored in the `screenshot_thumbnails` SQLite table.
- Checked against the blocker config — blocked apps/windows are never captured.

The WebUI Daily Tracking view shows a timeline of the day's screenshots grouped by hour, with a summary bar showing total count, hours covered, and top apps by screenshot count. Click any thumbnail to expand it.

## Blocker System

The blocker engine provides a privacy-aware filter that applies before any capture (screenshots, future text capture). It reads rules from `collector/blocker_config.json`:

```json
{
  "version": 1,
  "rules": [
    { "capture_type": "screenshot", "field": "process_name", "operator": "equals", "value": "Taskmgr.exe" },
    { "capture_type": "screenshot", "field": "window_title", "operator": "contains", "value": "Banking" }
  ]
}
```

Supported fields: `process_name`, `window_title`, `exe_path_hash`. Operators: `equals`, `contains`, `starts_with`. Blocked capture attempts are logged to the `blocker_hits` table and exposed via `GET /api/blockers`.

## App Architecture

The MVP uses a local frontend/backend flow:

1. Windows collector samples foreground-window state and captures periodic screen thumbnails.
2. SQLite stores `raw_events`, `window_events`, `screenshot_thumbnails`, and `blocker_hits`.
3. Local REST API exposes raw focus events, interval-shaped time events, screenshot metadata, screenshot summaries, and blocker config/hits.
4. Screenshot image files are served as static files via `/screenshots/`.
5. WebUI fetches `/api/time-events`, `/api/screenshots`, and `/api/screenshot-summary`.
6. Stats engine computes descriptive statistics and per-application duration summaries.
7. UI renders summary cards, application bar chart, collector status, event table, and daily screenshot timeline.

The collector uses polling first because it is verifiable and provides the fallback path required by the architecture. A later collector iteration can add `SetWinEventHook` without changing the WebUI contract.

## API Endpoints

| Method | Path | Query Params | Description |
| --- | --- | --- | --- |
| `GET` | `/api/health` | — | `{"status":"ok"}` |
| `GET` | `/api/window-events` | `?limit=N` | Raw joined window-focus events |
| `GET` | `/api/time-events` | `?limit=N` | Interval-shaped time events for statistics |
| `GET` | `/api/blockers` | `?limit=N` | Blocker rules and recent hits |
| `GET` | `/api/screenshots` | `?date=YYYY-MM-DD&limit=N` | Screenshot metadata for a given date |
| `GET` | `/api/screenshot-summary` | `?date=YYYY-MM-DD` | Aggregated screenshot stats (count, hours, top apps) |

Static files: `/screenshots/YYYY-MM-DD/HH-MM.jpg` serves captured thumbnail images.

## WebUI Views

The WebUI has two tabbed views:

- **Statistics** — Descriptive stats grid, application time bar chart, collector connection panel, and event rows table (Feature 1).
- **Daily Tracking** — Summary bar (total screenshots, hours active, top apps) and a scrollable timeline with thumbnails grouped by hour (Feature 3).

Both views support switching between built-in sample data and live collector data.

## UI Data Model

### Time Events (`GET /api/time-events`)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | string | Yes | Stable UI row identifier derived from raw event ids. |
| `app` | string | Yes | Application/process display name. |
| `title` | string | Yes | Window title or redacted/empty fallback. |
| `startedAt` | ISO timestamp/string | Yes | Start time of the active interval. |
| `endedAt` | ISO timestamp/string | Optional | End time. The currently focused window has no end yet. |
| `durationSeconds` | number | Optional | Active duration for completed intervals. |

### Screenshots (`GET /api/screenshots`)

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | number | Yes | Row id. |
| `capturedAt` | ISO timestamp/string | Yes | Capture timestamp. |
| `filePath` | string | Yes | Relative path, e.g. `2026-05-23/14-30.jpg`. |
| `width` | number | Yes | Thumbnail width in pixels. |
| `height` | number | Yes | Thumbnail height in pixels. |
| `processName` | string | Optional | Foreground app at capture time. |
| `windowTitle` | string | Optional | Window title at capture time. |
| `captureStatus` | string | Yes | `"ok"` or error status. |

Derived analysis objects should not mutate normalized event rows. SQLite preserves raw collector payloads and the WebUI receives normalized JSON from the local API. Future recorder/export layers should preserve full raw event provenance alongside derived records.

## Local Run and Test Commands

Use the project package manager and Rust toolchain:

```powershell
npm install
cargo test -p tsr-collector
cargo run -p tsr-collector -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317
npm run dev
npm test
npm run build
```

If the implementation uses a different package manager, keep equivalent scripts for:

- `dev`: start the local WebUI.
- `test`: run WebUI API/statistics tests.
- `build`: verify the static app compiles.

## Verification Checklist

- `sample-once` returns the current foreground process/window JSON.
- `record` writes `window_focus` events into SQLite.
- `serve` exposes `/api/health`, `/api/window-events`, `/api/time-events`, `/api/blockers`, `/api/screenshots`, and `/api/screenshot-summary`.
- Screenshot capture writes JPEG files to `data/screenshots/YYYY-MM-DD/HH-MM.jpg` every 60s when user is active.
- Screenshots are skipped when the user has been idle for >2 minutes.
- Blocked apps/windows are never captured; hits are logged in `blocker_hits`.
- Screenshot images are accessible via `/screenshots/YYYY-MM-DD/HH-MM.jpg`.
- WebUI loads collector data and falls back to sample datasets when offline.
- Descriptive statistics include count, mean, median, standard deviation, min, max, Q1, and Q3.
- Per-application summary totals match the row-level total duration.
- Daily Tracking timeline shows screenshots grouped by hour with app/title context.
- Empty API responses, malformed JSON payloads, missing fields, and invalid timestamps are handled without crashing.
- `cargo test -p tsr-collector`, `npm test`, and `npm run build` pass before review.

## Non-Goals and Risks

Non-goals for this MVP:

- No installed Windows service.
- No background collection, hooks, Raw Input, UI Automation.
- No sensitive text capture (Feature 2).
- No OCR or high-resolution screenshot archival.
- No productivity classification, AI labeling, or automatic task inference.

Risks:

- Sample fallback data can hide edge cases that live Windows collection will produce, such as permission-denied windows, lock screen gaps, title changes, and rapid focus switches.
- API response fields may drift unless collector models, WebUI validation, and documentation stay aligned.
- Window titles and screenshots can contain private information. The blocker system provides a first line of defense, but it is config-based and not automatic. The MVP should tolerate missing or redacted titles.
- Screenshots are stored as JPEG files on disk with no encryption; access control relies on the local machine's filesystem permissions.
- Local browser access must stay same-origin or proxied; permissive CORS would expose private activity data to arbitrary websites.
- Statistics are only as reliable as interval construction. Future recorder work must define how focus events become intervals.
- `GetLastInputInfo` only reports input for the current user session and can be fooled by continuous-input devices (game controllers, certain drivers).
