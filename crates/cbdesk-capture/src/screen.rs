// SPDX-License-Identifier: AGPL-3.0-or-later

//! One real desktop frame.
//!
//! Linux without `WAYLAND_DISPLAY` uses the X11 capturer. Linux with that
//! variable set uses the Wayland ScreenCast portal and PipeWire. Windows and
//! macOS use the platform capturer in the same library. A missing display or
//! a denied portal is [`CaptureError::Unavailable`], not a synthetic picture.

use crate::barcode::Frame;
use crate::error::CaptureError;

/// Capture the primary display.
pub fn capture_screen() -> Result<Frame, CaptureError> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        platform::capture_primary()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Err(CaptureError::Unavailable(
            "this operating system has no screen capturer".to_string(),
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
mod platform {
    use std::time::Duration;

    use super::*;

    enum Source {
        Still(xcap::Monitor),
        #[cfg(target_os = "linux")]
        Portal,
    }

    pub(super) fn capture_primary() -> Result<Frame, CaptureError> {
        #[cfg(target_os = "linux")]
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return capture(Source::Portal);
        }
        let monitor = primary_monitor()?;
        capture(Source::Still(monitor))
    }

    fn primary_monitor() -> Result<xcap::Monitor, CaptureError> {
        let mut monitors = xcap::Monitor::all().map_err(unavailable)?;
        if monitors.is_empty() {
            return Err(CaptureError::Unavailable("no monitor".to_string()));
        }
        let index = monitors
            .iter()
            .position(|monitor| matches!(monitor.is_primary(), Ok(true)))
            .unwrap_or(0);
        Ok(monitors.swap_remove(index))
    }

    fn capture(source: Source) -> Result<Frame, CaptureError> {
        match source {
            Source::Still(monitor) => {
                let image = monitor.capture_image().map_err(unavailable)?;
                from_rgba(image.width(), image.height(), image.as_raw())
            }
            #[cfg(target_os = "linux")]
            Source::Portal => capture_portal(),
        }
    }

    #[cfg(target_os = "linux")]
    fn capture_portal() -> Result<Frame, CaptureError> {
        let monitor = primary_monitor()?;
        let (recorder, frames) = monitor.video_recorder().map_err(unavailable)?;
        recorder.start().map_err(unavailable)?;
        let frame = frames
            .recv_timeout(Duration::from_secs(8))
            .map_err(|err| CaptureError::Unavailable(err.to_string()))?;
        let _ = recorder.stop();
        from_rgba(frame.width, frame.height, &frame.raw)
    }

    fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Frame, CaptureError> {
        let samples = usize::try_from(width)
            .ok()
            .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
            .ok_or(CaptureError::BadDimensions)?;
        if rgba.len() != samples.saturating_mul(4) || samples == 0 {
            return Err(CaptureError::Unavailable(
                "capturer returned a buffer that is not RGBA".to_string(),
            ));
        }
        let mut pixels = Vec::with_capacity(samples);
        for pixel in rgba.chunks_exact(4) {
            let luma =
                u16::from(pixel[0]) * 77 + u16::from(pixel[1]) * 150 + u16::from(pixel[2]) * 29;
            pixels.push(u8::try_from(luma >> 8).unwrap_or(255));
        }
        Ok(Frame {
            width,
            height,
            pixels,
        })
    }

    fn unavailable(err: impl std::fmt::Display) -> CaptureError {
        CaptureError::Unavailable(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_capture_reads_a_screen_when_a_display_is_present() {
        let required = std::env::var_os("CBDESK_REQUIRE_DISPLAY").is_some();
        let mut last = None;
        for _ in 0..5 {
            match capture_screen() {
                Ok(frame) => {
                    assert!(frame.width >= 1 && frame.height >= 1);
                    assert_eq!(
                        frame.pixels.len(),
                        usize::try_from(frame.width).unwrap_or(0)
                            * usize::try_from(frame.height).unwrap_or(0)
                    );
                    return;
                }
                Err(error) if !required => {
                    let _ = error;
                    return;
                }
                Err(error) => last = Some(error),
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!(
            "a display was required and capture failed: {}",
            last.map(|error| error.to_string())
                .unwrap_or_else(|| "no error".to_string())
        );
    }
}
