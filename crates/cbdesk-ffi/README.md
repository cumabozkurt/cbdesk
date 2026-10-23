# cbdesk-ffi

Flutter calls this crate through flutter_rust_bridge 2.12.0. The hand-written
surface is `src/api.rs`. It forbids unsafe. `src/frb_generated.rs` is generated
and is the only file in this crate that may use unsafe.

The manual clock and the test-only consent arm are not exported. A short code
does not authenticate anyone. The session indicator has no off switch.

Regenerate from the repository root after changing `src/api.rs`:

```sh
flutter_rust_bridge_codegen generate --config-file app/desktop/flutter_rust_bridge.yaml
```

The package name is `cbdesk_ffi` (underscores). Cargokit loads `libcbdesk_ffi`.
