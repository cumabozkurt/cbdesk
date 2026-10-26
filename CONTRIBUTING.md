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

Çekme isteği şablonunu kullanın. Uygunsa ana plandaki özellik veya test
kimliğini yazın. Yalnız bir ilkeyi kilitleyen testler, Bitti tanımının
hangi parçasının açık kaldığını söyler (arayüz, tehdit modeli incelemesi,
çeviri veya elle platform denetimi).

## Issues

Use the bug or feature template. Do not put a device seed, a short-code
password, a live ticket, or a vulnerability in a public issue. Security
reports go through `SECURITY.md`.

Hata veya özellik şablonunu kullanın. Herkese açık bir konuya cihaz tohumu,
kısa kod parolası, canlı bir bilet veya bir açığı yazmayın. Güvenlik
bildirimleri `SECURITY.md` üzerinden gider.

## Documentation

User-facing claims have to match `docs/dev/m0-status.md`. The English
changelog is `CHANGELOG.md`. `CHANGELOG.tr.md` is the Turkish copy; if the
two drift, the English file is the record. The Pages site is built from
`docs/book` and does not host a binary. Do not commit a drawn window as a
screenshot.

Kullanıcının göreceği iddialar `docs/dev/m0-status.md` ile aynı olmalıdır.
İngilizce günlük `CHANGELOG.md` dosyasıdır. `CHANGELOG.tr.md` onun
Türkçesidir; ayrışırlarsa İngilizce dosya esas alınır. Pages sitesi
`docs/book` kaynağından derlenir ve ikili dosya barındırmaz. Çizilmiş bir
pencereyi ekran görüntüsü diye koymayın.

## Translations

`app/i18n/en.json` and `app/i18n/tr.json` must have the same keys.
`tools/i18n/check.sh` enforces that. Turkish casing for short codes is
`i → İ` and `ı → I`.

## Reporting vulnerabilities

See `SECURITY.md`. Do not file a public issue for a vulnerability.
