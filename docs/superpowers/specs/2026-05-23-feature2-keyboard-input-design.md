# Feature 2: Keyboard Input Tracker — Design

## Overview

Feature 2 captures keyboard activity on Windows and records characters and text segments, enabling work-history reconstruction from typed content. It uses Raw Input API for key events and a WH_GETMESSAGE hook for IME composed text, writing per-event and per-segment records into SQLite and exposing them through REST endpoints and a WebUI panel.

## Capture Architecture

Two sources feed a single in-memory segment buffer:

- **Raw Input (Thread A):** `RegisterRawInputDevices` with `RIDEV_INPUTSINK` for keyboard. Receives `WM_INPUT` messages in a message-only window loop. Extracts virtual key code, scan code, and key-down/up flags. Calls `ToUnicodeEx` to map printable keystrokes to characters. Sends structured key events to an mpsc channel.

- **Message Hook (Thread B):** `SetWindowsHookExW(WH_GETMESSAGE)` intercepts `WM_CHAR` and `WM_IME_CHAR` before target-application dispatch. Reads the composed character from `wParam`. Catches IME output (Chinese, Japanese, etc.) that Raw Input misses. Sends composed characters to the same mpsc channel.

- **Main async task:** Drains the mpsc receiver. Accumulates characters into a segment buffer. On `VK_RETURN` keydown, flushes the segment to SQLite. On 30-second idle timeout, auto-flushes the current segment to prevent data loss.

## Segment Buffer Logic

One active segment at a time, identified by a UUID `segment_id`:

- Printable character arrives → append to `current_text`, increment `key_count`.
- `VK_BACK` arrives → pop last char from `current_text`, increment `backspace_count`.
- `VK_DELETE` arrives → increment `delete_count` (cannot reconstruct exact deleted character without cursor position).
- `VK_RETURN` keydown → flush: write the segment row to `text_segments`, write all buffered input events to `input_events`. New UUID starts the next segment.
- 30-second idle timer → auto-flush (handles app-switch-without-Enter).

Each flush uses a single SQLite transaction for both the segment row and event rows.

## SQLite Schema

### input_events

| Column | Type | Notes |
|---|---|---|
| `id` | INTEGER PK AUTOINCREMENT | |
| `event_ts` | TEXT NOT NULL | RFC 3339 |
| `event_type` | TEXT NOT NULL | `"keydown"` or `"keyup"` |
| `vk_code` | INTEGER NOT NULL | Virtual key code |
| `scan_code` | INTEGER NOT NULL | Hardware scan code |
| `character` | TEXT (nullable) | Mapped char, null for non-printable / IME compose keystrokes |
| `segment_id` | TEXT NOT NULL | FK to text_segments.id |
| `foreground_hwnd` | INTEGER NOT NULL | Foreground window handle at event time |
| `foreground_pid` | INTEGER NOT NULL | Foreground process ID |
| `process_name` | TEXT (nullable) | Resolved process name |
| `window_title` | TEXT (nullable) | Foreground window title |

Indexes: `idx_input_events_ts` on `event_ts`, `idx_input_events_segment` on `segment_id`.

### text_segments

| Column | Type | Notes |
|---|---|---|
| `id` | TEXT PK | UUID |
| `started_at` | TEXT NOT NULL | RFC 3339 |
| `ended_at` | TEXT (nullable) | RFC 3339, null until flushed |
| `text_content` | TEXT NOT NULL | Accumulated printable text after backspace processing |
| `key_count` | INTEGER NOT NULL | Total keydown events in segment |
| `backspace_count` | INTEGER NOT NULL | VK_BACK presses |
| `delete_count` | INTEGER NOT NULL | VK_DELETE presses |
| `foreground_hwnd` | INTEGER NOT NULL | |
| `foreground_pid` | INTEGER NOT NULL | |
| `process_name` | TEXT (nullable) | |
| `window_title` | TEXT (nullable) | |

Indexes: `idx_text_segments_at` on `started_at`.

## Rust Models

All in `collector/src/models.rs`, following existing `#[serde(rename_all = "camelCase")]` convention:

- `InputEvent { id, event_ts, event_type: InputEventType, vk_code, scan_code, character, segment_id, foreground_hwnd, foreground_pid, process_name, window_title }`
- `InputEventType` enum: `KeyDown` / `KeyUp` (serialized as `"keydown"` / `"keyup"` via `#[serde(rename_all = "snake_case")]`)
- `TextSegment { id, started_at, ended_at, text_content, key_count, backspace_count, delete_count, foreground_hwnd, foreground_pid, process_name, window_title }`
- `InputSummary { date, total_events, keydown_count, keyup_count, segment_count, total_chars, last_activity, top_apps }`
- `AppInputCount { process_name, char_count }`

## REST API

### GET /api/input-events?limit=N&segmentId=X

Returns `{ "events": InputEvent[] }`. Optional `segmentId` filter.

### GET /api/input-summary?date=YYYY-MM-DD

Returns `InputSummary` — aggregated stats for a date: total events, keydown/keyup counts, segment count, total characters, last activity timestamp, per-app character breakdown.

### GET /api/text-segments?date=YYYY-MM-DD&limit=N

Returns `{ "segments": TextSegment[] }`, ordered by `started_at DESC`. For browsing full typed text.

All three follow the existing Axum handler pattern: `State<AppState>`, `Query`, lock store, call method, return `Json` or error.

## WebUI — Feature 2 Panel

New tab "Input Activity" in the tab bar (alongside Statistics and Daily Tracking).

Summary cards (reusing existing `.metric` component):
- Total Events
- KeyDown Count
- KeyUp Count
- Segment Count / Total Chars
- Last Activity time

Per-app character bar chart (reusing existing `.bars` pattern from Feature 1).

Text segments table at bottom: columns Time | App | Text Preview (truncated) | Chars. Row click expands to show full text content.

Sample/Live data toggle, same pattern as Feature 1/3. Built-in `feature2Sample` data for offline/demo mode.

## Feature 2B Placeholder

The `text_segments` table and segment buffer are designed for future edit-operation tracking:
- Add `edit_operation` field to `InputEvent` (e.g., `"insert"`, `"delete_before_cursor"`, `"delete_after_cursor"`, `"replace"`, `"paste"`)
- Extend segment buffer to track cursor position and apply edit operations to reconstruct final text
- No schema migration needed for these additions

## Files Changed/Created

| File | Change |
|---|---|
| `collector/Cargo.toml` | Add `crossbeam-channel` dep for mpsc between threads |
| `collector/src/models.rs` | Add `InputEvent`, `InputEventType`, `TextSegment`, `InputSummary`, `AppInputCount` |
| `collector/src/input.rs` | **New** — Raw Input + message hook capture, segment buffer, flush logic |
| `collector/src/lib.rs` | Add `pub mod input;` |
| `collector/src/storage.rs` | Add `input_events` + `text_segments` tables, insert/query methods |
| `collector/src/api.rs` | Add 3 routes, wire input loop into `serve()`, add input-related fields to `AppState` |
| `collector/src/main.rs` | No arg changes needed (reuse existing `serve` command) |
| `src/types.ts` | Add `InputEvent`, `TextSegment`, `InputSummary`, `AppInputCount` types |
| `src/lib/input.ts` | **New** — API client for input endpoints |
| `src/data/feature2Sample.ts` | **New** — sample data |
| `src/InputActivity.tsx` | **New** — Feature 2 panel component |
| `src/App.tsx` | Add "Input Activity" tab |
| `src/styles.css` | Add segment table styles |

## Verification

1. `cargo build` compiles with new deps and modules
2. `cargo test -p tsr-collector` passes (existing + new storage tests)
3. Start collector, type in any app, observe `input_events` and `text_segments` rows in SQLite
4. `curl /api/input-events` returns events; `curl /api/input-summary?date=...` returns stats
5. `curl /api/text-segments?date=...` returns typed text content
6. WebUI shows "Input Activity" tab with summary cards and segment table
7. `npm test` passes (existing + new API client tests)
8. Type Chinese text via IME — composed characters appear in text_segments
9. Type text with backspace corrections — segment text reflects deletions
