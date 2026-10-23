# Supply chain

`deny.toml` is the cargo-deny 0.20 policy:

- Licenses outside the allow-list fail.
- Wildcard dependency requirements fail.
- Git and unknown registries fail.
- Advisory and yanked-crate findings warn or fail according to the file.
- Multiple versions warn. They are expected while a transitive crate has not
  caught up; they are not a reason to widen our pins onto Rust 1.85.

CI runs EmbarkStudios/cargo-deny-action and rustsec/audit-check. Both actions
are pinned to commits.

Exact versions in the workspace `Cargo.toml` are an MSRV lock for Rust 1.83,
which does not have the MSRV-aware resolver. Crates that look unused
(`base64ct`, `hashbrown`, `indexmap`, `zeroize_derive`, `tempfile`,
`toml_edit`) pin transitive releases. Removing them re-resolves a crate that
needs edition 2024.

Not in CI yet: CodeQL, gitleaks, OSV-Scanner, cargo-machete, cargo-semver-checks,
SBOM, and signed release artifacts.
