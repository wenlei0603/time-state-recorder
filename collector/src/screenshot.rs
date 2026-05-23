#[cfg(windows)]
mod platform {
    use image::{ImageFormat, imageops};
    use std::io::Cursor;
    use windows_sys::Win32::{
        System::SystemInformation::GetTickCount,
        UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
    };

    pub fn capture_thumbnail(max_width: u32, _quality: u8) -> Option<(Vec<u8>, u32, u32)> {
        let monitors = xcap::Monitor::all().ok()?;
        let primary = monitors.iter().find(|m| m.is_primary().unwrap_or(false))?;
        let image = primary.capture_image().ok()?;
        let (w, h) = (image.width(), image.height());

        let thumb = if w > max_width {
            let ratio = max_width as f64 / w as f64;
            let new_h = (h as f64 * ratio) as u32;
            let dynamic = image::DynamicImage::ImageRgba8(image);
            dynamic.resize(max_width, new_h, imageops::FilterType::Lanczos3)
        } else {
            image::DynamicImage::ImageRgba8(image)
        };

        let mut buf = Cursor::new(Vec::new());
        thumb.write_to(&mut buf, ImageFormat::Jpeg).ok()?;

        let inner = buf.into_inner();
        let (tw, th) = (thumb.width(), thumb.height());
        Some((inner, tw, th))
    }

    pub fn idle_seconds() -> f64 {
        unsafe {
            let mut lii = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };

            if GetLastInputInfo(&mut lii) != 0 {
                let now = GetTickCount();
                let idle_ms = now.wrapping_sub(lii.dwTime);
                idle_ms as f64 / 1000.0
            } else {
                0.0
            }
        }
    }
}

#[cfg(not(windows))]
mod platform {
    pub fn capture_thumbnail(_max_width: u32, _quality: u8) -> Option<(Vec<u8>, u32, u32)> {
        None
    }

    pub fn idle_seconds() -> f64 {
        0.0
    }
}

pub use platform::*;
