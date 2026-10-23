Screen capture and the synthetic barcode used by tests.

`capture_screen` reads the operating-system display: X11, the Wayland
ScreenCast portal plus PipeWire when `WAYLAND_DISPLAY` is set, and the
Windows and macOS capturer in the same library. A missing display is an
error. It is not replaced with a barcode. The caller must already have
granted View. This crate does not check that grant.
