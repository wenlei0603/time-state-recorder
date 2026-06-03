use image::{DynamicImage, codecs::jpeg::JpegEncoder, imageops};
use std::io::Cursor;

fn resize_and_encode_jpeg(
    image: image::RgbaImage,
    max_width: u32,
    quality: u8,
) -> Option<(Vec<u8>, u32, u32)> {
    let (w, h) = (image.width(), image.height());
    let dynamic = DynamicImage::ImageRgba8(image);
    let resized = if w > max_width {
        let ratio = max_width as f64 / w as f64;
        let new_h = (h as f64 * ratio) as u32;
        dynamic.resize(max_width, new_h, imageops::FilterType::Lanczos3)
    } else {
        dynamic
    };

    let mut buf = Cursor::new(Vec::new());
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, quality.clamp(1, 100));
    encoder.encode_image(&resized).ok()?;

    let inner = buf.into_inner();
    let (tw, th) = (resized.width(), resized.height());
    Some((inner, tw, th))
}

#[cfg(windows)]
mod platform {
    use super::resize_and_encode_jpeg;
    use windows_sys::Win32::{
        System::SystemInformation::GetTickCount,
        UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
    };

    pub fn capture_thumbnail(max_width: u32, quality: u8) -> Option<(Vec<u8>, u32, u32)> {
        let monitors = xcap::Monitor::all().ok()?;
        let primary = monitors.iter().find(|m| m.is_primary().unwrap_or(false))?;
        let image = primary.capture_image().ok()?;
        resize_and_encode_jpeg(image, max_width, quality)
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

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn jpeg_quality_changes_encoded_output_size() {
        let image = patterned_image(320, 240);

        let low_quality = resize_and_encode_jpeg(image.clone(), 320, 35).unwrap();
        let high_quality = resize_and_encode_jpeg(image, 320, 92).unwrap();

        assert_eq!((low_quality.1, low_quality.2), (320, 240));
        assert_eq!((high_quality.1, high_quality.2), (320, 240));
        assert!(
            high_quality.0.len() > low_quality.0.len(),
            "higher JPEG quality should preserve more detail and produce a larger file"
        );
    }

    #[test]
    fn jpeg_encoding_respects_max_width() {
        let image = patterned_image(1600, 900);

        let encoded = resize_and_encode_jpeg(image, 960, 82).unwrap();

        assert_eq!((encoded.1, encoded.2), (960, 540));
    }

    fn patterned_image(width: u32, height: u32) -> ImageBuffer<Rgba<u8>, Vec<u8>> {
        ImageBuffer::from_fn(width, height, |x, y| {
            let r = ((x * 13 + y * 7) % 256) as u8;
            let g = ((x * 3 + y * 19) % 256) as u8;
            let b = ((x * 23 + y * 5) % 256) as u8;
            Rgba([r, g, b, 255])
        })
    }
}
