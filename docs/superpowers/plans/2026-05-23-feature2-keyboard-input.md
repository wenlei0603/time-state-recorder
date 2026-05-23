# Feature 2: Keyboard Input Tracker — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Capture keyboard activity via Raw Input API, accumulate text in Enter-delimited segments, store in SQLite, and expose through REST API and WebUI panel.

**Architecture:** A Windows Raw Input message-only window captures keyboard events on a dedicated thread. Events flow through a tokio unbounded mpsc channel to an async drain task that maintains a segment buffer (accumulating text, handling backspace/delete) and flushes to SQLite on Enter or idle timeout. REST endpoints expose events, segments, and summary stats. The WebUI adds a third tab ("Input Activity") with summary cards, per-app bar chart, and a text segments table.

**Tech Stack:** Rust + windows-sys (Raw Input, GetKeyboardState, ToUnicodeEx), tokio::sync::mpsc, React + TypeScript, same Axum/Vitest patterns as Features 1 & 3.

**IME note:** For this MVP, `ToUnicodeEx` maps characters for non-IME keyboard input. Chinese/Japanese IME composed characters require a `WH_GETMESSAGE` global hook DLL, which will be added in a follow-up (Feature 2B). The `InputSignal` enum and segment buffer are designed to accept IME-composed chars when that source is connected.

---

### Task 1: Add input models to collector/src/models.rs

**Files:**
- Modify: `collector/src/models.rs` (append after line 125)

- [ ] **Step 1: Add InputEventType, InputEvent, TextSegment, InputSummary, AppInputCount**

Append to `collector/src/models.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputEventType {
    KeyDown,
    KeyUp,
}

impl InputEventType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::KeyDown => "keydown",
            Self::KeyUp => "keyup",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "keyup" => Self::KeyUp,
            _ => Self::KeyDown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputEvent {
    pub id: i64,
    pub event_ts: DateTime<Utc>,
    pub event_type: InputEventType,
    pub vk_code: u32,
    pub scan_code: u32,
    pub character: Option<String>,
    pub segment_id: String,
    pub foreground_hwnd: i64,
    pub foreground_pid: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSegment {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub text_content: String,
    pub key_count: usize,
    pub backspace_count: usize,
    pub delete_count: usize,
    pub foreground_hwnd: i64,
    pub foreground_pid: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSummary {
    pub date: String,
    pub total_events: usize,
    pub keydown_count: usize,
    pub keyup_count: usize,
    pub segment_count: usize,
    pub total_chars: usize,
    pub last_activity: Option<DateTime<Utc>>,
    pub top_apps: Vec<AppInputCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInputCount {
    pub process_name: String,
    pub char_count: usize,
}
```

- [ ] **Step 2: Build check**

Run: `cargo check -p tsr-collector 2>&1`
Expected: compiles (models are additive, no callers yet).

- [ ] **Step 3: Commit**

```bash
git add collector/src/models.rs
git commit -m "feat: add input event, segment, and summary models"
```

---

### Task 2: Add input schema + store methods to collector/src/storage.rs

**Files:**
- Modify: `collector/src/storage.rs`

- [ ] **Step 1: Add schema DDL in `init()`**

In `collector/src/storage.rs`, inside the `init()` method's `execute_batch` string, append after the `screenshot_thumbnails` block:

```sql
CREATE TABLE IF NOT EXISTS input_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  event_ts TEXT NOT NULL,
  event_type TEXT NOT NULL,
  vk_code INTEGER NOT NULL,
  scan_code INTEGER NOT NULL,
  character TEXT,
  segment_id TEXT NOT NULL,
  foreground_hwnd INTEGER NOT NULL,
  foreground_pid INTEGER NOT NULL,
  process_name TEXT,
  window_title TEXT
);
CREATE INDEX IF NOT EXISTS idx_input_events_ts ON input_events(event_ts);
CREATE INDEX IF NOT EXISTS idx_input_events_segment ON input_events(segment_id);

CREATE TABLE IF NOT EXISTS text_segments (
  id TEXT PRIMARY KEY,
  started_at TEXT NOT NULL,
  ended_at TEXT,
  text_content TEXT NOT NULL,
  key_count INTEGER NOT NULL DEFAULT 0,
  backspace_count INTEGER NOT NULL DEFAULT 0,
  delete_count INTEGER NOT NULL DEFAULT 0,
  foreground_hwnd INTEGER NOT NULL,
  foreground_pid INTEGER NOT NULL,
  process_name TEXT,
  window_title TEXT
);
CREATE INDEX IF NOT EXISTS idx_text_segments_at ON text_segments(started_at);
```

- [ ] **Step 2: Add store methods**

Append to `impl Store` (before the closing `}` at line 371):

```rust
pub fn insert_input_segment(
    &mut self,
    segment: &crate::models::TextSegment,
    events: &[crate::models::InputEvent],
) -> Result<()> {
    let tx = self.conn.transaction()?;

    tx.execute(
        r#"
        INSERT INTO text_segments
          (id, started_at, ended_at, text_content, key_count, backspace_count, delete_count,
           foreground_hwnd, foreground_pid, process_name, window_title)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            segment.id,
            segment.started_at.to_rfc3339(),
            segment.ended_at.map(|t| t.to_rfc3339()),
            segment.text_content,
            segment.key_count,
            segment.backspace_count,
            segment.delete_count,
            segment.foreground_hwnd,
            segment.foreground_pid,
            segment.process_name,
            segment.window_title,
        ],
    )?;

    for event in events {
        tx.execute(
            r#"
            INSERT INTO input_events
              (event_ts, event_type, vk_code, scan_code, character, segment_id,
               foreground_hwnd, foreground_pid, process_name, window_title)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                event.event_ts.to_rfc3339(),
                event.event_type.as_str(),
                event.vk_code,
                event.scan_code,
                event.character,
                event.segment_id,
                event.foreground_hwnd,
                event.foreground_pid,
                event.process_name,
                event.window_title,
            ],
        )?;
    }

    tx.commit()?;
    Ok(())
}

pub fn list_input_events(
    &self,
    limit: usize,
    segment_id: Option<&str>,
) -> Result<Vec<crate::models::InputEvent>> {
    let query = if segment_id.is_some() {
        "SELECT id, event_ts, event_type, vk_code, scan_code, character, segment_id,
                foreground_hwnd, foreground_pid, process_name, window_title
         FROM input_events
         WHERE segment_id = ?2
         ORDER BY event_ts ASC, id ASC
         LIMIT ?1"
    } else {
        "SELECT id, event_ts, event_type, vk_code, scan_code, character, segment_id,
                foreground_hwnd, foreground_pid, process_name, window_title
         FROM input_events
         ORDER BY event_ts DESC, id DESC
         LIMIT ?1"
    };

    let mut stmt = self.conn.prepare(query)?;

    let rows = if let Some(sid) = segment_id {
        stmt.query_map(params![limit as i64, sid], |row| {
            let event_ts: String = row.get(1)?;
            let event_type: String = row.get(2)?;
            Ok(InputEvent {
                id: row.get(0)?,
                event_ts: parse_ts(&event_ts)?,
                event_type: InputEventType::from_db(&event_type),
                vk_code: row.get(3)?,
                scan_code: row.get(4)?,
                character: row.get(5)?,
                segment_id: row.get(6)?,
                foreground_hwnd: row.get(7)?,
                foreground_pid: row.get(8)?,
                process_name: row.get(9)?,
                window_title: row.get(10)?,
            })
        })?
    } else {
        stmt.query_map(params![limit as i64], |row| {
            let event_ts: String = row.get(1)?;
            let event_type: String = row.get(2)?;
            Ok(InputEvent {
                id: row.get(0)?,
                event_ts: parse_ts(&event_ts)?,
                event_type: InputEventType::from_db(&event_type),
                vk_code: row.get(3)?,
                scan_code: row.get(4)?,
                character: row.get(5)?,
                segment_id: row.get(6)?,
                foreground_hwnd: row.get(7)?,
                foreground_pid: row.get(8)?,
                process_name: row.get(9)?,
                window_title: row.get(10)?,
            })
        })?
    };

    let mut events = Vec::new();
    for row in rows {
        events.push(row?);
    }
    Ok(events)
}

pub fn list_text_segments(
    &self,
    date: &str,
    limit: usize,
) -> Result<Vec<crate::models::TextSegment>> {
    let pattern = format!("{date}%");
    let mut stmt = self.conn.prepare(
        r#"
        SELECT id, started_at, ended_at, text_content, key_count, backspace_count, delete_count,
               foreground_hwnd, foreground_pid, process_name, window_title
        FROM text_segments
        WHERE started_at LIKE ?1
        ORDER BY started_at DESC
        LIMIT ?2
        "#,
    )?;

    let rows = stmt.query_map(params![&pattern, limit as i64], |row| {
        let started_at: String = row.get(1)?;
        let ended_at: Option<String> = row.get(2)?;
        Ok(crate::models::TextSegment {
            id: row.get(0)?,
            started_at: crate::storage::parse_ts(&started_at)?,
            ended_at: match ended_at {
                Some(s) => Some(crate::storage::parse_ts(&s)?),
                None => None,
            },
            text_content: row.get(3)?,
            key_count: row.get(4)?,
            backspace_count: row.get(5)?,
            delete_count: row.get(6)?,
            foreground_hwnd: row.get(7)?,
            foreground_pid: row.get(8)?,
            process_name: row.get(9)?,
            window_title: row.get(10)?,
        })
    })?;

    let mut segments = Vec::new();
    for row in rows {
        segments.push(row?);
    }
    Ok(segments)
}

pub fn get_input_summary(&self, date: &str) -> Result<crate::models::InputSummary> {
    let pattern = format!("{date}%");

    let total: usize = self.conn.query_row(
        "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1",
        params![&pattern],
        |row| row.get(0),
    ).unwrap_or(0);

    let keydown: usize = self.conn.query_row(
        "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1 AND event_type = 'keydown'",
        params![&pattern],
        |row| row.get(0),
    ).unwrap_or(0);

    let keyup: usize = self.conn.query_row(
        "SELECT COUNT(*) FROM input_events WHERE event_ts LIKE ?1 AND event_type = 'keyup'",
        params![&pattern],
        |row| row.get(0),
    ).unwrap_or(0);

    let segments: usize = self.conn.query_row(
        "SELECT COUNT(*) FROM text_segments WHERE started_at LIKE ?1",
        params![&pattern],
        |row| row.get(0),
    ).unwrap_or(0);

    let total_chars: usize = self.conn.query_row(
        "SELECT COALESCE(SUM(LENGTH(text_content)), 0) FROM text_segments WHERE started_at LIKE ?1",
        params![&pattern],
        |row| row.get(0),
    ).unwrap_or(0);

    let last_activity: Option<String> = self.conn.query_row(
        "SELECT event_ts FROM input_events WHERE event_ts LIKE ?1 ORDER BY event_ts DESC LIMIT 1",
        params![&pattern],
        |row| row.get(0),
    ).ok();

    let mut stmt = self.conn.prepare(
        r#"
        SELECT process_name, SUM(LENGTH(text_content)) as total_chars
        FROM text_segments
        WHERE started_at LIKE ?1 AND process_name IS NOT NULL
        GROUP BY process_name
        ORDER BY total_chars DESC
        LIMIT 10
        "#,
    )?;

    let top_apps: Vec<crate::models::AppInputCount> = stmt
        .query_map(params![&pattern], |row| {
            Ok(crate::models::AppInputCount {
                process_name: row.get(0)?,
                char_count: row.get::<_, i64>(1)? as usize,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(crate::models::InputSummary {
        date: date.to_string(),
        total_events: total,
        keydown_count: keydown,
        keyup_count: keyup,
        segment_count: segments,
        total_chars,
        last_activity: last_activity.and_then(|s| parse_ts(&s).ok()),
        top_apps,
    })
}
```

- [ ] **Step 3: Make `parse_ts` pub(crate)**

Change the `parse_ts` function on line 374 from `fn parse_ts` to `pub(crate) fn parse_ts` so `list_text_segments` and `get_input_summary` can reference it.

- [ ] **Step 4: Build check**

Run: `cargo check -p tsr-collector 2>&1`
Expected: compiles. New methods are dead-code warned but structs/calls type-check.

- [ ] **Step 5: Commit**

```bash
git add collector/src/storage.rs
git commit -m "feat: add input_events and text_segments tables with query methods"
```

---

### Task 3: Create collector/src/input.rs — capture loop + segment buffer

**Files:**
- Create: `collector/src/input.rs`

- [ ] **Step 1: Write the full input module**

Create `collector/src/input.rs`:

```rust
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    models::{InputEvent, InputEventType, TextSegment},
    storage::Store,
    window::sample_foreground_window,
};

const VK_BACK: u16 = 0x08;
const VK_RETURN: u16 = 0x0D;
const VK_DELETE: u16 = 0x2E;

#[derive(Debug, Clone)]
pub(crate) struct InputSignal {
    pub event_ts: chrono::DateTime<Utc>,
    pub is_keydown: bool,
    pub vk_code: u16,
    pub scan_code: u16,
    pub character: Option<String>,
    pub foreground_hwnd: i64,
    pub foreground_pid: u32,
    pub process_name: Option<String>,
    pub window_title: Option<String>,
}

struct SegmentBuffer {
    segment_id: String,
    current_text: String,
    events: Vec<InputSignal>,
    key_count: usize,
    backspace_count: usize,
    delete_count: usize,
    started_at: chrono::DateTime<Utc>,
    last_activity: chrono::DateTime<Utc>,
    foreground_hwnd: i64,
    foreground_pid: u32,
    process_name: Option<String>,
    window_title: Option<String>,
}

impl SegmentBuffer {
    fn new(signal: &InputSignal) -> Self {
        Self {
            segment_id: Uuid::new_v4().to_string(),
            current_text: String::new(),
            events: Vec::new(),
            key_count: 0,
            backspace_count: 0,
            delete_count: 0,
            started_at: signal.event_ts,
            last_activity: signal.event_ts,
            foreground_hwnd: signal.foreground_hwnd,
            foreground_pid: signal.foreground_pid,
            process_name: signal.process_name.clone(),
            window_title: signal.window_title.clone(),
        }
    }

    fn ingest(&mut self, signal: InputSignal) -> bool {
        self.last_activity = signal.event_ts;
        self.foreground_hwnd = signal.foreground_hwnd;
        self.foreground_pid = signal.foreground_pid;
        self.process_name = signal.process_name.clone();
        self.window_title = signal.window_title.clone();

        let vk = signal.vk_code;
        let is_enter = vk == VK_RETURN && signal.is_keydown;

        self.events.push(signal);

        if is_enter {
            self.current_text.push('\n');
            return true;
        }

        if signal.is_keydown {
            self.key_count += 1;

            if vk == VK_BACK {
                self.current_text.pop();
                self.backspace_count += 1;
            } else if vk == VK_DELETE {
                self.delete_count += 1;
            } else if let Some(ref ch) = signal.character {
                if !ch.is_empty() {
                    self.current_text.push_str(ch);
                }
            }
        }

        false
    }

    fn flush(self) -> (TextSegment, Vec<InputEvent>) {
        let segment = TextSegment {
            id: self.segment_id,
            started_at: self.started_at,
            ended_at: Some(self.last_activity),
            text_content: self.current_text,
            key_count: self.key_count,
            backspace_count: self.backspace_count,
            delete_count: self.delete_count,
            foreground_hwnd: self.foreground_hwnd,
            foreground_pid: self.foreground_pid,
            process_name: self.process_name,
            window_title: self.window_title,
        };

        let events: Vec<InputEvent> = self
            .events
            .into_iter()
            .enumerate()
            .map(|(i, s)| InputEvent {
                id: 0,
                event_ts: s.event_ts,
                event_type: if s.is_keydown {
                    InputEventType::KeyDown
                } else {
                    InputEventType::KeyUp
                },
                vk_code: s.vk_code as u32,
                scan_code: s.scan_code as u32,
                character: s.character,
                segment_id: segment.id.clone(),
                foreground_hwnd: s.foreground_hwnd,
                foreground_pid: s.foreground_pid,
                process_name: s.process_name,
                window_title: s.window_title,
            })
            .collect();

        (segment, events)
    }
}

pub fn spawn_input_collector(store: Arc<Mutex<Store>>) {
    let (tx, mut rx) = mpsc::unbounded_channel::<InputSignal>();

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

#[cfg(windows)]
mod platform {
    use std::sync::OnceLock;

    use chrono::Utc;
    use tokio::sync::mpsc;
    use windows_sys::Win32::{
        UI::{
            Input::KeyboardAndMouse::{
                GetRawInputData, RAWINPUT, RAWINPUTHEADER, RAWKEYBOARD,
                RIDEV_INPUTSINK, RID_INPUT, RIM_TYPEKEYBOARD, RI_KEY_BREAK,
                RegisterRawInputDevices, RAWINPUTDEVICE, HID_USAGE_PAGE_GENERIC,
                HID_USAGE_GENERIC_KEYBOARD,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                GetForegroundWindow, GetKeyboardState, GetMessageW, GetWindowThreadProcessId,
                PostQuitMessage, RegisterClassW, SetWindowLongPtrW, GetWindowLongPtrW,
                TranslateMessage, CS_HREDRAW, CS_VREDRAW, HWND, HWND_MESSAGE, LPARAM,
                LRESULT, MSG, WNDCLASSW, WPARAM, WM_DESTROY, WM_INPUT, GWLP_USERDATA,
            },
        },
        System::LibraryLoader::GetModuleHandleW,
        Foundation::HINSTANCE,
    };

    use super::InputSignal;
    use crate::window::sample_foreground_window;

    static SENDER: OnceLock<mpsc::UnboundedSender<InputSignal>> = OnceLock::new();

    struct WindowState {
        sender: mpsc::UnboundedSender<InputSignal>,
        key_state: [u8; 256],
    }

    pub(super) fn run_raw_input_loop(tx: mpsc::UnboundedSender<InputSignal>) {
        SENDER.set(tx.clone()).ok();

        unsafe {
            let hmod = GetModuleHandleW(std::ptr::null());

            let class_name = encode_wide("TSRInputWindow\0");
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hmod,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: class_name.as_ptr(),
            };
            RegisterClassW(&wc);

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                encode_wide("TSR Input\0").as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                hmod,
                std::ptr::null_mut(),
            );

            if hwnd.is_null() {
                eprintln!("input collector: failed to create message window");
                return;
            }

            let mut state = Box::new(WindowState {
                sender: tx,
                key_state: [0u8; 256],
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut *state as *mut WindowState as isize);

            let mut rid = RAWINPUTDEVICE {
                usUsagePage: HID_USAGE_PAGE_GENERIC,
                usUsage: HID_USAGE_GENERIC_KEYBOARD,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            };

            if RegisterRawInputDevices(
                &rid,
                1,
                std::mem::size_of::<RAWINPUTDEVICE>() as u32,
            ) == 0
            {
                eprintln!("input collector: RegisterRawInputDevices failed");
                DestroyWindow(hwnd);
                return;
            }

            let mut msg = MSG {
                hwnd: HWND(std::ptr::null_mut()),
                message: 0,
                wParam: WPARAM(0),
                lParam: LPARAM(0),
                time: 0,
                pt: windows_sys::Foundation::POINT { x: 0, y: 0 },
            };

            while GetMessageW(
                &mut msg,
                HWND(std::ptr::null_mut()),
                0,
                0,
            ) > 0
            {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Clean up
            let _boxed = Box::from_raw(
                GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState,
            );
            DestroyWindow(hwnd);
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_INPUT {
            let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
            if state_ptr.is_null() {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }

            let state = &mut *state_ptr;

            let mut size: u32 = 0;
            if GetRawInputData(
                windows_sys::Win32::UI::Input::KeyboardAndMouse::HRAWINPUT(lparam.0),
                RID_INPUT,
                std::ptr::null_mut(),
                &mut size,
                std::mem::size_of::<RAWINPUTHEADER>() as u32,
            ) != 0
            {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }

            let mut buf: Vec<u8> = vec![0; size as usize];
            let copied = GetRawInputData(
                windows_sys::Win32::UI::Input::KeyboardAndMouse::HRAWINPUT(lparam.0),
                RID_INPUT,
                buf.as_mut_ptr() as *mut _,
                &mut size,
                std::mem::size_of::<RAWINPUTHEADER>() as u32,
            );

            if copied as u32 != size {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }

            let raw = buf.as_ptr() as *const RAWINPUT;
            if (*raw).header.dwType == RIM_TYPEKEYBOARD {
                let kb = &(*raw).data.keyboard;

                let vk = kb.VKey as u16;
                let scan = kb.MakeCode as u16;
                let is_keydown = kb.Flags & RI_KEY_BREAK == 0;
                let is_e0 = kb.Flags & 0x02 != 0; // RI_KEY_E0

                // Update key state for ToUnicodeEx
                if is_keydown {
                    state.key_state[vk as usize] = 0x80;
                } else {
                    state.key_state[vk as usize] = 0;
                }
                // E0 scan codes set bit 7
                if is_e0 {
                    state.key_state[scan as usize] = 0x80;
                }

                // Map to character on keydown
                let character = if is_keydown {
                    let mut buf = [0u16; 4];
                    let result = unsafe {
                        let keyboard_layout = {
                            let fg = GetForegroundWindow();
                            let mut tid = 0u32;
                            GetWindowThreadProcessId(fg, &mut tid);
                            windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardLayout(tid)
                        };

                        // We need ToUnicodeEx from windows-sys
                        // Use the key_state we maintain
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::ToUnicodeEx(
                            vk as u32,
                            scan as u32,
                            state.key_state.as_ptr(),
                            buf.as_mut_ptr(),
                            buf.len() as i32,
                            0,
                            keyboard_layout,
                        )
                    };

                    if result > 0 {
                        Some(String::from_utf16_lossy(&buf[..result as usize]))
                    } else {
                        None
                    }
                } else {
                    None
                };

                let foreground = sample_foreground_window().ok();
                let (fh, fp, pn, wt) = match foreground {
                    Some(ref snap) => (
                        snap.hwnd,
                        snap.pid,
                        Some(snap.process_name.clone()),
                        snap.window_title.clone(),
                    ),
                    None => (0, 0, None, None),
                };

                let signal = InputSignal {
                    event_ts: Utc::now(),
                    is_keydown,
                    vk_code: vk,
                    scan_code: scan,
                    character,
                    foreground_hwnd: fh,
                    foreground_pid: fp,
                    process_name: pn,
                    window_title: wt,
                };

                let _ = state.sender.send(signal);
            }
        } else if msg == WM_DESTROY {
            PostQuitMessage(0);
            return 0;
        }

        DefWindowProcW(hwnd, msg, wparam, lparam)
    }

    fn encode_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use tokio::sync::mpsc;
    use super::InputSignal;

    pub(super) fn run_raw_input_loop(_tx: mpsc::UnboundedSender<InputSignal>) {
        eprintln!("input collector: not supported on this platform");
    }
}
```

- [ ] **Step 2: Build check**

Run: `cargo check -p tsr-collector 2>&1`
Expected: compiles. Platform-gated module should resolve on Windows.

- [ ] **Step 3: Commit**

```bash
git add collector/src/input.rs
git commit -m "feat: add Raw Input keyboard collector with segment buffer"
```

---

### Task 4: Wire input endpoints into collector/src/api.rs

**Files:**
- Modify: `collector/src/api.rs`

- [ ] **Step 1: Add imports and response structs**

In `collector/src/api.rs`, add to existing imports (after line 30):

```rust
use crate::input;
```

Add after the `ScreenshotsResponse` struct (line 73):

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InputEventsResponse {
    events: Vec<crate::models::InputEvent>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TextSegmentsResponse {
    segments: Vec<crate::models::TextSegment>,
}
```

- [ ] **Step 2: Add routes to `router_from_state()`**

In `router_from_state()`, add three new routes to the `Router::new()` chain (after the screenshot-summary route):

```rust
.route("/api/input-events", get(input_events))
.route("/api/input-summary", get(input_summary))
.route("/api/text-segments", get(text_segments))
```

- [ ] **Step 3: Spawn input collector in `serve()`**

In the `serve()` function, after `spawn_screenshot_loop(state.clone(), session_id);`, add:

```rust
input::spawn_input_collector(state.store.clone());
```

- [ ] **Step 4: Add handler functions**

Append to `collector/src/api.rs` (before the `internal_error` function):

```rust
#[derive(Debug, Deserialize)]
struct InputEventsQuery {
    limit: Option<usize>,
    #[serde(rename = "segmentId")]
    segment_id: Option<String>,
}

async fn input_events(
    State(state): State<AppState>,
    Query(query): Query<InputEventsQuery>,
) -> impl IntoResponse {
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_input_events(limit, query.segment_id.as_deref()) {
        Ok(events) => Json(InputEventsResponse { events }).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn input_summary(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date = query.date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.get_input_summary(&date) {
        Ok(summary) => Json(summary).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn text_segments(
    State(state): State<AppState>,
    Query(query): Query<DateQuery>,
) -> impl IntoResponse {
    let date = query.date.unwrap_or_else(|| Utc::now().format("%Y-%m-%d").to_string());
    let limit = query.limit.unwrap_or(500).min(5_000);
    let store = match state.store.lock() {
        Ok(store) => store,
        Err(_) => return internal_error("store lock poisoned"),
    };

    match store.list_text_segments(&date, limit) {
        Ok(segments) => Json(TextSegmentsResponse { segments }).into_response(),
        Err(err) => internal_error(err),
    }
}
```

- [ ] **Step 5: Build check**

Run: `cargo check -p tsr-collector 2>&1`
Expected: compiles with new routes and handler functions.

- [ ] **Step 6: Commit**

```bash
git add collector/src/api.rs
git commit -m "feat: add /api/input-events, /api/input-summary, /api/text-segments endpoints"
```

---

### Task 5: Register input module in collector/src/lib.rs

**Files:**
- Modify: `collector/src/lib.rs`

- [ ] **Step 1: Add module declaration**

Add after `pub mod blocker;`:

```rust
pub mod input;
```

- [ ] **Step 2: Build check**

Run: `cargo check -p tsr-collector 2>&1`
Expected: compiles.

- [ ] **Step 3: Commit**

```bash
git add collector/src/lib.rs
git commit -m "feat: register input module"
```

---

### Task 6: Add TypeScript types for input data

**Files:**
- Modify: `src/types.ts`

- [ ] **Step 1: Append input types**

Append to `src/types.ts`:

```typescript
export type InputEvent = {
  id: number;
  eventTs: string;
  eventType: "keydown" | "keyup";
  vkCode: number;
  scanCode: number;
  character?: string;
  segmentId: string;
  foregroundHwnd: number;
  foregroundPid: number;
  processName?: string;
  windowTitle?: string;
};

export type TextSegment = {
  id: string;
  startedAt: string;
  endedAt?: string;
  textContent: string;
  keyCount: number;
  backspaceCount: number;
  deleteCount: number;
  foregroundHwnd: number;
  foregroundPid: number;
  processName?: string;
  windowTitle?: string;
};

export type InputSummary = {
  date: string;
  totalEvents: number;
  keydownCount: number;
  keyupCount: number;
  segmentCount: number;
  totalChars: number;
  lastActivity?: string;
  topApps: AppInputCount[];
};

export type AppInputCount = {
  processName: string;
  charCount: number;
};
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors (types are additive, no callers yet).

- [ ] **Step 3: Commit**

```bash
git add src/types.ts
git commit -m "feat: add InputEvent, TextSegment, InputSummary TypeScript types"
```

---

### Task 7: Create API client src/lib/input.ts

**Files:**
- Create: `src/lib/input.ts`

- [ ] **Step 1: Write the input API client**

Create `src/lib/input.ts`:

```typescript
import type { InputEvent, InputSummary, TextSegment } from "../types";

type Fetcher = (
  input: string,
) => Promise<Pick<Response, "ok" | "status" | "statusText" | "json">>;

export async function fetchInputEvents(
  fetcher: Fetcher = fetch,
): Promise<InputEvent[]> {
  const response = await fetcher("/api/input-events");
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.events)) {
    throw new Error("Collector API returned an invalid input-events response");
  }

  return body.events.map(toInputEvent);
}

export async function fetchInputSummary(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<InputSummary> {
  const response = await fetcher(
    `/api/input-summary?date=${encodeURIComponent(date)}`,
  );
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body)) {
    throw new Error(
      "Collector API returned an invalid input-summary response",
    );
  }

  return {
    date: readString(body, "date"),
    totalEvents: readNumber(body, "totalEvents"),
    keydownCount: readNumber(body, "keydownCount"),
    keyupCount: readNumber(body, "keyupCount"),
    segmentCount: readNumber(body, "segmentCount"),
    totalChars: readNumber(body, "totalChars"),
    lastActivity: readOptionalString(body, "lastActivity"),
    topApps: Array.isArray(body.topApps)
      ? body.topApps.map((item: unknown) => {
          if (!isRecord(item)) {
            throw new Error("topApps row is not a record");
          }
          return {
            processName: readString(item, "processName"),
            charCount: readNumber(item, "charCount"),
          };
        })
      : [],
  };
}

export async function fetchTextSegments(
  date: string,
  fetcher: Fetcher = fetch,
): Promise<TextSegment[]> {
  const response = await fetcher(
    `/api/text-segments?date=${encodeURIComponent(date)}`,
  );
  if (!response.ok) {
    throw new Error(
      `Collector API failed: ${response.status} ${response.statusText}`.trim(),
    );
  }

  const body: unknown = await response.json();
  if (!isRecord(body) || !Array.isArray(body.segments)) {
    throw new Error(
      "Collector API returned an invalid text-segments response",
    );
  }

  return body.segments.map(toTextSegment);
}

function toInputEvent(value: unknown): InputEvent {
  if (!isRecord(value)) {
    throw new Error("API returned an invalid input event row");
  }
  return {
    id: readNumber(value, "id"),
    eventTs: readString(value, "eventTs"),
    eventType: readString(value, "eventType") as "keydown" | "keyup",
    vkCode: readNumber(value, "vkCode"),
    scanCode: readNumber(value, "scanCode"),
    character: readOptionalString(value, "character"),
    segmentId: readString(value, "segmentId"),
    foregroundHwnd: readNumber(value, "foregroundHwnd"),
    foregroundPid: readNumber(value, "foregroundPid"),
    processName: readOptionalString(value, "processName"),
    windowTitle: readOptionalString(value, "windowTitle"),
  };
}

function toTextSegment(value: unknown): TextSegment {
  if (!isRecord(value)) {
    throw new Error("API returned an invalid text segment row");
  }
  return {
    id: readString(value, "id"),
    startedAt: readString(value, "startedAt"),
    endedAt: readOptionalString(value, "endedAt"),
    textContent: readString(value, "textContent"),
    keyCount: readNumber(value, "keyCount"),
    backspaceCount: readNumber(value, "backspaceCount"),
    deleteCount: readNumber(value, "deleteCount"),
    foregroundHwnd: readNumber(value, "foregroundHwnd"),
    foregroundPid: readNumber(value, "foregroundPid"),
    processName: readOptionalString(value, "processName"),
    windowTitle: readOptionalString(value, "windowTitle"),
  };
}

function readString(record: Record<string, unknown>, key: string): string {
  const value = record[key];
  if (typeof value !== "string") {
    throw new Error(`API row is missing ${key}`);
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
    throw new Error(`API row has invalid ${key}`);
  }
  return value;
}

function readNumber(record: Record<string, unknown>, key: string): number {
  const value = record[key];
  if (typeof value !== "number") {
    throw new Error(`API row is missing ${key}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib/input.ts
git commit -m "feat: add input API client (fetchInputEvents, fetchInputSummary, fetchTextSegments)"
```

---

### Task 8: Create sample data src/data/feature2Sample.ts

**Files:**
- Create: `src/data/feature2Sample.ts`

- [ ] **Step 1: Write sample data**

Create `src/data/feature2Sample.ts`:

```typescript
import type { InputSummary, TextSegment } from "../types";

export const feature2SampleSummary: InputSummary = {
  date: "2026-05-23",
  totalEvents: 4821,
  keydownCount: 2410,
  keyupCount: 2411,
  segmentCount: 87,
  totalChars: 3420,
  lastActivity: "2026-05-23T17:30:00Z",
  topApps: [
    { processName: "Code", charCount: 2100 },
    { processName: "WindowsTerminal", charCount: 890 },
    { processName: "Obsidian", charCount: 310 },
    { processName: "chrome", charCount: 120 },
  ],
};

export const feature2SampleSegments: TextSegment[] = [
  {
    id: "seg-1",
    startedAt: "2026-05-23T09:00:00Z",
    endedAt: "2026-05-23T09:00:05Z",
    textContent: "fn main() {\n",
    keyCount: 12,
    backspaceCount: 2,
    deleteCount: 0,
    foregroundHwnd: 1111,
    foregroundPid: 100,
    processName: "Code",
    windowTitle: "main.rs",
  },
  {
    id: "seg-2",
    startedAt: "2026-05-23T09:00:06Z",
    endedAt: "2026-05-23T09:00:12Z",
    textContent: '    println!("Hello, world!");\n',
    keyCount: 30,
    backspaceCount: 1,
    deleteCount: 0,
    foregroundHwnd: 1111,
    foregroundPid: 100,
    processName: "Code",
    windowTitle: "main.rs",
  },
  {
    id: "seg-3",
    startedAt: "2026-05-23T09:05:00Z",
    endedAt: "2026-05-23T09:05:03Z",
    textContent: "cargo run\n",
    keyCount: 10,
    backspaceCount: 0,
    deleteCount: 0,
    foregroundHwnd: 2222,
    foregroundPid: 200,
    processName: "WindowsTerminal",
    windowTitle: "PowerShell",
  },
  {
    id: "seg-4",
    startedAt: "2026-05-23T09:10:00Z",
    endedAt: "2026-05-23T09:10:06Z",
    textContent: "git push origin main\n",
    keyCount: 21,
    backspaceCount: 0,
    deleteCount: 0,
    foregroundHwnd: 2222,
    foregroundPid: 200,
    processName: "WindowsTerminal",
    windowTitle: "PowerShell",
  },
  {
    id: "seg-5",
    startedAt: "2026-05-23T10:00:00Z",
    endedAt: "2026-05-23T10:00:08Z",
    textContent: "# Project Notes\n\n## Feature 2 Design\n- Keyboard capture via Raw Input\n",
    keyCount: 57,
    backspaceCount: 3,
    deleteCount: 1,
    foregroundHwnd: 3333,
    foregroundPid: 300,
    processName: "Obsidian",
    windowTitle: "Project notes",
  },
  {
    id: "seg-6",
    startedAt: "2026-05-23T10:15:00Z",
    endedAt: "2026-05-23T10:15:04Z",
    textContent: "function handleClick() {\n",
    keyCount: 25,
    backspaceCount: 2,
    deleteCount: 0,
    foregroundHwnd: 1111,
    foregroundPid: 100,
    processName: "Code",
    windowTitle: "App.tsx",
  },
];
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/data/feature2Sample.ts
git commit -m "feat: add Feature 2 sample segments and summary data"
```

---

### Task 9: Create InputActivity.tsx component

**Files:**
- Create: `src/InputActivity.tsx`

- [ ] **Step 1: Write the InputActivity component**

Create `src/InputActivity.tsx`:

```typescript
import { Keyboard, MousePointerClick } from "lucide-react";
import { useState } from "react";
import {
  feature2SampleSegments,
  feature2SampleSummary,
} from "./data/feature2Sample";
import { fetchInputSummary, fetchTextSegments } from "./lib/input";
import type { InputSummary, TextSegment } from "./types";

type DataSource = "sample" | "live";

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "Invalid";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function formatChars(n: number): string {
  if (n < 1000) return String(n);
  if (n < 10000) return `${(n / 1000).toFixed(1)}k`;
  return `${Math.round(n / 1000)}k`;
}

export function InputActivity({ date }: { date: string }) {
  const [segments, setSegments] = useState<TextSegment[]>(
    feature2SampleSegments,
  );
  const [summary, setSummary] = useState<InputSummary>(
    feature2SampleSummary,
  );
  const [source, setSource] = useState<DataSource>("sample");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);

  async function loadLive() {
    setLoading(true);
    setError(null);
    try {
      const [seg, sum] = await Promise.all([
        fetchTextSegments(date),
        fetchInputSummary(date),
      ]);
      setSegments(seg);
      setSummary(sum);
      setSource("live");
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }

  function loadSample() {
    setSegments(feature2SampleSegments);
    setSummary(feature2SampleSummary);
    setSource("sample");
    setError(null);
  }

  const topAppList = summary.topApps
    .slice(0, 3)
    .map((a) => `${a.processName} (${formatChars(a.charCount)})`)
    .join(" · ");

  const largest = Math.max(
    ...summary.topApps.map((a) => a.charCount),
    1,
  );

  return (
    <section className="dailyTracking">
      <div className="dailyHeader">
        <div>
          <h2>Input Activity</h2>
          <p className="dailyDate">{date}</p>
        </div>
        <div className="actions">
          <button type="button" onClick={loadSample}>
            Sample
          </button>
          <button
            type="button"
            onClick={() => void loadLive()}
            disabled={loading}
          >
            <Keyboard aria-hidden="true" size={18} />
            <span>{loading ? "Loading..." : "Live Data"}</span>
          </button>
        </div>
      </div>

      <div className="summaryBar">
        <div className="summaryStat">
          <Keyboard aria-hidden="true" size={16} />
          <span>
            <strong>{formatChars(summary.totalEvents)}</strong> total events
          </span>
        </div>
        <div className="summaryStat">
          <span>
            KeyDown: <strong>{formatChars(summary.keydownCount)}</strong>
          </span>
        </div>
        <div className="summaryStat">
          <span>
            KeyUp: <strong>{formatChars(summary.keyupCount)}</strong>
          </span>
        </div>
        <div className="summaryStat">
          <MousePointerClick aria-hidden="true" size={16} />
          <span>
            <strong>{summary.segmentCount}</strong> segments ·{" "}
            <strong>{formatChars(summary.totalChars)}</strong> chars
          </span>
        </div>
        {summary.lastActivity && (
          <div className="summaryStat">
            <span>
              Last: <strong>{formatTime(summary.lastActivity)}</strong>
            </span>
          </div>
        )}
      </div>

      {error && (
        <p className="errors" role="status">
          {error}
        </p>
      )}

      {source === "sample" && (
        <p className="sampleNotice">
          Showing sample data. Click "Live Data" when the collector is running.
        </p>
      )}

      <div className="panel" style={{ marginTop: 16 }}>
        <div className="panelHeader">
          <Keyboard aria-hidden="true" size={20} />
          <h2>Per-App Characters</h2>
        </div>
        <div className="bars">
          {summary.topApps.map((item) => (
            <div className="barRow" key={item.processName}>
              <div className="barLabel">
                <span>{item.processName}</span>
                <strong>{formatChars(item.charCount)}</strong>
              </div>
              <div className="barTrack" aria-hidden="true">
                <div
                  className="barFill"
                  style={{
                    width: `${(item.charCount / largest) * 100}%`,
                  }}
                />
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="panel tablePanel">
        <div className="panelHeader">
          <Keyboard aria-hidden="true" size={20} />
          <h2>Text Segments</h2>
        </div>
        <div className="tableWrap">
          <table>
            <thead>
              <tr>
                <th>Time</th>
                <th>App</th>
                <th>Preview</th>
                <th>Chars</th>
                <th>Keys</th>
              </tr>
            </thead>
            <tbody>
              {segments.map((seg) => (
                <tr
                  key={seg.id}
                  onClick={() =>
                    setExpanded(expanded === seg.id ? null : seg.id)
                  }
                  onKeyDown={(e) => {
                    if (e.key === "Enter" || e.key === " ") {
                      e.preventDefault();
                      setExpanded(expanded === seg.id ? null : seg.id);
                    }
                  }}
                  tabIndex={0}
                  role="button"
                  style={{ cursor: "pointer" }}
                >
                  <td>{formatTime(seg.startedAt)}</td>
                  <td>{seg.processName || "Unknown"}</td>
                  <td
                    style={{
                      maxWidth: 360,
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: expanded === seg.id ? "normal" : "nowrap",
                    }}
                  >
                    {seg.textContent.slice(0, expanded === seg.id ? undefined : 80)}
                  </td>
                  <td>{seg.textContent.length}</td>
                  <td>
                    {seg.keyCount}
                    {seg.backspaceCount > 0 &&
                      ` · ${seg.backspaceCount}⌫`}
                    {seg.deleteCount > 0 && ` · ${seg.deleteCount}⌦`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
}
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/InputActivity.tsx
git commit -m "feat: add InputActivity panel with summary cards and segments table"
```

---

### Task 10: Add "Input Activity" tab to App.tsx

**Files:**
- Modify: `src/App.tsx`

- [ ] **Step 1: Import InputActivity and add tab**

In `src/App.tsx`:

Change the import line (line 1) to include `Keyboard`:
```typescript
import { BarChart3, Camera, Keyboard, RefreshCw, RotateCcw, Server, TableProperties } from "lucide-react";
```

Add import after line 3 (`DailyTracking`):
```typescript
import { InputActivity } from "./InputActivity";
```

Change the `ViewMode` type (line 15) from `"stats" | "daily"` to:
```typescript
type ViewMode = "stats" | "daily" | "input";
```

In the tab bar (after the Daily Tracking tab button), add:
```typescript
<button
  type="button"
  className={`tab ${viewMode === "input" ? "active" : ""}`}
  onClick={() => setViewMode("input")}
>
  <Keyboard aria-hidden="true" size={16} />
  <span>Input Activity</span>
</button>
```

In the conditional rendering (line 90), change to:
```typescript
{viewMode === "daily" ? (
  <DailyTracking date={today} />
) : viewMode === "input" ? (
  <InputActivity date={today} />
) : (
```

- [ ] **Step 2: Type-check**

Run: `npx tsc --noEmit`
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/App.tsx
git commit -m "feat: add Input Activity tab to App"
```

---

### Task 11: Add CSS styles for input segments table

**Files:**
- Modify: `src/styles.css`

- [ ] **Step 1: Add segment text styles**

Append to `src/styles.css`:

```css
/* Segment text in expandable rows */
td pre {
  margin: 0;
  font-family: ui-monospace, SFMono-Regular, Consolas, "Liberation Mono", monospace;
  font-size: 0.84rem;
  white-space: pre-wrap;
  word-break: break-word;
  max-width: 400px;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/styles.css
git commit -m "style: add segment text formatting styles"
```

---

### Task 12: Write tests

**Files:**
- Create: `src/lib/input.test.ts`
- Modify: `collector/tests/api_tests.rs`

- [ ] **Step 1: Write TypeScript API client tests**

Create `src/lib/input.test.ts`:

```typescript
import { describe, expect, it, vi } from "vitest";
import { fetchInputEvents, fetchInputSummary, fetchTextSegments } from "./input";

describe("fetchInputEvents", () => {
  it("loads input events from the collector REST API", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        events: [
          {
            id: 1,
            eventTs: "2026-05-23T09:00:00Z",
            eventType: "keydown",
            vkCode: 65,
            scanCode: 30,
            character: "a",
            segmentId: "seg-1",
            foregroundHwnd: 1111,
            foregroundPid: 100,
            processName: "Code",
            windowTitle: "main.rs",
          },
        ],
      }),
    });

    await expect(fetchInputEvents(fetcher)).resolves.toEqual([
      {
        id: 1,
        eventTs: "2026-05-23T09:00:00Z",
        eventType: "keydown",
        vkCode: 65,
        scanCode: 30,
        character: "a",
        segmentId: "seg-1",
        foregroundHwnd: 1111,
        foregroundPid: 100,
        processName: "Code",
        windowTitle: "main.rs",
      },
    ]);
    expect(fetcher).toHaveBeenCalledWith("/api/input-events");
  });

  it("reports API failures", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
    });

    await expect(fetchInputEvents(fetcher)).rejects.toThrow(
      "Collector API failed: 503 Service Unavailable",
    );
  });
});

describe("fetchInputSummary", () => {
  it("loads input summary with date param", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        date: "2026-05-23",
        totalEvents: 100,
        keydownCount: 50,
        keyupCount: 50,
        segmentCount: 10,
        totalChars: 200,
        lastActivity: "2026-05-23T10:00:00Z",
        topApps: [{ processName: "Code", charCount: 150 }],
      }),
    });

    const result = await fetchInputSummary("2026-05-23", fetcher);
    expect(result.totalEvents).toBe(100);
    expect(result.keydownCount).toBe(50);
    expect(result.topApps).toHaveLength(1);
    expect(fetcher).toHaveBeenCalledWith(
      "/api/input-summary?date=2026-05-23",
    );
  });
});

describe("fetchTextSegments", () => {
  it("loads text segments with date param", async () => {
    const fetcher = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        segments: [
          {
            id: "seg-1",
            startedAt: "2026-05-23T09:00:00Z",
            endedAt: "2026-05-23T09:00:05Z",
            textContent: "fn main() {\n",
            keyCount: 12,
            backspaceCount: 2,
            deleteCount: 0,
            foregroundHwnd: 1111,
            foregroundPid: 100,
            processName: "Code",
            windowTitle: "main.rs",
          },
        ],
      }),
    });

    const result = await fetchTextSegments("2026-05-23", fetcher);
    expect(result).toHaveLength(1);
    expect(result[0].textContent).toBe("fn main() {\n");
    expect(fetcher).toHaveBeenCalledWith(
      "/api/text-segments?date=2026-05-23",
    );
  });
});
```

- [ ] **Step 2: Run TypeScript tests**

Run: `npx vitest run`
Expected: 16 tests pass (10 existing + 6 new). The `fetchInputEvents` call won't have CORS test since the pattern doesn't vary — existing CORS coverage is sufficient.

- [ ] **Step 3: Update Rust integration tests**

In `collector/tests/api_tests.rs`, add a test for the input endpoints:

After the `does_not_allow_cross_origin_reads` test, add:

```rust
#[tokio::test]
async fn serves_input_summary_as_json() {
    let store = Store::open_memory().unwrap();
    store.init().unwrap();

    let app = api::router(store, None);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let response = reqwest::get(format!("http://{addr}/api/input-summary?date=2026-05-23"))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["date"], "2026-05-23");
    assert!(body["totalEvents"].is_number());

    server.abort();
}
```

This test verifies the /api/input-summary endpoint returns valid JSON with expected fields, even with an empty database.

- [ ] **Step 4: Run Rust tests**

Run: `cargo test -p tsr-collector 2>&1`
Expected: all tests pass. New storage methods work with empty DB.

- [ ] **Step 5: Commit**

```bash
git add src/lib/input.test.ts collector/tests/api_tests.rs
git commit -m "test: add input API client tests and input-summary integration test"
```

---

### Task 13: Update documentation

**Files:**
- Modify: `README.md`
- Modify: `docs/infra.md`

- [ ] **Step 1: Update README**

In `README.md`:

After "Not included yet:" list item about Feature 2, change `- Keyboard/input monitoring (Feature 2 — text capture).` to remove it from "not included" and add to "Included":

Under "Included:", add:
```
- **Feature 2** — Keyboard input tracking via Raw Input API with text segment accumulation.
```

Add Feature 2 definition section after Feature 3:

```markdown
## Feature 2 Definition

Feature 2 is keyboard input tracking.

The collector registers for raw keyboard input via `RegisterRawInputDevices` and records every keydown/keyup event with virtual key code, scan code, and mapped character (via `ToUnicodeEx`). Typed text is accumulated into segments delimited by Enter presses. Each segment tracks:
- The accumulated text content (with backspace/deletion applied).
- Key count, backspace count, and delete count.
- Foreground application context (process name, window title).

IME composed text (Chinese, Japanese) will be captured in a follow-up via `WH_GETMESSAGE` hook (Feature 2B).

Text is stored in plaintext in the `text_segments` SQLite table and transmitted as JSON over the local REST API.
```

Add to API endpoints table:
```
| `GET` | `/api/input-events` | `?limit=N&segmentId=X` | Raw keyboard input events |
| `GET` | `/api/input-summary` | `?date=YYYY-MM-DD` | Aggregated input stats for a date |
| `GET` | `/api/text-segments` | `?date=YYYY-MM-DD&limit=N` | Typed text segments, newest first |
```

Under "WebUI Views", add:
```
- **Input Activity** — Total events, keydown/keyup counts, last activity, per-app character chart, and text segments table (Feature 2).
```

Under "Not included yet:", remove the Feature 2 line and add:
```
- Global `WH_GETMESSAGE` hook DLL for IME composed text capture (Feature 2B).
```

Add to verification checklist:
```
- Keydown/keyup events are captured from all foreground applications via Raw Input.
- Printable characters are mapped via `ToUnicodeEx` and accumulated into segments.
- Backspace and Delete key presses are tracked per segment.
- Enter key flushes the current segment to the `text_segments` table.
- `/api/input-events`, `/api/input-summary`, and `/api/text-segments` return valid JSON.
- WebUI Input Activity tab shows summary cards, per-app chart, and segments table.
- Sample data fallback works when collector is offline.
```

- [ ] **Step 2: Update infra.md flow diagram**

In `docs/infra.md`, update the mermaid flowchart to include the input collector:

Add to the OS sources:
```
OS --> InputCollector["Input Collector (Raw Input)"]
```

Add:
```
InputCollector --> Storage
```

Add a new section "Input Collector (Feature 2)" after the Screenshot Collector section:

```markdown
Input Collector (Feature 2):

- Registers for raw keyboard input via `RegisterRawInputDevices` with `RIDEV_INPUTSINK`.
- Runs a message-only window on a dedicated thread to receive `WM_INPUT`.
- Extracts VK code, scan code, and key-down/up flags from `RAWKEYBOARD`.
- Maps printable characters via `ToUnicodeEx` with the foreground window's keyboard layout.
- Accumulates text in an in-memory segment buffer.
- Flushes segments to `text_segments` on Enter or 30-second idle timeout.
- All input events are written to `input_events` alongside each segment.
```

Add `InputCollector --> Storage` to the mermaid flow.

Add to "Current collector/storage owns:":
```
- Keyboard input capture via Raw Input API.
- Character mapping via ToUnicodeEx.
- Text segment accumulation with backspace/delete tracking.
- Enter-delimited segment flushing.
```

- [ ] **Step 3: Commit**

```bash
git add README.md docs/infra.md
git commit -m "docs: add Feature 2 keyboard input tracker documentation"
```

---

## Verification Summary

After all tasks complete:

1. `cargo check -p tsr-collector` compiles
2. `cargo test -p tsr-collector` passes (existing + new integration test)
3. `npx tsc --noEmit` passes
4. `npx vitest run` passes (16 tests: 10 existing + 6 new)
5. WebUI renders three tabs: Statistics, Daily Tracking, Input Activity
6. Input Activity tab shows summary cards, per-app bar chart, segments table
7. Sample data toggles correctly
