# Supply chain

`deny.toml` is the cargo-deny 0.20 policy:

- Licenses outside the allow-list fail.
- Wildcard dependency requirements fail.
- Git and unknown registries fail.
- Advisory and yanked-crate findings warn or fail according to the file.
- Multiple versions warn. iroh 1.4.1 pulls a second major of `ed25519-dalek`
  and of `getrandom`. The identity crate stays on ed25519-dalek 2, so the
  duplicates stay.
- `CDLA-Permissive-2.0` is allowed for the `webpki-roots` CA bundle that
  iroh pulls in. It is a permissive data license, not a copyleft exception.
- `AGPL-3.0-only` is allowed for rav1d-safe. The crate's SPDX expression is
  `AGPL-3.0-only OR LicenseRef-Imazen-Commercial`. This tree uses the AGPL
  side (ADR-015). Assembler features of rav1e and rav1d-safe stay off.
- `BSL-1.0` is allowed for `clipboard-win`, the Windows text clipboard behind
  arboard. It is the Boost Software License.
- `wide` 1.7.1 is `Zlib OR Apache-2.0 OR MIT`. The damage scan calls it.
  The intrinsics stay in that crate.
- oxideav-vp8 is MIT. winit is Apache-2.0. softbuffer and arboard are MIT OR
  Apache-2.0. `cargo audit` may warn that `ttf-parser` (pulled by winit's
  Wayland decorations) and the older zbus crates are unmaintained. Those
  warnings do not fail the audit.

CI runs EmbarkStudios/cargo-deny-action, pinned to a commit, and
`cargo audit` from cargo-audit 0.22.2. The RustSec GitHub Action installs
whatever cargo-audit is newest, which failed on the previous 1.83 pin
(`cargo-audit` 0.22.2 needs Rust 1.88). The workflow now installs that
version with `--locked` on the 1.91 compiler.

Exact versions in the workspace `Cargo.toml` stay exact after the move to
Rust 1.91. Crates that look unused (`base64ct`, `hashbrown`, `indexmap`,
`zeroize_derive`, `tempfile`, `toml_edit`) still pin transitive releases.

Not in CI yet: CodeQL, gitleaks, OSV-Scanner, cargo-machete, cargo-semver-checks,
SBOM, and signed release artifacts.
