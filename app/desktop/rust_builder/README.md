# cbdesk_ffi plugin

This Flutter FFI plugin builds `crates/cbdesk-ffi` and bundles `libcbdesk_ffi`
with the desktop shell.

`cargokit/` is the copy shipped with flutter_rust_bridge 2.12.0
(`frb_example/flutter_via_create/rust_builder/cargokit`). Its license is MIT
(`cargokit/LICENSE`, Copyright 2022 Matej Knopp). Two local edits keep the
build on this repository's Rust pin:

- `build_tool/lib/src/builder.dart` runs `cargo` directly when the crate has
  no `cargokit.yaml`, so `rust-toolchain.toml` selects Rust 1.91.0.
- `cmake/cargokit.cmake` accepts an absolute path to the crate. The Linux
  and Windows CMake files resolve that path from the real location of this
  plugin, including when Windows reaches it through a symlink.
- `cargokit/build_pod.sh` treats an absolute first argument as the crate
  directory. The macOS podspec passes that absolute path.

The plugin does not open a session by itself. `lib/cbdesk_ffi.dart` is only
the package entry.
