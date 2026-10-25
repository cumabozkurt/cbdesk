# Contributing

Thank you for looking at CBDesk. The master plan is `docs/MASTER_PLAN.md`.
Status of the current milestone is `docs/dev/m0-status.md`.

Katkılar için teşekkürler. Plan `docs/MASTER_PLAN.md` dosyasındadır. Bu
kilometre taşının durumu `docs/dev/m0-status.md` içindedir.

## License and DCO

By adding a `Signed-off-by` trailer you certify the Developer Certificate of
Origin 1.1:

https://developercertificate.org/

There is no CLA. The license is AGPL-3.0-or-later. Do not send a contribution
that you cannot license that way.

```sh
git commit -s
```

## Setup

Rust 1.91.0 is pinned in `rust-toolchain.toml`. See `docs/dev/building.md`.

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

## Code

- New Rust files start with `// SPDX-License-Identifier: AGPL-3.0-or-later`.
- Public items need documentation. `missing_docs` is denied.
- `unsafe` is forbidden in the workspace.
- Library code denies `unwrap` and `expect` outside tests.
- Do not add a dependency that needs a newer Rust than 1.91 without an ADR.
  Versions in the workspace `Cargo.toml` are exact on purpose. ADR-006 records
  why the pin is 1.91.
- Do not invent a cryptographic construction. Use the crates named in ADR-005,
  or write a Proposed ADR first.
- Secrets use `Redacted` or an equivalent that does not implement `Display`.

## Safety bar

A change is in scope only if the person at the host stays in control.

- No stealth flag, hidden session, or indicator switch.
- An invite ticket does not grant access.
- Quick Support cannot enable unattended access.
- `approval = "never"` is invalid unless unattended access is explicitly on.
- First-contact high-risk permissions (file write, terminal, block input,
  privacy mode) require an explicit opt-in.
- Default endpoints do not use a vendor relay list.

## Pull requests

Use the pull request template. Name the master-plan feature or test id when
one applies. Tests that only lock a policy should say which part of the
Definition of Done is still open (UI, threat-model review, translation, or a
manual platform check).

## Translations

`app/i18n/en.json` and `app/i18n/tr.json` must have the same keys.
`tools/i18n/check.sh` enforces that. Turkish casing for short codes is
`i → İ` and `ı → I`.

## Reporting vulnerabilities

See `SECURITY.md`. Do not file a public issue for a vulnerability.
