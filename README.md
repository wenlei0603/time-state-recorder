# Time State Recorder MVP

Time State Recorder is a local-first, Windows-first project for understanding how time is spent on a computer. It records local activity signals, stores them in SQLite, derives rollups, and presents summaries in a React WebUI without sending private activity data to a cloud service.

The current repository contains a Rust/SQLite Windows collector, a local JSON API, a blocker/filter engine, periodic screenshot thumbnails, Raw Input keyboard capture with text segment reconstruction, and a React/TypeScript WebUI for statistics and daily timeline views.

## Current MVP Scope

The MVP answers three practical questions:

1. Which applications and windows were active, for how long, and with what descriptive statistics?
2. What did the screen look like minute-by-minute throughout the day?
3. What keyboard activity and reconstructed text segments were captured by the local collector?

Included:

- **Feature 1** - Active window/process monitoring via polling `GetForegroundWindow`.
- **Feature 2** - Keyboard input capture via the Raw Input API with text segment reconstruction.
- **Feature 3** - Periodic screenshot thumbnails with idle detection and file-system storage.
- **Blocker system** - JSON-configurable app/window/title blocklist used before screenshot capture.
- Local SQLite storage for raw events, window events, screenshots, input events, text segments, and blocker hits.
- Local REST JSON endpoints served by `tsr-collector`.
- Static screenshot file serving from `/screenshots/`.
- React WebUI views for time statistics, input activity, collector health, and daily screenshot tracking.
- Built-in sample data fallback when the collector is offline.

Not included yet:

- `SetWinEventHook` event-driven foreground-window collection.
- Tray app or installed Windows service.
- WH_GETMESSAGE hook for IME/composed Chinese or Japanese text capture.
- OCR or high-resolution screenshot archival.
- Cloud sync or multi-device support.

## Feature Definitions

### Feature 1: Window Activity

The collector polls `GetForegroundWindow` at a configurable interval, defaulting to 1000ms. It writes `window_focus` events containing timestamp, process name, PID, window title or redacted title, executable hash, and capture status.

The WebUI turns raw focus changes into active intervals and computes count, mean, median, standard deviation, min, max, quartiles, total duration, and per-application summaries.

### Feature 2: Input Activity

The input collector registers a Raw Input keyboard device (`RIDEV_INPUTSINK`) on a dedicated message-only window thread. It receives `WM_INPUT` messages system-wide, maps virtual-key codes to characters via `ToUnicodeEx` using the foreground window keyboard layout, and buffers keydown/keyup events into text segments.

A segment is flushed when Enter (`VK_RETURN`) is pressed or after a 30-second idle timeout.

Each segment records:

- `textContent` - accumulated printable characters with backspace/delete tracking.
- `keyCount`, `backspaceCount`, `deleteCount` - edit statistics.
- `startedAt` and `endedAt` - segment time boundaries.
- Foreground window metadata at capture time.

Known limitation: Raw Input alone cannot capture composed IME characters. Feature 2B should add a WH_GETMESSAGE hook DLL for pre-edit/composed text.

### Feature 3: Screenshot Tracking

The screenshot collector captures a thumbnail of the primary monitor every 60 seconds, but only when the user is active. Activity is detected with `GetLastInputInfo`; screenshots are skipped after 2 minutes of idle time.

Each capture is:

- Resized to max 640px width while preserving aspect ratio.
- Encoded as JPEG and stored under `data/screenshots/YYYY-MM-DD/HH-MM.jpg`.
- Stored in SQLite with timestamp, dimensions, foreground app/window, and capture status.
- Checked against blocker rules before capture.

The Daily Tracking view shows the day's screenshots grouped by hour, with app/title context and per-app screenshot counts.

## Blocker System

The blocker engine provides a privacy-aware filter before screenshot capture. Future text redaction and hook-based capture should reuse the same rule model.

Rules live in `collector/blocker_config.json`:

```json
{
  "version": 1,
  "rules": [
    { "capture_type": "screenshot", "field": "process_name", "operator": "equals", "value": "Taskmgr.exe" },
    { "capture_type": "screenshot", "field": "window_title", "operator": "contains", "value": "Banking" }
  ]
}
```

Supported fields: `process_name`, `window_title`, `exe_path_hash`.

Supported operators: `equals`, `contains`, `starts_with`.

Blocked attempts are logged to `blocker_hits` and exposed through `GET /api/blockers`.

## Architecture

1. The Windows collector starts with `cargo run -p tsr-collector -- serve` or `npm run collector`.
2. The collector samples foreground-window state, captures keyboard input, and periodically captures screenshots.
3. SQLite stores `raw_events`, `window_events`, `screenshot_thumbnails`, `input_events`, `text_segments`, and `blocker_hits`.
4. The local REST API exposes raw focus events, interval-shaped time events, input events, text segments, summaries, screenshots, blocker hits, and collector health.
5. Screenshot files are served as static files under `/screenshots/`.
6. The WebUI fetches local API data and renders statistics, collector status, input activity, and screenshot timelines.

The collector intentionally starts with polling because it is easy to verify and keeps the WebUI contract stable. A future collector can add `SetWinEventHook` without changing the API shape.

## Windows Toolchain

The repository is configured for `x86_64-pc-windows-gnullvm` in `.cargo/config.toml` and `rust-toolchain.toml`. This avoids a Visual Studio / Windows SDK requirement, but it requires LLVM-MinGW MSVCRT tools on `PATH`.

Install the required local toolchain:

```powershell
winget install --id MartinStorsjo.LLVM-MinGW.MSVCRT --exact --accept-source-agreements --accept-package-agreements
npm install
```

If the current shell cannot find `cargo` or `x86_64-w64-mingw32-clang`, refresh `PATH`:

```powershell
$mingwBin = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe" -Recurse -Filter x86_64-w64-mingw32-clang.exe | Select-Object -First 1 -ExpandProperty DirectoryName
$env:PATH = "$mingwBin;$env:USERPROFILE\.cargo\bin;$env:PATH"
```

Common build failures:

- `kernel32.lib` or other `.lib` files missing: the build is using an MSVC target. Use the configured `x86_64-pc-windows-gnullvm` target.
- `x86_64-w64-mingw32-clang` not found: install LLVM-MinGW MSVCRT and refresh `PATH`.
- Missing `target/.../tsr-collector.exe`: rebuild with `cargo build -p tsr-collector`; `cargo clean` removes the debug executable.

## One-Click Windows Start

For day-to-day use, double-click:

- `Start Time State Recorder.bat`

The launcher starts the collector on `127.0.0.1:4317`, serves the built WebUI on `127.0.0.1:5173`, waits until both are ready, then opens the browser. If the collector binary or WebUI build is missing, it attempts to build them first. Runtime logs are written under `logs/`.

To stop the app processes, double-click:

- `Stop Time State Recorder.bat`

The launcher uses a lightweight Node static server (`scripts/web-server.mjs`) for the WebUI and proxies `/api` and `/screenshots` to the collector. It does not use the Vite dev server for the user-facing start path.

## Manual Local Run

Start the collector API:

```powershell
cargo build -p tsr-collector
cargo run -p tsr-collector -- serve --db data/local.sqlite3 --addr 127.0.0.1:4317
```

Build and serve the WebUI:

```powershell
npm run build
npm run serve:app
```

Open:

- WebUI: `http://127.0.0.1:5173`
- Collector health: `http://127.0.0.1:4317/api/health`

## API Endpoints

| Method | Path | Query Params | Description |
| --- | --- | --- | --- |
| `GET` | `/api/health` | None | Collector health, subsystem states, uptime, and DB row counts |
| `GET` | `/api/window-events` | `?limit=N` | Raw joined window-focus events |
| `GET` | `/api/time-events` | `?limit=N` | Interval-shaped time events for statistics |
| `GET` | `/api/blockers` | `?limit=N` | Blocker rules and recent hits |
| `GET` | `/api/screenshots` | `?date=YYYY-MM-DD&limit=N` | Screenshot metadata for a given date |
| `GET` | `/api/screenshot-summary` | `?date=YYYY-MM-DD` | Aggregated screenshot stats |
| `GET` | `/api/input-events` | `?limit=N&segmentId=S` | Raw keyboard input events |
| `GET` | `/api/input-summary` | `?date=YYYY-MM-DD` | Aggregated input stats |
| `GET` | `/api/text-segments` | `?date=YYYY-MM-DD&limit=N` | Text segments with content and edit stats |

Static files: `/screenshots/YYYY-MM-DD/HH-MM.jpg` serves captured thumbnail images.

## WebUI Views

- **Statistics** - Descriptive stats grid, application time chart, collector health, and event rows table.
- **Input Activity** - Input summary cards, per-app character chart, and expandable text segments.
- **Daily Tracking** - Screenshot summary bar and timeline grouped by hour.

The WebUI supports switching between built-in sample data and live collector data.

## Data Model Notes

Input events use this JSON shape:

```json
{
  "id": 1,
  "eventTs": "2026-05-23T09:00:01Z",
  "eventType": "keydown",
  "vkCode": 70,
  "scanCode": 33,
  "character": "f",
  "segmentId": "segment-uuid",
  "foregroundHwnd": 1111,
  "foregroundPid": 100,
  "processName": "Code",
  "windowTitle": "main.rs"
}
```

`eventType` is serialized as `keydown` or `keyup`, matching the SQLite values and the TypeScript client contract.

Text segments use `textContent`, `keyCount`, `backspaceCount`, `deleteCount`, `startedAt`, `endedAt`, and foreground window metadata. `totalChars` in `/api/input-summary` is the SQLite `LENGTH(text_content)` total for matching segments.

Derived analysis objects should not mutate normalized event rows. SQLite preserves raw collector payloads and the WebUI receives normalized JSON from the local API.

## Verification

Run these before review or push:

```powershell
cargo fmt --all
cargo test -p tsr-collector -- --nocapture
cargo build -p tsr-collector
npm test -- --run
npm run build
```

Manual checklist:

- `sample-once` returns current foreground process/window JSON.
- `serve` exposes `/api/health`, `/api/time-events`, `/api/blockers`, `/api/screenshots`, `/api/screenshot-summary`, `/api/input-events`, `/api/input-summary`, and `/api/text-segments`.
- `/api/health` returns full `CollectorHealth` JSON with uptime, subsystem states, and DB row counts.
- WebUI CollectorMonitor shows offline state when the collector is unreachable and live status when it is running.
- Screenshot capture writes JPEG files under `data/screenshots/YYYY-MM-DD/HH-MM.jpg` when active.
- Screenshots are skipped after the idle threshold.
- Blocked apps/windows are not captured and are logged in `blocker_hits`.
- WebUI loads collector data and falls back to sample datasets when offline.
- Empty API responses, malformed JSON payloads, missing fields, and invalid timestamps do not crash the UI.

## Non-Goals and Risks

Non-goals for this MVP:

- No installed Windows service or tray app.
- No event-driven `SetWinEventHook` collector yet.
- No WH_GETMESSAGE hook for IME/composed character capture yet.
- No OCR or high-resolution screenshot archival.
- No productivity classification, AI labeling, or automatic task inference.
- No cloud sync or multi-device support.

Risks:

- Sample fallback data can hide edge cases from live Windows collection, such as permission-denied windows, lock screen gaps, title changes, and rapid focus switches.
- API response fields may drift unless collector models, WebUI validation, and documentation stay aligned.
- Window titles, screenshots, and keyboard text content can contain private information. The blocker system is config-based and not automatic.
- Keyboard text capture stores plaintext segments on disk; encryption and automatic redaction are future concerns.
- Screenshots are stored as JPEG files on disk with no encryption; access control relies on local filesystem permissions.
- Local browser access must stay same-origin or proxied; permissive CORS would expose private activity data to arbitrary websites.
- Statistics are only as reliable as interval construction. Future recorder work must define how focus events become intervals.
- `GetLastInputInfo` only reports input for the current user session and can be fooled by continuous-input devices.
