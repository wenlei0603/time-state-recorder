# MVP Information Flow

This document explains how information moves through the live collector MVP and how that path maps to the future local recorder infrastructure.

## Current Flow

The current MVP has a real local backend with three subsystems: a window collector, a screenshot collector with idle detection, and a blocker/filter engine. The Rust collector writes SQLite rows and JPEG files, exposes JSON over a local REST API, serves screenshot images as static files, and the WebUI renders descriptive statistics and a daily screenshot timeline.

```mermaid
flowchart LR
    OS["Windows foreground + input APIs"] --> WinCollector["Window Collector"]
    OS --> SSCollector["Screenshot Collector"]
    WinCollector --> Storage["SQLite"]
    SSCollector --> Storage
    SSCollector --> Files["data/screenshots/"]
    Blocker["Blocker Engine"] --> SSCollector
    Storage --> API["Local REST API"]
    API --> Client["WebUI API Client"]
    Files --> Client
    Sample1["feature1 sample"] --> Client
    Sample3["feature3 sample"] --> Client
    Client --> Stats["Stats engine"]
    Client --> Timeline["Daily Tracking timeline"]
    Stats --> Cards["UI summary cards"]
    Stats --> Tables["Per-application tables"]
```

## Step Details

Window Collector (Feature 1):

- `sample-once` reads the current foreground window.
- `record` writes focus changes for a bounded period.
- `serve` runs the polling collector and local REST API in one process.
- Polls `GetForegroundWindow` at configurable intervals (default 1000ms).
- Deduplicates: writes only when foreground window identity changes.

Screenshot Collector (Feature 3):

- Runs in a background tokio task alongside the window collector loop.
- Captures every 60s, but only when the user is active (keyboard/mouse input within the last 2 minutes, via `GetLastInputInfo`).
- Uses `xcap` (Windows Graphics Capture API) to capture the primary monitor.
- Resizes to max 640px width and encodes as JPEG.
- Writes files to `data/screenshots/YYYY-MM-DD/HH-MM.jpg`.
- Metadata (timestamp, file path, dimensions, foreground app/window) is stored in the `screenshot_thumbnails` table.
- Checks blocker config before capture — blocked apps/windows are never screenshotted.

Blocker Engine:

- Loads rules from `collector/blocker_config.json` at startup.
- Supports `screenshot` and `text_capture` capture types (forward-looking).
- Matches on `process_name`, `window_title`, or `exe_path_hash`.
- Operators: `equals`, `contains`, `starts_with`.
- Blocked capture attempts are logged to the `blocker_hits` table.

Storage:

- SQLite stores append-only `raw_events`, `window_events`, `screenshot_thumbnails`, and `blocker_hits`.
- Raw event payloads preserve the sampled window snapshot as JSON.
- Screenshot image data lives on the filesystem; SQLite stores only metadata and file paths.
- The open interval is derived at query time and has no `endedAt` until the next focus event.

API:

- `/api/health` — local backend health.
- `/api/window-events` — raw joined focus events.
- `/api/time-events` — interval-shaped records for the WebUI.
- `/api/blockers` — blocker rules and recent hits.
- `/api/screenshots` — screenshot metadata for a given date.
- `/api/screenshot-summary` — aggregated stats (count, hours, top apps).
- `/screenshots/` — static file serving for captured thumbnail images.

Descriptive stats engine:

- Computes row count, total duration, mean, median, standard deviation, min, max, Q1, and Q3.
- Groups durations by application/process.
- Produces display-ready summaries without changing the original records.

Daily Tracking timeline:

- Groups screenshots by hour.
- Shows thumbnail, timestamp, and foreground app/title for each capture.
- Click-to-expand for full-size view.
- Summary bar shows total screenshots, hours covered, and top apps by screenshot count.

UI cards/tables:

- Cards show global metrics at a glance.
- Tables show per-application active duration and event counts.
- Tab bar switches between Statistics (Feature 1) and Daily Tracking (Feature 3) views.
- Both views support sample data fallback and live collector data.
- Validation output should be visible and actionable for bad API payloads or empty collector responses.

Future recorder pipeline:

- Event-driven `SetWinEventHook` can replace or augment window polling.
- Feature 2 (text capture) can use the same blocker engine and session infrastructure.
- A bounded event bus will decouple collectors from storage.
- Derived rollups can be persisted for faster queries.
- Server-Sent Events or WebSocket streaming can push live focus changes.

## Current MVP vs Future Infrastructure

Current WebUI owns:

- REST client for `/api/time-events`, `/api/screenshots`, and `/api/screenshot-summary`.
- Built-in sample fallbacks (feature1 and feature3).
- Descriptive statistics.
- Per-application summaries.
- Daily screenshot timeline.
- Tab-based view switching.
- Static UI rendering.

Current collector/storage owns:

- Windows foreground-window polling.
- Periodic screenshot capture with idle detection.
- Blocker/filter engine with JSON config.
- Raw event creation, including `window_focus` events.
- Screenshot thumbnail generation and file storage.
- Capture status handling for lock screen, UAC, permission errors, and unavailable windows.
- SQLite WAL storage (4 tables).
- Interval generation from raw events.
- Local REST JSON API + static file serving for the WebUI.

Boundary rules:

- The UI should not call Windows APIs directly.
- Browser UI access should use the local `/api` proxy or a future same-origin shell; the collector must not expose permissive CORS for private activity data.
- The WebUI API client should accept recorder query data without knowing how it was captured.
- Raw events and derived rollups should remain reproducible; do not overwrite raw source rows during analysis.
- Privacy-sensitive fields, especially window titles and screenshots, must be optional and redaction-friendly.
- Screenshot files are served from the collector process; the WebUI does not access the filesystem directly.
