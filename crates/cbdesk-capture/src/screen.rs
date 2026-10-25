// SPDX-License-Identifier: AGPL-3.0-or-later

//! One real desktop frame.
//!
//! Linux without `WAYLAND_DISPLAY` uses the X11 capturer. Linux with that
//! variable set uses the Wayland ScreenCast portal and PipeWire. Windows and
//! macOS use the platform capturer in the same library. A missing display or
//! a denied portal is [`CaptureError::Unavailable`], not a synthetic picture.

use crate::barcode::Frame;
use crate::error::CaptureError;
use crate::picture::{self, Picture};

/// One display reported by the operating system.
///
/// `index` is the position in [`list_monitors`] for this call. It is not a
/// stable hardware id. This workspace did not attach a second physical
/// display, so a list of one monitor is the observation that was available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonitorInfo {
    /// Position in the list from this call.
    pub index: u32,
    /// Name the capturer reported. Empty when the name could not be read.
    pub name: String,
    /// True when the capturer marked this display primary.
    pub primary: bool,
    /// Width in pixels. Zero when the size could not be read.
    pub width: u32,
    /// Height in pixels. Zero when the size could not be read.
    pub height: u32,
}

/// List the displays the capturer can see.
///
/// An empty machine is [`CaptureError::Unavailable`]. The live host still
/// calls [`capture_picture`], which is the primary display.
pub fn list_monitors() -> Result<Vec<MonitorInfo>, CaptureError> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        platform::list()
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        Err(CaptureError::Unavailable(
            "this operating system has no screen capturer".to_string(),
        ))
    }
}

/// Capture one display from [`list_monitors`] as I420.
///
/// An index past the list is [`CaptureError::Unavailable`]. On Linux with
/// `WAYLAND_DISPLAY` set, this uses that monitor's ScreenCast recorder.
pub fn capture_picture_on(index: u32) -> Result<Picture, CaptureError> {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    {
        platform::capture_index(index)
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = index;
        Err(CaptureError::Unavailable(
            "this operating system has no screen capturer".to_string(),
        ))
    }
}

/// Capture the primary display as I420.
pub fn capture_picture() -> Result<Picture, CaptureError> {
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

/// Capture the primary display and keep the luma plane.
pub fn capture_screen() -> Result<Frame, CaptureError> {
    capture_picture()?.to_luma()
}

#[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
mod platform {
    use super::*;

    pub(super) fn list() -> Result<Vec<MonitorInfo>, CaptureError> {
        Ok(monitors()?
            .iter()
            .enumerate()
            .map(|(index, monitor)| MonitorInfo {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                name: monitor.name().unwrap_or_default(),
                primary: matches!(monitor.is_primary(), Ok(true)),
                width: monitor.width().unwrap_or(0),
                height: monitor.height().unwrap_or(0),
            })
            .collect())
    }

    pub(super) fn capture_index(index: u32) -> Result<Picture, CaptureError> {
        let mut found = monitors()?;
        let position = usize::try_from(index).map_err(|_| {
            CaptureError::Unavailable("monitor index does not fit this process".to_string())
        })?;
        if position >= found.len() {
            return Err(CaptureError::Unavailable(format!(
                "monitor {index} is not in a list of {}",
                found.len()
            )));
        }
        let monitor = found.swap_remove(position);
        #[cfg(target_os = "linux")]
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return capture_portal(monitor);
        }
        capture_still(monitor)
    }

    pub(super) fn capture_primary() -> Result<Picture, CaptureError> {
        let listed = list()?;
        let index = listed
            .iter()
            .find(|monitor| monitor.primary)
            .map(|monitor| monitor.index)
            .unwrap_or(0);
        capture_index(index)
    }

    fn monitors() -> Result<Vec<xcap::Monitor>, CaptureError> {
        let monitors = xcap::Monitor::all().map_err(unavailable)?;
        if monitors.is_empty() {
            return Err(CaptureError::Unavailable("no monitor".to_string()));
        }
        Ok(monitors)
    }

    fn capture_still(monitor: xcap::Monitor) -> Result<Picture, CaptureError> {
        let image = monitor.capture_image().map_err(unavailable)?;
        from_rgba(image.width(), image.height(), image.as_raw())
    }

    #[cfg(target_os = "linux")]
    fn capture_portal(monitor: xcap::Monitor) -> Result<Picture, CaptureError> {
        use std::time::Duration;

        let (recorder, frames) = monitor.video_recorder().map_err(unavailable)?;
        recorder.start().map_err(unavailable)?;
        let frame = frames
            .recv_timeout(Duration::from_secs(8))
            .map_err(|err| CaptureError::Unavailable(err.to_string()))?;
        let _ = recorder.stop();
        from_rgba(frame.width, frame.height, &frame.raw)
    }

    fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Picture, CaptureError> {
        picture::from_rgba(width, height, rgba).map_err(|_| {
            CaptureError::Unavailable("capturer returned a buffer that is not RGBA".to_string())
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

    #[test]
    fn an_index_past_the_list_is_refused_when_a_display_is_present() {
        let required = std::env::var_os("CBDESK_REQUIRE_DISPLAY").is_some();
        let monitors = match list_monitors() {
            Ok(monitors) => monitors,
            Err(error) if !required => {
                let _ = error;
                return;
            }
            Err(error) => panic!("a display was required and listing failed: {error}"),
        };
        assert!(!monitors.is_empty());
        assert!(monitors.iter().filter(|monitor| monitor.primary).count() <= 1);
        let index = monitors
            .iter()
            .find(|monitor| monitor.primary)
            .map(|monitor| monitor.index)
            .unwrap_or(0);
        let picture = capture_picture_on(index).expect("indexed capture");
        assert!(picture.width >= 1 && picture.height >= 1);
        assert!(capture_picture_on(u32::MAX).is_err());
    }
}
