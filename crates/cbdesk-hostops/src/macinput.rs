// SPDX-License-Identifier: AGPL-3.0-or-later

//! macOS session event tap.
//!
//! `CGEventTapCreate` returns null without Accessibility permission. The tap
//! runs on its own thread, which polls `CFRunLoopRunInMode` until the guard
//! is dropped. Drop sets a flag and joins that thread, so the tap is not
//! left installed. This module does not record that a desktop was blocked.
//! The privacy cover stays an X11 window.

use std::ffi::c_void;
use std::os::raw::{c_int, c_uint, c_ulonglong};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use cbdesk_session::{OsPermit, PermitKind};

use crate::OsError;

/// A session event tap. Drop stops it.
#[derive(Debug)]
pub struct InputGrab {
    tap: usize,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for InputGrab {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        if self.tap != 0 {
            // SAFETY: `tap` came from CGEventTapCreate. The worker has
            // exited its run-loop poll, disabled the tap, and no longer
            // uses the pointer. CFRelease is the matching release.
            unsafe {
                CFRelease(self.tap as *const c_void);
            }
            self.tap = 0;
        }
    }
}

/// No macOS privacy window is implemented.
#[derive(Debug)]
pub struct PrivacyCover;

/// Swallow session keyboard and mouse events while the guard lives.
pub fn block_local_input(permit: OsPermit, _display: Option<&str>) -> Result<InputGrab, OsError> {
    if permit.kind() != PermitKind::BlockInput {
        return Err(OsError::WrongPermit);
    }
    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_stop = Arc::clone(&stop);
    let worker = thread::Builder::new()
        .name("cbdesk-event-tap".to_string())
        .spawn(move || tap_thread(worker_stop, tx))
        .map_err(|err| OsError::Failed(err.to_string()))?;
    match rx.recv_timeout(Duration::from_secs(2)) {
        Ok(Ok(tap)) => Ok(InputGrab {
            tap,
            stop,
            thread: Some(worker),
        }),
        Ok(Err(message)) => {
            stop.store(true, Ordering::Release);
            let _ = worker.join();
            Err(OsError::Unavailable(message))
        }
        Err(_) => {
            stop.store(true, Ordering::Release);
            let _ = worker.join();
            // The worker may have sent the tap after the timeout. This
            // function did not hand that pointer to a guard, so it releases
            // it here. A guard that already received the pointer is not this
            // branch.
            if let Ok(Ok(tap)) = rx.try_recv() {
                // SAFETY: `tap` came from CGEventTapCreate and the worker
                // has exited. Nothing else holds it.
                unsafe {
                    CFRelease(tap as *const c_void);
                }
            }
            Err(OsError::Unavailable(
                "the event tap did not answer within 2 seconds".to_string(),
            ))
        }
    }
}

/// Privacy mode is still an X11 window.
pub fn cover_display(permit: OsPermit, _display: Option<&str>) -> Result<PrivacyCover, OsError> {
    let _ = permit.kind();
    Err(OsError::Unavailable(
        "privacy mode is an X11 window; macOS has no cover in this tree".to_string(),
    ))
}

fn tap_thread(stop: Arc<AtomicBool>, tx: mpsc::Sender<Result<usize, String>>) {
    // SAFETY: location 1 is kCGSessionEventTap, placement 0 is head insert,
    // options 0 is the default tap, and the mask is every event bit. A null
    // return means the process may not tap, and no event is swallowed.
    let tap = unsafe { CGEventTapCreate(1, 0, 0, !0, swallow, std::ptr::null_mut()) };
    if tap.is_null() {
        let _ = tx.send(Err(
            "CGEventTapCreate returned null; accessibility permission was not granted and input was not blocked"
                .to_string(),
        ));
        return;
    }
    // SAFETY: `tap` is non-null. The default-mode constant is Apple's
    // process-wide CFString. The poll returns within 0.2s so Drop can join.
    unsafe {
        CGEventTapEnable(tap, true);
    }
    if tx.send(Ok(tap as usize)).is_err() {
        // SAFETY: the receiver is gone, so this thread still owns the tap.
        unsafe {
            CGEventTapEnable(tap, false);
            CFRelease(tap as *const c_void);
        }
        return;
    }
    while !stop.load(Ordering::Acquire) {
        // SAFETY: the tap is enabled. `return_after_source` is false, so the
        // call waits up to 0.2 seconds and then returns.
        unsafe {
            let _ = CFRunLoopRunInMode(kCFRunLoopDefaultMode, 0.2, false);
        }
    }
    // SAFETY: the owner CFReleases after this thread returns. Disable first
    // so the tap stops swallowing events while the join finishes.
    unsafe {
        CGEventTapEnable(tap, false);
    }
}

/// Discard one tapped event.
///
/// # Safety
///
/// Apple calls this only while the tap exists. A null return discards the
/// event instead of passing it on.
unsafe extern "C" fn swallow(
    _proxy: *mut c_void,
    _kind: c_uint,
    _event: *mut c_void,
    _user: *mut c_void,
) -> *mut c_void {
    std::ptr::null_mut()
}

// SAFETY: Apple's C signatures. `bool` matches C `_Bool` for the enable flag
// and for `returnAfterSourceHandled`. Each framework is its own block.
// Stacking two `#[link]` attributes on one block trips
// `clippy::duplicated_attributes` on macOS.
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: c_uint,
        place: c_uint,
        options: c_uint,
        events: c_ulonglong,
        callback: unsafe extern "C" fn(
            *mut c_void,
            c_uint,
            *mut c_void,
            *mut c_void,
        ) -> *mut c_void,
        user: *mut c_void,
    ) -> *mut c_void;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(value: *const c_void);
    static kCFRunLoopDefaultMode: *const c_void;
    fn CFRunLoopRunInMode(mode: *const c_void, seconds: f64, return_after_source: bool) -> c_int;
}
