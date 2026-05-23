use anyhow::Result;

use crate::models::WindowSnapshot;

pub fn sample_foreground_window() -> Result<WindowSnapshot> {
    platform::sample_foreground_window()
}

#[cfg(windows)]
mod platform {
    use anyhow::Result;
    use chrono::Utc;
    use sha2::{Digest, Sha256};
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        System::Threading::{
            OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
        },
        UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
        },
    };

    use crate::models::{CaptureStatus, WindowSnapshot};

    pub fn sample_foreground_window() -> Result<WindowSnapshot> {
        let captured_at = Utc::now();
        let hwnd = unsafe { GetForegroundWindow() };

        if hwnd.is_null() {
            return Ok(WindowSnapshot {
                captured_at,
                hwnd: 0,
                pid: 0,
                process_name: "no-foreground-window".to_string(),
                exe_path_hash: None,
                window_title: None,
                capture_status: CaptureStatus::NoForegroundWindow,
            });
        }

        let mut pid = 0u32;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut pid);
        }

        let (process_name, exe_path_hash, process_status) = process_metadata(pid);

        Ok(WindowSnapshot {
            captured_at,
            hwnd: hwnd as isize as i64,
            pid,
            process_name,
            exe_path_hash,
            window_title: window_title(hwnd),
            capture_status: process_status,
        })
    }

    fn window_title(hwnd: *mut core::ffi::c_void) -> Option<String> {
        let length = unsafe { GetWindowTextLengthW(hwnd) };
        if length <= 0 {
            return None;
        }

        let mut buffer = vec![0u16; length as usize + 1];
        let copied = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
        if copied <= 0 {
            return None;
        }

        Some(String::from_utf16_lossy(&buffer[..copied as usize]))
    }

    fn process_metadata(pid: u32) -> (String, Option<String>, CaptureStatus) {
        if pid == 0 {
            return (
                "unknown-process".to_string(),
                None,
                CaptureStatus::Unavailable,
            );
        }

        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return (format!("pid-{pid}"), None, CaptureStatus::PermissionDenied);
        }

        let mut size = 32_768u32;
        let mut buffer = vec![0u16; size as usize];
        let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut size) };
        unsafe {
            CloseHandle(handle);
        }

        if ok == 0 || size == 0 {
            return (format!("pid-{pid}"), None, CaptureStatus::Unavailable);
        }

        let path = String::from_utf16_lossy(&buffer[..size as usize]);
        let process_name = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown-process")
            .to_string();

        (process_name, Some(sha256_hex(&path)), CaptureStatus::Ok)
    }

    fn sha256_hex(value: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(value.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(not(windows))]
mod platform {
    use anyhow::Result;
    use chrono::Utc;

    use crate::models::{CaptureStatus, WindowSnapshot};

    pub fn sample_foreground_window() -> Result<WindowSnapshot> {
        Ok(WindowSnapshot {
            captured_at: Utc::now(),
            hwnd: 0,
            pid: 0,
            process_name: "unsupported-platform".to_string(),
            exe_path_hash: None,
            window_title: None,
            capture_status: CaptureStatus::Unavailable,
        })
    }
}
