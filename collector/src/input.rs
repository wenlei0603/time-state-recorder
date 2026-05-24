use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    models::{CollectorHealth, InputEvent, InputEventType, TextSegment},
    storage::Store,
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
        let is_keydown = signal.is_keydown;
        let is_enter = vk == VK_RETURN && is_keydown;
        let ch = signal.character.clone();

        self.events.push(signal);

        if is_enter {
            self.current_text.push('\n');
            return true;
        }

        if is_keydown {
            self.key_count += 1;

            if vk == VK_BACK {
                self.current_text.pop();
                self.backspace_count += 1;
            } else if vk == VK_DELETE {
                self.delete_count += 1;
            } else if let Some(ref c) = ch {
                if !c.is_empty() {
                    self.current_text.push_str(c);
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
            .map(|s| InputEvent {
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

pub fn spawn_input_collector(
    store: Arc<Mutex<Store>>,
    health: Arc<Mutex<CollectorHealth>>,
) -> tokio::task::JoinHandle<()> {
    let (tx, mut rx) = mpsc::unbounded_channel::<InputSignal>();

    {
        let mut h = health.lock().unwrap();
        h.input_collector.status = "running".into();
    }

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
                            if let Ok(mut h) = health.lock() {
                                h.input_collector.last_event_at = Some(Utc::now());
                                h.input_collector.error_count = 0;
                                h.input_collector.last_error = None;
                            }
                        }
                    }
                }
                Ok(None) => {
                    if let Some(buf) = buffer.take() {
                        let (segment, events) = buf.flush();
                        if let Ok(mut store) = store.lock() {
                            let _ = store.insert_input_segment(&segment, &events);
                            if let Ok(mut h) = health.lock() {
                                h.input_collector.last_event_at = Some(Utc::now());
                                h.input_collector.error_count = 0;
                                h.input_collector.last_error = None;
                            }
                        }
                    }
                    break;
                }
                Err(_elapsed) => {
                    if let Some(buf) = buffer.take() {
                        let (segment, events) = buf.flush();
                        if let Ok(mut store) = store.lock() {
                            let _ = store.insert_input_segment(&segment, &events);
                            if let Ok(mut h) = health.lock() {
                                h.input_collector.last_event_at = Some(Utc::now());
                                h.input_collector.error_count = 0;
                                h.input_collector.last_error = None;
                            }
                        }
                    }
                }
            }
        }
    })
}

#[cfg(windows)]
mod platform {
    use std::sync::OnceLock;

    use tokio::sync::mpsc;
    use windows_sys::Win32::{
        Devices::HumanInterfaceDevice::{HID_USAGE_GENERIC_KEYBOARD, HID_USAGE_PAGE_GENERIC},
        Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{GetKeyboardLayout, ToUnicodeEx},
            Input::{
                GetRawInputData, HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT,
                RIDEV_INPUTSINK, RIM_TYPEKEYBOARD, RegisterRawInputDevices,
            },
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CreateWindowExW, DefWindowProcW, DestroyWindow,
                DispatchMessageW, GWLP_USERDATA, GetForegroundWindow, GetMessageW,
                GetWindowLongPtrW, GetWindowThreadProcessId, HWND_MESSAGE, MSG, PostQuitMessage,
                RI_KEY_BREAK, RegisterClassW, SetWindowLongPtrW, TranslateMessage, WM_DESTROY,
                WM_INPUT, WNDCLASSW,
            },
        },
    };

    use chrono::Utc;

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
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                &mut *state as *mut WindowState as isize,
            );

            let rid = RAWINPUTDEVICE {
                usUsagePage: HID_USAGE_PAGE_GENERIC,
                usUsage: HID_USAGE_GENERIC_KEYBOARD,
                dwFlags: RIDEV_INPUTSINK,
                hwndTarget: hwnd,
            };

            if RegisterRawInputDevices(&rid, 1, std::mem::size_of::<RAWINPUTDEVICE>() as u32) == 0 {
                eprintln!("input collector: RegisterRawInputDevices failed");
                DestroyWindow(hwnd);
                return;
            }

            let mut msg = MSG {
                hwnd: std::ptr::null_mut(),
                message: 0,
                wParam: 0,
                lParam: 0,
                time: 0,
                pt: POINT { x: 0, y: 0 },
            };

            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Clean up
            let _boxed = Box::from_raw(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState);
            DestroyWindow(hwnd);
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            if msg == WM_INPUT {
                let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
                if state_ptr.is_null() {
                    return DefWindowProcW(hwnd, msg, wparam, lparam);
                }

                let state = &mut *state_ptr;

                let mut size: u32 = 0;
                let hrawinput = lparam as HRAWINPUT;
                if GetRawInputData(
                    hrawinput,
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
                    hrawinput,
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
                    let is_keydown = kb.Flags & RI_KEY_BREAK as u16 == 0;
                    let is_e0 = kb.Flags & 0x02 != 0;

                    // Update key state for ToUnicodeEx
                    if is_keydown {
                        state.key_state[vk as usize] = 0x80;
                    } else {
                        state.key_state[vk as usize] = 0;
                    }
                    if is_e0 {
                        state.key_state[scan as usize] = 0x80;
                    }

                    // Map to character on keydown
                    let character = if is_keydown {
                        let mut buf = [0u16; 4];
                        let result = {
                            let keyboard_layout = {
                                let fg = GetForegroundWindow();
                                let mut tid = 0u32;
                                GetWindowThreadProcessId(fg, &mut tid);
                                GetKeyboardLayout(tid)
                            };

                            ToUnicodeEx(
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
    }

    fn encode_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
}

#[cfg(not(windows))]
mod platform {
    use super::InputSignal;
    use tokio::sync::mpsc;

    pub(super) fn run_raw_input_loop(_tx: mpsc::UnboundedSender<InputSignal>) {
        eprintln!("input collector: not supported on this platform");
    }
}
