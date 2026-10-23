# Building

## Requirements

- Rust 1.83.0 and the `rustfmt` and `clippy` components. `rust-toolchain.toml`
  selects them.
- A C toolchain is not required for this milestone. BLAKE3 is built with the
  `pure` feature so the crate does not compile C.
- Flutter is not required. There is no Dart package yet.

## Commands

From the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

`cargo test` includes an Argon2id vector at the backup parameters (64 MiB,
3 passes). That test takes a fraction of a second on a desktop and longer on
a small CI runner. It is the parameter lock for identity backup, which is not
wired to a file format yet.

## Pins

Dependencies are exact versions. Current crates.io releases often need Rust
1.85, and Rust 1.83 does not have the MSRV-aware resolver. Do not change a
version to a range. The direct dependencies on `base64ct`, `hashbrown`,
`indexmap`, `zeroize_derive`, `tempfile`, and `toml_edit` are version anchors
for transitive crates.

## What will not build

Screen capture, input injection, installers, and mobile targets have no code.
Their directories contain a README so the layout from the master plan is
visible. They are not Cargo workspace members.
