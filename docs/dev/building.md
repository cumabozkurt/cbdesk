# Building

## Requirements

- Rust 1.91.0 and the `rustfmt` and `clippy` components. `rust-toolchain.toml`
  selects them. ADR-006 records why the pin is 1.91 and not a newer compiler:
  that is the `rust-version` of iroh 1.4.1.
- A C toolchain (gcc, clang, or MSVC) is required. iroh's `tls-ring` feature
  compiles `ring`. BLAKE3 in this workspace still uses the `pure` feature.
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

The QUIC echo binds two loopback sockets. The LAN tests send the mDNS and
broadcast encodings between loopback UDP sockets. Neither test needs a public
network or port 5353.

## Pins

Dependencies are exact versions. Do not change a version to a range. The
direct dependencies on `base64ct`, `hashbrown`, `indexmap`, `zeroize_derive`,
`tempfile`, and `toml_edit` keep transitive crates on releases this tree has
already compiled. iroh brings its own copy of newer majors where the crates
do not unify (`ed25519-dalek` 3, `getrandom` 0.4).

## What will not build

Screen capture, input injection, installers, and mobile targets have no code.
Their directories contain a README so the layout from the master plan is
visible. They are not Cargo workspace members.
