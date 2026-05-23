# Time State Recorder MVP

Time State Recorder is a local-first, Windows-first project for understanding how time is spent on a computer. The long-term product is a recorder that stores raw activity events locally, derives rollups, and presents useful summaries without sending private activity data to a cloud service.

This repository now contains the first live collector slice: a Rust/SQLite Windows foreground-window collector with a local JSON API, plus a React/TypeScript WebUI that consumes that API and computes descriptive statistics.

## Current MVP Scope

The MVP answers one question:

> Given active-window events, what applications were active, for how long, and what are the basic descriptive statistics of those durations?

Included:

- Sample the real Windows foreground window with a Rust CLI.
- Store `window_focus` raw events in SQLite.
- Serve collector data over local REST JSON endpoints.
- Load the built-in `feature1` sample dataset for demo and regression checks.
- Validate required fields before analysis.
- Calculate count, mean, median, standard deviation, min, max, quartiles, and total duration.
- Group active duration by application/process and display cards or tables in the WebUI.
- Run fully locally through `tsr-collector` and the browser/dev server.

Not included yet:

- `SetWinEventHook` event-driven collection.
- Tray app or installed background service.
- Keyboard/input monitoring.
- Screenshot thumbnail capture.
- Cloud sync or multi-device support.

## Feature 1 Definition

Feature 1 is active window/process monitoring.

In the full recorder, Feature 1 will observe foreground-window changes on Windows and write raw `window_focus` events containing timestamp, process name, PID, window title or redacted title, executable hash, and capture status.

In this MVP, Feature 1 is implemented by the Rust collector. The WebUI can still fall back to built-in sample rows when the collector is offline.

## App Architecture

The MVP uses a local frontend/backend flow:

1. Windows collector samples foreground-window state.
2. SQLite stores `raw_events` and `window_events`.
3. Local REST API exposes raw focus events and interval-shaped time events.
4. WebUI fetches `/api/time-events`.
5. Stats engine computes descriptive statistics and per-application duration summaries.
6. UI renders summary cards, collector status, and event tables.

The collector uses polling first because it is verifiable and provides the fallback path required by the architecture. A later collector iteration can add `SetWinEventHook` without changing the WebUI contract.

## UI Data Model

The WebUI expects `GET /api/time-events` to return `{ "events": [...] }` with normalized interval records.

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `id` | string | Yes | Stable UI row identifier derived from raw event ids. |
| `app` | string | Yes | Application/process display name. |
| `title` | string | Yes | Window title or redacted/empty fallback. |
| `startedAt` | ISO timestamp/string | Yes | Start time of the active interval. |
| `endedAt` | ISO timestamp/string | Optional | End time. The currently focused window has no end yet. |
| `durationSeconds` | number | Optional | Active duration for completed intervals. |

`GET /api/window-events` exposes lower-level collector rows for debugging and future recorder work. Those rows include `rawEventId`, `sessionId`, `eventTs`, `hwnd`, `pid`, `processName`, `exePathHash`, `windowTitle`, and `captureStatus`.

Derived analysis objects should not mutate normalized event rows. In the current MVP, SQLite preserves raw collector payloads and the WebUI receives normalized JSON from the local API. Future recorder/export layers should preserve full raw event provenance alongside derived records.

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
- `serve` exposes `/api/health`, `/api/window-events`, and `/api/time-events`.
- WebUI loads collector data from `/api/time-events` and falls back to `feature1` sample when offline.
- Descriptive statistics include count, mean, median, standard deviation, min, max, Q1, and Q3.
- Per-application summary totals match the row-level total duration.
- Empty API responses, malformed JSON payloads, missing app names, and invalid timestamps are handled without crashing.
- `cargo test -p tsr-collector`, `npm test`, and `npm run build` pass before review.

## Non-Goals and Risks

Non-goals for this MVP:

- No installed Windows service.
- No background collection, hooks, Raw Input, UI Automation, or screenshot APIs.
- No sensitive text capture.
- No productivity classification, AI labeling, or automatic task inference.

Risks:

- Sample fallback data can hide edge cases that live Windows collection will produce, such as permission-denied windows, lock screen gaps, title changes, and rapid focus switches.
- API response fields may drift unless collector models, WebUI validation, and documentation stay aligned.
- Window titles can contain private information. The MVP should tolerate missing or redacted titles.
- Local browser access must stay same-origin or proxied; permissive CORS would expose private activity data to arbitrary websites.
- Statistics are only as reliable as interval construction. Future recorder work must define how focus events become intervals.
