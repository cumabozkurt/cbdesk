// SPDX-License-Identifier: AGPL-3.0-or-later

//! X11 privacy window and input grab.
//!
//! Both effects use the display name the caller passes. Tests pass a private
//! Xvfb. Dropping the guard destroys the window or releases the grab. Wayland
//! is not handled here: `x11rb` only talks to an X server.

use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    ConnectionExt, CreateWindowAux, EventMask, GrabMode, GrabStatus, WindowClass,
};
use x11rb::rust_connection::RustConnection;

use cbdesk_session::{OsPermit, PermitKind};

use crate::OsError;

/// A keyboard and pointer grab on one X server.
pub struct InputGrab {
    conn: RustConnection,
}

impl Drop for InputGrab {
    fn drop(&mut self) {
        let _ = self.conn.ungrab_pointer(x11rb::CURRENT_TIME);
        let _ = self.conn.ungrab_keyboard(x11rb::CURRENT_TIME);
        let _ = self.conn.flush();
    }
}

/// A fullscreen black window on one X server.
pub struct PrivacyCover {
    conn: RustConnection,
    window: u32,
}

impl PrivacyCover {
    /// The window's current size, read back from the server.
    pub fn mapped_size(&self) -> Result<(u16, u16), OsError> {
        let geometry = self
            .conn
            .get_geometry(self.window)
            .map_err(|err| OsError::Failed(err.to_string()))?
            .reply()
            .map_err(|err| OsError::Failed(err.to_string()))?;
        Ok((geometry.width, geometry.height))
    }
}

impl Drop for PrivacyCover {
    fn drop(&mut self) {
        let _ = self.conn.destroy_window(self.window);
        let _ = self.conn.flush();
    }
}

/// Grab the keyboard and pointer on `display`.
pub fn block_local_input(permit: OsPermit, display: Option<&str>) -> Result<InputGrab, OsError> {
    if permit.kind() != PermitKind::BlockInput {
        return Err(OsError::WrongPermit);
    }
    let (conn, screen) = connect(display)?;
    let root = conn.setup().roots[screen].root;
    let keyboard = conn
        .grab_keyboard(
            false,
            root,
            x11rb::CURRENT_TIME,
            GrabMode::ASYNC,
            GrabMode::ASYNC,
        )
        .map_err(|err| OsError::Failed(err.to_string()))?
        .reply()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    if keyboard.status != GrabStatus::SUCCESS {
        return Err(OsError::Failed(format!(
            "keyboard grab status {}",
            u8::from(keyboard.status)
        )));
    }
    let pointer = conn
        .grab_pointer(
            false,
            root,
            EventMask::default(),
            GrabMode::ASYNC,
            GrabMode::ASYNC,
            x11rb::NONE,
            x11rb::NONE,
            x11rb::CURRENT_TIME,
        )
        .map_err(|err| OsError::Failed(err.to_string()))?
        .reply()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    if pointer.status != GrabStatus::SUCCESS {
        let _ = conn.ungrab_keyboard(x11rb::CURRENT_TIME);
        let _ = conn.flush();
        return Err(OsError::Failed(format!(
            "pointer grab status {}",
            u8::from(pointer.status)
        )));
    }
    conn.flush()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    Ok(InputGrab { conn })
}

/// Map a black window that covers the X screen.
pub fn cover_display(permit: OsPermit, display: Option<&str>) -> Result<PrivacyCover, OsError> {
    if permit.kind() != PermitKind::Privacy {
        return Err(OsError::WrongPermit);
    }
    let (conn, screen_index) = connect(display)?;
    let window = conn
        .generate_id()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    let screen = &conn.setup().roots[screen_index];
    let root = screen.root;
    let width = screen.width_in_pixels;
    let height = screen.height_in_pixels;
    let black = screen.black_pixel;
    conn.create_window(
        x11rb::COPY_DEPTH_FROM_PARENT,
        window,
        root,
        0,
        0,
        width,
        height,
        0,
        WindowClass::INPUT_OUTPUT,
        x11rb::COPY_FROM_PARENT,
        &CreateWindowAux::new()
            .background_pixel(black)
            .override_redirect(1_u32),
    )
    .map_err(|err| OsError::Failed(err.to_string()))?;
    conn.map_window(window)
        .map_err(|err| OsError::Failed(err.to_string()))?;
    conn.flush()
        .map_err(|err| OsError::Failed(err.to_string()))?;
    Ok(PrivacyCover { conn, window })
}

fn connect(display: Option<&str>) -> Result<(RustConnection, usize), OsError> {
    x11rb::connect(display).map_err(|err| OsError::Unavailable(err.to_string()))
}
