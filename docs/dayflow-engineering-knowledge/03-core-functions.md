# Core Functions

## Product Capabilities

The current project implements five core user-facing capabilities:

1. Active window time tracking.
2. Keyboard activity and reconstructed text segments.
3. Periodic screenshot timeline.
4. Local privacy blocker and redacted/raw UI modes.
5. Collector health and local runtime monitoring.

## 1. Active Window Time Tracking

### What It Does

Records foreground application/window changes and derives active-time intervals.

### Implementation Path

```text
GetForegroundWindow
-> WindowSnapshot
-> raw_events + window_events
-> build_time_events_with_lifecycle()
-> /api/time-events
-> Dashboard and TimelineView
```

### Core Code

- `collector/src/window.rs:26` gets the foreground window handle, process id, process name, executable path hash, and title.
- `collector/src/api.rs:218` polls the foreground window.
- `collector/src/storage.rs:332` writes `raw_events` and `window_events`.
- `collector/src/interval.rs:9` derives `TimeEvent` intervals.
- `src/lib/api.ts:5` fetches time events.
- `src/Dashboard.tsx:25` renders daily active metrics.
- `src/TimelineView.tsx:21` renders event or hourly timeline rows.

### Behavioral Rules

- Polling defaults to 1000 ms.
- Repeated samples are deduplicated by `(hwnd, pid, window_title)`.
- A window interval ends at the next focus event in the same session.
- Lifecycle cut events can end a window interval earlier.
- The current open interval may have no end.

### Current Limitations

- It is polling-based, not event-driven `SetWinEventHook`.
- Browser URL capture is not implemented.
- The title is stored raw when available.
- Rapid focus switches shorter than the poll interval can be missed.

## 2. Lifecycle-Aware Timeline

### What It Does

Prevents service gaps and selected system states from being counted as continuous active app time.

### Implementation Path

```text
session_start / session_stop / collector_gap
-> lifecycle_events
-> build_time_events_with_lifecycle()
-> /api/time-events
-> TimelineView and Dashboard
```

### Core Code

- `collector/src/storage.rs:146` creates capture sessions.
- `collector/src/storage.rs:197` closes stale sessions.
- `collector/src/storage.rs:260` writes lifecycle events.
- `collector/src/interval.rs:106` defines lifecycle types that cut active intervals.
- `collector/src/interval.rs:119` defines lifecycle start/end pairs.
- `src/lib/uiModel.ts:85` filters lifecycle/window layers.

### Behavioral Rules

- Active window intervals do not bridge session ids.
- `collector_gap` is written at the last known event timestamp for stale sessions.
- Lock, suspend, idle start, capture unavailable, collector gap, session stop, and session disconnect cut active time.
- Lock/unlock, suspend/resume, idle start/end, and disconnect/reconnect can appear as system intervals.

### Current Limitations

- Real Windows messages for lock, suspend, and session changes are not yet wired.
- Most lifecycle facts are service lifecycle facts, not complete OS lifecycle telemetry.

## 3. Keyboard Activity And Text Segments

### What It Does

Captures keyboard activity with Raw Input, reconstructs approximate text segments, and tracks correction counts.

### Implementation Path

```text
Raw Input WM_INPUT
-> InputSignal
-> SegmentBuffer
-> input_events + text_segments
-> /api/input-summary + /api/text-segments
-> InputActivity and Dashboard input insights
```

### Core Code

- `collector/src/input.rs:250` sets up the message-only window.
- `collector/src/input.rs:301` registers Raw Input keyboard device.
- `collector/src/input.rs:341` handles `WM_INPUT`.
- `collector/src/input.rs:395` maps keydown events through `ToUnicodeEx`.
- `collector/src/input.rs:65` ingests input signals into a segment buffer.
- `collector/src/storage.rs:586` writes text segment and input event rows.
- `src/lib/input.ts:25` fetches input summary.
- `src/lib/uiModel.ts:185` summarizes input insights.
- `src/InputActivity.tsx:30` renders input activity.

### Behavioral Rules

- Enter flushes the current segment.
- A 30-second idle timeout flushes the current segment.
- Backspace pops one character from the local segment buffer and increments `backspaceCount`.
- Delete increments `deleteCount`.
- Text content is shown only in raw privacy mode.

### Current Limitations

- `text_segments.text_content` is plaintext on disk.
- Input tables do not currently include `session_id`.
- Chinese IME committed text is not reliably captured by Raw Input plus `ToUnicodeEx`.
- Paste, selection replace, and true cursor-aware Delete semantics are not implemented.
- `text_capture` blocker rules exist in config but are not enforced in the input path.

## 4. Screenshot Tracking

### What It Does

Captures minute-level thumbnail evidence while the user is active and the foreground window is not blocked.

### Implementation Path

```text
timer every 60s
-> idle_seconds()
-> foreground WindowSnapshot
-> BlockerEngine
-> capture_thumbnail()
-> data/screenshots/YYYY-MM-DD/HH-MM.jpg
-> screenshot_thumbnails
-> /api/screenshot-summary + /api/screenshots
-> DailyTracking
```

### Core Code

- `collector/src/api.rs:276` starts the screenshot loop.
- `collector/src/screenshot.rs:33` checks idle time.
- `collector/src/blocker.rs:28` checks blocker rules.
- `collector/src/screenshot.rs:10` captures and resizes the primary monitor.
- `collector/src/storage.rs:475` writes metadata.
- `src/lib/screenshots.ts:24` fetches summary.
- `src/DailyTracking.tsx:34` renders daily tracking.

### Behavioral Rules

- Default interval is 60 seconds.
- Screenshots are skipped after 120 seconds idle.
- Thumbnail width is capped at 640 px.
- Redacted UI mode hides images and raw screenshot rows.
- Raw UI mode can load and render `/screenshots/<filePath>`.

### Current Limitations

- Primary monitor only.
- JPEG files are stored unencrypted.
- The JPEG quality parameter is currently unused.
- No OCR, object detection, semantic scene labels, or high-resolution archive.

## 5. Privacy And Blocker System

### What It Does

Applies configurable rules before screenshot capture and records blocked attempts.

### Implementation Path

```text
blocker_config.json
-> BlockerEngine
-> screenshot loop match
-> blocker_hits
-> /api/blockers
```

### Core Code

- `collector/src/blocker.rs:13` loads config.
- `collector/src/blocker.rs:51` applies rule matching.
- `collector/src/api.rs:299` calls screenshot blocker.
- `collector/src/storage.rs:426` writes blocker hits.

### Behavioral Rules

- Fields: `process_name`, `window_title`, `exe_path_hash`.
- Operators: `equals`, `contains`, `starts_with`.
- Matching screenshot rules skip capture.

### Current Limitations

- Blocker rules are not a complete redaction policy.
- Input/text capture does not currently use blocker enforcement.
- Screenshot files already written are not retroactively cleaned by blocker changes.

## 6. WebUI Review Workspace

### Dashboard

`src/Dashboard.tsx` shows:

- active time;
- lifecycle time;
- focus block count;
- context switches;
- correction ratio;
- top app and recent timeline.

Data source:

- `TimeEvent[]`
- `TextSegment[]`
- `src/lib/uiModel.ts` summaries.

### Timeline

`src/TimelineView.tsx` shows:

- event-level rows or hourly buckets;
- active/lifecycle split;
- layer visibility for windows and lifecycle.

Data source:

- `TimeEvent[]`
- `buildTimelineItems()`
- `buildHourlyTimelineItems()`

### Daily Tracking

`src/DailyTracking.tsx` shows:

- screenshot count;
- hours covered;
- top apps by screenshot count;
- grouped screenshot rows by hour;
- expandable raw screenshot image in raw mode.

Data source:

- `ScreenshotSummary`
- `ScreenshotMeta[]`

### Input Activity

`src/InputActivity.tsx` shows:

- total input events;
- keydown/keyup counts;
- segment count;
- character count;
- last activity;
- characters by application;
- raw or redacted text segment table.

Data source:

- `InputSummary`
- `TextSegment[]`

### Collector Monitor

`src/CollectorMonitor.tsx` shows:

- collector status;
- uptime;
- subsystem health;
- DB row counts;
- last errors.

Data source:

- `GET /api/health`, polled every 5 seconds.

## Feature Completeness Snapshot

| Capability | Current Status | Dayflow Reproduction Implication |
| --- | --- | --- |
| Local collector | Implemented | Good baseline for local-first Dayflow clone |
| Window timeline | Implemented, polling-based | Useful but may miss rapid switches |
| Lifecycle-aware active time | Partially implemented | Service gaps handled; real Windows lock/suspend pending |
| Keyboard/input activity | Implemented approximation | Needs IME/text-edit upgrade for Chinese-heavy use |
| Screenshot timeline | Implemented thumbnail path | Good visual evidence baseline |
| Privacy modes in UI | Implemented frontend gate | Needs backend policy enforcement for robust privacy |
| Blocker rules | Implemented for screenshots | Text blocker gap must be closed before claiming full privacy |
| Dayflow query/chat | Not implemented | Future core reproduction area |
| Evidence drawer | Partially implemented in Today Flow Board | Raw screenshot thumbnails and event evidence exist; deeper source drilldown is still future work |
| Weekly review/export | Not implemented | Future workflow layer |
