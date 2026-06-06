# Time State Recorder

Time State Recorder is a local-first Windows workday memory layer. It captures desktop activity signals, turns them into a reviewable time flow, and produces evidence-backed daily summaries that can be archived into a personal knowledge system.

It is not a timer, employee-monitoring tool, or generic productivity scorecard. It is designed for a single user who wants to reconstruct what happened during a workday: which projects appeared, when focus shifted, what evidence exists, and what can be safely summarized for later reflection.

## Product Pitch

Knowledge work disappears into tabs, chats, editors, documents, and meetings. By the end of the day, the user often knows they were busy but cannot reliably reconstruct the work path.

Time State Recorder creates a private, local record of that path. It observes active windows, input activity, screenshots, visual summaries, and lifecycle events, then organizes them into a Today view, 5-hour reports, a Daily Brief, and a Notion-ready archive payload.

The result is a personal operating log: enough evidence to review the day, restore context, and write a useful diary entry without manually tracking every task.

See [docs/product-pitch.md](docs/product-pitch.md) for a reusable pitch, demo script, and positioning notes.

## What It Does

- Captures active Windows process/window focus and turns raw changes into time intervals.
- Captures keyboard input activity and reconstructs local text segments where Raw Input can observe them.
- Captures periodic screenshot thumbnails and high-resolution screenshot evidence when the user is active.
- Applies configurable blocker rules before screenshot capture.
- Stores raw activity facts in local SQLite and screenshot files on the local filesystem.
- Exposes a local REST API on `127.0.0.1:4317`.
- Serves a React WebUI on `127.0.0.1:5173`.
- Presents a Today Flow Board, timeline, activity review, screenshot evidence, collector health, and input activity views.
- Generates 5-hour insight reports and a Daily Brief from local activity metrics and optional model-backed analysis.
- Exposes `/api/notion/daily-archive` so Notion Principles OS can archive a selected day's summary without parsing the UI.
- Provides a repo-local smoke command that writes a sample Notion archive JSON artifact for other agents.

## Privacy Model

The default product boundary is local capture and local storage:

- Activity data is stored in SQLite under `data/`.
- Screenshot files are stored under `data/screenshots/` and `data/high-res-screenshots/`.
- The WebUI talks to the local collector through same-origin/proxied local routes.
- Screenshot retention is treated as a temporary local cache, currently 30 days by default.
- Blocker rules live in `collector/blocker_config.json` and are checked before screenshot capture.

Optional AI analysis can send selected screenshots or structured summaries to a configured model provider. Notion integration is intentionally read-only from this repo: Time State Recorder exposes local JSON/Markdown payloads, while the separate Notion Principles OS tooling performs Notion writes and verification.

## Current Capabilities

| Area | Status |
| --- | --- |
| Window activity | Active foreground-window polling through Windows APIs |
| Lifecycle events | Session start/stop, lock/unlock, service stop, stale-session handling |
| Input activity | Raw Input keyboard events, text segment reconstruction, input summaries |
| Screenshot evidence | Thumbnail and high-resolution screenshot metadata, local file serving |
| Privacy controls | Blocker rules, raw/redacted UI modes, same-origin local access |
| Daily review | Today Flow Board, timeline, activity buckets, evidence drawer |
| Visual analysis | Optional screenshot/window summaries and structured visual labels |
| 5-hour reports | Project-oriented work trajectory summaries |
| Daily Brief | Daily stats, hourly metrics, comparison, and action trajectory |
| Notion archive | Read-only daily archive API plus smoke artifact for downstream agents |
| Packaging | Windows x64 release ZIP with collector, WebUI, scripts, and README |

## Quick Start

For day-to-day use on Windows, build or download a release package, then run:

```powershell
.\scripts\start-user.ps1
```

Or double-click:

- `Start Time State Recorder.bat`

The launcher starts:

- Collector API: `http://127.0.0.1:4317`
- WebUI: `http://127.0.0.1:5173`

Stop it with:

```powershell
.\scripts\stop-user.ps1
```

Or double-click:

- `Stop Time State Recorder.bat`

## Manual Development Run

Install dependencies:

```powershell
npm install
```

Start the collector API:

```powershell
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

## Notion Daily Archive Smoke Test

Run this before changing the Notion Principles OS archive job:

```powershell
npm run smoke:notion-daily-archive
```

The command starts an in-memory collector API with sample data, calls `/api/notion/daily-archive`, asserts the required Markdown sections and same-day 5-hour report filtering, and writes:

```text
reports/notion-daily-archive-smoke.json
```

That artifact is a contract fixture other agents can consume when wiring the Notion Daily Diary append and verification flow.

## Architecture

```text
Windows signals
  -> Rust collector
  -> SQLite + local screenshot files
  -> local REST API
  -> React WebUI
  -> optional daily archive payloads for Notion Principles OS
```

The collector owns capture, storage, aggregation, and local API contracts. The WebUI owns review and interaction. External systems, such as Notion Principles OS, consume read-only payloads and perform their own write/readback verification outside this repo.

Core components:

- `collector/src/api.rs` - Axum routes and response builders.
- `collector/src/storage.rs` - SQLite schema and query/aggregation methods.
- `collector/src/insights.rs` - local and MiniMax-backed insight/report generation.
- `collector/src/visual_analysis.rs` - screenshot visual analysis providers.
- `collector/src/input.rs` - Raw Input capture and text segments.
- `src/` - React/TypeScript WebUI.
- `docs/api/` - API contracts and downstream integration notes.
- `scripts/start-user.ps1` and `scripts/stop-user.ps1` - local runtime entrypoints.

## API Surface

| Method | Path | Purpose |
| --- | --- | --- |
| `GET` | `/api/health` | Collector health, subsystem status, DB stats, retention state |
| `GET` | `/api/window-events` | Raw joined foreground-window events |
| `GET` | `/api/lifecycle-events` | Session, lock/unlock, service lifecycle events |
| `GET` | `/api/time-events` | Interval-shaped activity and lifecycle rows |
| `GET` | `/api/activity-buckets` | Date-scoped activity buckets for review |
| `GET` | `/api/blockers` | Blocker config and recent blocked capture attempts |
| `GET` | `/api/screenshots` | Thumbnail screenshot metadata by date |
| `GET` | `/api/high-res-screenshots` | High-resolution screenshot metadata by date |
| `GET` | `/api/screenshot-summary` | Screenshot coverage and app counts |
| `GET` | `/api/visual-summaries` | Per-screenshot visual summaries |
| `GET` | `/api/visual-observations` | High-resolution visual observations |
| `GET` | `/api/visual-window-summaries` | Structured 5-minute window summaries |
| `GET` | `/api/insight-reports` | 5-hour reports and related report records |
| `GET` | `/api/daily-brief` | Daily stats, hourly metrics, reports, and action trajectory |
| `GET` | `/api/notion/daily-archive` | Notion-ready daily archive JSON and Markdown |
| `POST` | `/api/daily-brief/generate` | Generate or refresh a daily brief |
| `POST` | `/api/screenshots/{id}/analyze` | Analyze one screenshot |
| `GET` | `/api/input-events` | Raw keyboard input events |
| `GET` | `/api/input-summary` | Aggregated input activity by date |
| `GET` | `/api/text-segments` | Reconstructed text segments |
| `POST` | `/api/shutdown` | Graceful collector shutdown |

Static evidence routes:

- `/screenshots/...`
- `/high-res-screenshots/...`

## WebUI Views

- **Today Flow Board** - Overview-first daily flow with evidence drawers and raw/redacted modes.
- **Dashboard** - Summary statistics and current collector status.
- **Timeline** - Time intervals and lifecycle-aware activity rows.
- **Activity Review** - Activity buckets and reviewable work segments.
- **Daily Tracking** - Screenshot coverage and day-level evidence.
- **Input Activity** - Keyboard/input summaries and text segments when raw mode is enabled.
- **Review Notes / Daily Brief** - Model-backed or computed summaries for the selected date.

## Toolchain

The repository targets `x86_64-pc-windows-gnullvm` through `.cargo/config.toml` and `rust-toolchain.toml`. This avoids a Visual Studio build dependency but requires LLVM-MinGW MSVCRT tools.

Install the local toolchain:

```powershell
winget install --id MartinStorsjo.LLVM-MinGW.MSVCRT --exact --accept-source-agreements --accept-package-agreements
npm install
```

If the shell cannot find Cargo or the MinGW compiler, refresh `PATH`:

```powershell
$mingwBin = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\MartinStorsjo.LLVM-MinGW.MSVCRT_Microsoft.Winget.Source_8wekyb3d8bbwe" -Recurse -Filter x86_64-w64-mingw32-clang.exe | Select-Object -First 1 -ExpandProperty DirectoryName
$env:PATH = "$mingwBin;$env:USERPROFILE\.cargo\bin;$env:PATH"
```

## Verification

Run these before review, packaging, or release:

```powershell
cargo fmt --all
cargo test -p tsr-collector
npm test
npm run smoke:notion-daily-archive
npm run build
cargo build -p tsr-collector --release
```

Package a Windows release ZIP:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\package-release.ps1 -Version 1.4.0
```

## Non-Goals

- No employee monitoring or team surveillance workflow.
- No automatic productivity scoring.
- No direct Notion writes from the collector.
- No cloud sync or multi-device account model.
- No installed Windows service or tray app yet.
- No full IME/composed Chinese or Japanese text capture yet.
- No guarantee that every off-screen, offline, or permission-denied activity is observed.

## Documentation

- [Product pitch](docs/product-pitch.md)
- [Notion Daily Archive API](docs/api/notion-daily-archive.md)
- [Next query API design](docs/api/next-query-api.md)
- [Windows collector runbook](docs/runbooks/windows-collector.md)
- [Dayflow engineering knowledge](docs/dayflow-engineering-knowledge/00-index.md)
