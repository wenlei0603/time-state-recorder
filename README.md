# Time State Recorder MVP

Time State Recorder is a local-first, Windows-first project for understanding how time is spent on a computer. The long-term product is a recorder that stores raw activity events locally, derives rollups, and presents useful summaries without sending private activity data to a cloud service.

This repository currently documents and prototypes the first UI-facing slice. The MVP is intentionally smaller than the full recorder: it is a static React/TypeScript WebUI that can load active-window sample events from CSV or a built-in `feature1` dataset, calculate descriptive statistics, and summarize active duration by application.

## Current MVP Scope

The MVP answers one question:

> Given active-window events, what applications were active, for how long, and what are the basic descriptive statistics of those durations?

Included:

- Load active-window records from a user-provided CSV file.
- Load the built-in `feature1` sample dataset for demo and regression checks.
- Validate required fields before analysis.
- Calculate count, mean, median, standard deviation, min, max, quartiles, and total duration.
- Group active duration by application/process and display cards or tables in the WebUI.
- Run fully locally in the browser/dev server.

Not included yet:

- Real Windows API collection.
- SQLite storage, event bus, recorder service, tray app, or background agent.
- Keyboard/input monitoring.
- Screenshot thumbnail capture.
- Cloud sync or multi-device support.

## Feature 1 Definition

Feature 1 is active window/process monitoring.

In the full recorder, Feature 1 will observe foreground-window changes on Windows and write raw `window_focus` events containing timestamp, process name, PID, window title or redacted title, executable hash, and capture status.

In this MVP, Feature 1 is represented by sample/imported rows rather than live collection. The UI treats each row as an already-captured active-window interval or event-derived interval.

## App Architecture

The MVP uses a small one-way flow:

1. Data source: built-in `feature1` sample or user CSV upload.
2. Parser: converts CSV/sample rows into typed event records.
3. Validation: checks required fields, timestamp/duration shape, and rejects unusable rows.
4. Stats engine: computes descriptive statistics and per-application duration summaries.
5. UI: renders summary cards, validation messages, and application tables.

The architecture deliberately mirrors the future recorder, but only the browser-side analysis path is implemented now. Future collectors and storage should feed the same logical data model so the UI can stay mostly unchanged.

## UI Data Model

The WebUI expects active-window records with these logical fields. The CSV parser accepts both camelCase UI names and snake_case recorder-export names where noted.

| Field | Type | Required | Notes |
| --- | --- | --- | --- |
| `started_at` | ISO timestamp/string | Yes | Start time of the active interval. |
| `ended_at` | ISO timestamp/string | Preferred | End time. If absent, `duration_seconds` must exist. |
| `duration_seconds` | number | Preferred | Active duration for this row. Can be derived from start/end. |
| `app` | string | Yes | Application or process display name. |
| `process_name` | string | Optional | Raw process name when distinct from `app`. |
| `window_title` | string | Optional | May be redacted or omitted for privacy. |
| `pid` | number/string | Optional | Process identifier from live collection in future. |
| `capture_status` | string | Optional | Example: `ok`, `permission_denied`, `locked`, `unavailable`. |

Derived analysis objects should not mutate normalized event rows. In the current MVP, the source CSV text remains visible in the editor for review; a future recorder/export layer should preserve full raw row provenance alongside normalized records.

## Local Run and Test Commands

Use the project package manager once the WebUI files are present:

```powershell
npm install
npm run dev
npm test
npm run build
```

If the implementation uses a different package manager, keep equivalent scripts for:

- `dev`: start the local WebUI.
- `test`: run parser/statistics tests.
- `build`: verify the static app compiles.

## Verification Checklist

- Built-in `feature1` sample loads without user input.
- CSV upload accepts valid active-window rows and reports invalid rows clearly.
- Duration calculations are correct when using explicit `duration_seconds`.
- Duration calculations are correct when deriving from `started_at` and `ended_at`.
- Reversed intervals and calendar-invalid ISO timestamps are rejected with row-level errors.
- Descriptive statistics include count, mean, median, standard deviation, min, max, Q1, and Q3.
- Per-application summary totals match the row-level total duration.
- Empty data, malformed CSV, missing app name, and invalid timestamps are handled without crashing.
- `npm test` and `npm run build` pass before review.

## Non-Goals and Risks

Non-goals for this MVP:

- No live recorder or Windows service.
- No background collection, hooks, Raw Input, UI Automation, or screenshot APIs.
- No SQLite database yet.
- No sensitive text capture.
- No productivity classification, AI labeling, or automatic task inference.

Risks:

- Sample data can hide edge cases that live Windows collection will produce, such as permission-denied windows, lock screen gaps, title changes, and rapid focus switches.
- CSV field names may drift unless parser aliases and documentation stay aligned.
- Window titles can contain private information. The MVP should tolerate missing or redacted titles.
- Statistics are only as reliable as interval construction. Future recorder work must define how focus events become intervals.
