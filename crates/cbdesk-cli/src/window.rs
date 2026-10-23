// SPDX-License-Identifier: AGPL-3.0-or-later

//! A window that shows grayscale frames and, after an Input grant, forwards
//! the pointer and keyboard.
//!
//! The event loop runs on the caller thread. macOS requires that thread to
//! be the main thread. Input events are not queued when `allow_input` is
//! false: sending them without a grant would be `PROTOCOL_VIOLATION`.

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cbdesk_capture::Frame;
use cbdesk_input::{
    BUTTON_LEFT, BUTTON_MIDDLE, BUTTON_RIGHT, KEY_BACKSPACE, KEY_DOWN, KEY_ENTER, KEY_ESCAPE,
    KEY_LEFT, KEY_RIGHT, KEY_TAB, KEY_UP, MOD_ALT, MOD_CTRL, MOD_META, MOD_SHIFT,
};
use cbdesk_proto::{InputBody, InputEvent, KeyEvent, PointerEvent, UnknownFields};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::platform::pump_events::{EventLoopExtPumpEvents, PumpStatus};
use winit::window::{Window, WindowId};

use crate::error::CliError;

/// How to open the viewer window.
#[derive(Clone, Copy, Debug)]
pub struct WindowOptions {
    /// Queue pointer and key events. False until the host grants Input.
    pub allow_input: bool,
    /// Create the loop off the main thread. The smoke test on Linux uses this.
    pub any_thread: bool,
    /// Close after the window has been up this long. `None` waits for the person.
    pub close_after: Option<Duration>,
}

/// Show frames until the window closes or `options.close_after` elapses.
pub fn run(
    frames: Arc<Mutex<Option<Frame>>>,
    input_tx: tokio::sync::mpsc::UnboundedSender<InputEvent>,
    stop_tx: tokio::sync::watch::Sender<bool>,
    options: WindowOptions,
) -> Result<(), CliError> {
    let mut event_loop = build_loop(options.any_thread)?;
    let mut app = ViewerApp {
        frames,
        input_tx,
        allow_input: options.allow_input,
        gfx: None,
        buttons: 0,
        modifiers: 0,
        cursor: (0.0, 0.0),
        current: None,
        failed: None,
    };
    let started = Instant::now();
    loop {
        let status = event_loop.pump_app_events(Some(Duration::from_millis(16)), &mut app);
        let expired = options
            .close_after
            .is_some_and(|limit| started.elapsed() >= limit);
        if expired && app.gfx.is_none() && app.failed.is_none() {
            app.failed = Some("window did not open before the deadline".to_string());
        }
        if app.failed.is_some() || expired || matches!(status, PumpStatus::Exit(_)) {
            break;
        }
    }
    let _ = stop_tx.send(true);
    match app.failed.take() {
        Some(err) => Err(CliError::new(err)),
        None => Ok(()),
    }
}

/// Drop an input event unless Input was granted.
pub fn forward_input(allow_input: bool, event: InputEvent) -> Option<InputEvent> {
    allow_input.then_some(event)
}

/// Map a window cursor into frame pixels.
pub fn frame_point(cursor: (f64, f64), window: (u32, u32), frame: (u32, u32)) -> (i32, i32) {
    if window.0 == 0 || window.1 == 0 || frame.0 == 0 || frame.1 == 0 {
        return (0, 0);
    }
    let x = (cursor.0 * f64::from(frame.0) / f64::from(window.0)).floor() as i32;
    let y = (cursor.1 * f64::from(frame.1) / f64::from(window.1)).floor() as i32;
    let max_x = i32::try_from(frame.0.saturating_sub(1)).unwrap_or(i32::MAX);
    let max_y = i32::try_from(frame.1.saturating_sub(1)).unwrap_or(i32::MAX);
    (x.clamp(0, max_x), y.clamp(0, max_y))
}

fn build_loop(any_thread: bool) -> Result<EventLoop<()>, CliError> {
    let mut builder = EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11 as _;
        if any_thread {
            builder.with_any_thread(true);
        }
    }
    let _ = any_thread;
    builder
        .build()
        .map_err(|err| CliError::new(err.to_string()))
}

struct Gfx {
    /// The display connection the surface shares. Dropping it first can close
    /// that connection on some backends.
    #[allow(dead_code)]
    context: softbuffer::Context<Arc<Window>>,
    surface: softbuffer::Surface<Arc<Window>, Arc<Window>>,
}

struct ViewerApp {
    frames: Arc<Mutex<Option<Frame>>>,
    input_tx: tokio::sync::mpsc::UnboundedSender<InputEvent>,
    allow_input: bool,
    gfx: Option<Gfx>,
    buttons: u32,
    modifiers: u32,
    cursor: (f64, f64),
    current: Option<Frame>,
    failed: Option<String>,
}

impl ViewerApp {
    fn open_window(&mut self, event_loop: &ActiveEventLoop) {
        if self.gfx.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("CBDesk")
            .with_inner_size(LogicalSize::new(640.0, 360.0));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => window,
            Err(err) => {
                self.failed = Some(err.to_string());
                event_loop.exit();
                return;
            }
        };
        match open_gfx(window) {
            Ok(gfx) => self.gfx = Some(gfx),
            Err(err) => {
                self.failed = Some(err);
                event_loop.exit();
            }
        }
    }

    fn pull_frame(&mut self) {
        let mut guard = self
            .frames
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(frame) = guard.take() {
            self.current = Some(frame);
        }
    }

    fn paint(&mut self) {
        self.pull_frame();
        let Some(frame) = self.current.clone() else {
            return;
        };
        let Some(gfx) = self.gfx.as_mut() else {
            return;
        };
        if let Err(err) = paint_frame(gfx, &frame) {
            self.failed = Some(err);
        }
    }

    fn emit(&self, event: InputEvent) {
        if let Some(event) = forward_input(self.allow_input, event) {
            let _ = self.input_tx.send(event);
        }
    }

    fn emit_pointer(&self, wheel: i32) {
        let Some(gfx) = self.gfx.as_ref() else {
            return;
        };
        let size = gfx.surface.window().inner_size();
        let frame = self
            .current
            .as_ref()
            .map(|frame| (frame.width, frame.height))
            .unwrap_or((size.width.max(1), size.height.max(1)));
        let (x, y) = frame_point(self.cursor, (size.width, size.height), frame);
        self.emit(InputEvent {
            body: InputBody::Pointer(PointerEvent {
                x,
                y,
                buttons: self.buttons,
                wheel,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        });
    }
}

impl ApplicationHandler for ViewerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.open_window(event_loop);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) => self.paint(),
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor = (position.x, position.y);
                self.emit_pointer(0);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                apply_button(&mut self.buttons, button, state == ElementState::Pressed);
                self.emit_pointer(0);
            }
            WindowEvent::MouseWheel { delta, .. } => self.emit_pointer(wheel_ticks(delta)),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifier_mask(modifiers.state());
            }
            WindowEvent::KeyboardInput { event, .. } => self.on_key(event),
            WindowEvent::RedrawRequested => self.paint(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.paint();
        if self.failed.is_some() {
            event_loop.exit();
        }
    }
}

impl ViewerApp {
    fn on_key(&self, event: winit::event::KeyEvent) {
        if event.repeat {
            return;
        }
        let Some(keycode) = keycode_from_logical(&event.logical_key) else {
            return;
        };
        self.emit(InputEvent {
            body: InputBody::Key(KeyEvent {
                keycode,
                down: event.state == ElementState::Pressed,
                modifiers: self.modifiers,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        });
    }
}

fn open_gfx(window: Window) -> Result<Gfx, String> {
    let window = Arc::new(window);
    let context = softbuffer::Context::new(Arc::clone(&window)).map_err(|err| err.to_string())?;
    let surface = softbuffer::Surface::new(&context, window).map_err(|err| err.to_string())?;
    Ok(Gfx { context, surface })
}

fn paint_frame(gfx: &mut Gfx, frame: &Frame) -> Result<(), String> {
    let size = gfx.surface.window().inner_size();
    let width =
        NonZeroU32::new(size.width.max(1)).ok_or_else(|| "window width is zero".to_string())?;
    let height =
        NonZeroU32::new(size.height.max(1)).ok_or_else(|| "window height is zero".to_string())?;
    gfx.surface
        .resize(width, height)
        .map_err(|err| err.to_string())?;
    let mut buffer = gfx.surface.buffer_mut().map_err(|err| err.to_string())?;
    fill_xrgb(&mut buffer, width.get(), height.get(), frame);
    buffer.present().map_err(|err| err.to_string())?;
    Ok(())
}

fn fill_xrgb(buffer: &mut [u32], dst_w: u32, dst_h: u32, frame: &Frame) {
    let src_w = frame.width.max(1);
    let src_h = frame.height.max(1);
    for y in 0..dst_h {
        let src_y = ((u64::from(y) * u64::from(src_h)) / u64::from(dst_h.max(1))) as u32;
        let src_y = src_y.min(src_h - 1);
        for x in 0..dst_w {
            let src_x = ((u64::from(x) * u64::from(src_w)) / u64::from(dst_w.max(1))) as u32;
            let src_x = src_x.min(src_w - 1);
            let index = (src_y as usize).saturating_mul(src_w as usize) + src_x as usize;
            let luma = frame.pixels.get(index).copied().unwrap_or(0);
            let pixel = u32::from(luma);
            let slot = (y as usize).saturating_mul(dst_w as usize) + x as usize;
            if let Some(dest) = buffer.get_mut(slot) {
                *dest = (pixel << 16) | (pixel << 8) | pixel;
            }
        }
    }
}

fn apply_button(buttons: &mut u32, button: MouseButton, down: bool) {
    let Some(bit) = button_bit(button) else {
        return;
    };
    if down {
        *buttons |= bit;
    } else {
        *buttons &= !bit;
    }
}

fn button_bit(button: MouseButton) -> Option<u32> {
    match button {
        MouseButton::Left => Some(BUTTON_LEFT),
        MouseButton::Right => Some(BUTTON_RIGHT),
        MouseButton::Middle => Some(BUTTON_MIDDLE),
        MouseButton::Back | MouseButton::Forward | MouseButton::Other(_) => None,
    }
}

fn wheel_ticks(delta: MouseScrollDelta) -> i32 {
    let lines = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(position) => (position.y as f32) / 40.0,
    };
    lines.round() as i32
}

fn modifier_mask(state: ModifiersState) -> u32 {
    let mut mask = 0;
    if state.shift_key() {
        mask |= MOD_SHIFT;
    }
    if state.control_key() {
        mask |= MOD_CTRL;
    }
    if state.alt_key() {
        mask |= MOD_ALT;
    }
    if state.super_key() {
        mask |= MOD_META;
    }
    mask
}

fn keycode_from_logical(key: &Key) -> Option<u32> {
    match key {
        Key::Named(NamedKey::Enter) => Some(KEY_ENTER),
        Key::Named(NamedKey::Escape) => Some(KEY_ESCAPE),
        Key::Named(NamedKey::Backspace) => Some(KEY_BACKSPACE),
        Key::Named(NamedKey::Tab) => Some(KEY_TAB),
        Key::Named(NamedKey::ArrowLeft) => Some(KEY_LEFT),
        Key::Named(NamedKey::ArrowRight) => Some(KEY_RIGHT),
        Key::Named(NamedKey::ArrowUp) => Some(KEY_UP),
        Key::Named(NamedKey::ArrowDown) => Some(KEY_DOWN),
        Key::Character(text) => ascii_key(text),
        _ => None,
    }
}

fn ascii_key(text: &str) -> Option<u32> {
    let mut chars = text.chars();
    let ch = chars.next()?;
    if chars.next().is_some() || !(ch.is_ascii_graphic() || ch == ' ') {
        return None;
    }
    Some(u32::from(ch))
}

#[cfg(test)]
mod tests {
    use cbdesk_capture::{SyntheticCapturer, BARCODE_WIDTH};
    use cbdesk_input::KEY_ENTER;
    use winit::keyboard::{Key, NamedKey};

    use super::*;

    #[test]
    fn input_is_dropped_without_the_grant() {
        let event = InputEvent {
            body: InputBody::Key(KeyEvent {
                keycode: KEY_ENTER,
                down: true,
                modifiers: 0,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        };
        assert!(forward_input(false, event.clone()).is_none());
        assert!(forward_input(true, event).is_some());
    }

    #[test]
    fn named_and_ascii_keys_map() {
        assert_eq!(
            keycode_from_logical(&Key::Named(NamedKey::Enter)),
            Some(KEY_ENTER)
        );
        assert_eq!(
            keycode_from_logical(&Key::Character("a".into())),
            Some(u32::from(b'a'))
        );
    }

    #[test]
    fn cursor_maps_into_the_frame() {
        assert_eq!(frame_point((10.0, 10.0), (20, 20), (10, 10)), (5, 5));
    }

    #[test]
    fn viewer_window_opens_when_a_display_is_required() {
        if std::env::var_os("CBDESK_REQUIRE_DISPLAY").is_none() {
            return;
        }
        let frame = SyntheticCapturer::new(BARCODE_WIDTH, 16)
            .capture(1, 1_000)
            .expect("frame");
        let frames = Arc::new(Mutex::new(Some(frame)));
        let (input_tx, _input_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stop_tx, _stop_rx) = tokio::sync::watch::channel(false);
        run(
            frames,
            input_tx,
            stop_tx,
            WindowOptions {
                allow_input: false,
                any_thread: true,
                close_after: Some(Duration::from_millis(600)),
            },
        )
        .expect("window");
    }
}
