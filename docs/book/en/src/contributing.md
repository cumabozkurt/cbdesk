# Contributing

Thank you for looking at CBDesk. The full note is
[CONTRIBUTING.md](https://github.com/cumabozkurt/cbdesk/blob/main/CONTRIBUTING.md).
The plan is
[docs/MASTER_PLAN.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/MASTER_PLAN.md).
The milestone ledger is
[docs/dev/m0-status.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/m0-status.md).

The license is AGPL-3.0-or-later. There is no CLA. A commit needs a
`Signed-off-by` trailer, which is the Developer Certificate of Origin:

<https://developercertificate.org/>

```sh
git commit -s
```

Rust 1.91.0 is pinned. Public items need documentation. `unsafe` is
forbidden in the workspace. Do not invent a cryptographic construction.
Do not add a hidden session, a way to turn the indicator off, or a path
that treats a ticket, a short code, or a DHT record as authorization.

The user guide in this book is built with mdBook 0.5.4:

```sh
tools/docs/build-book.sh
python3 tools/docs/check-book-links.py site
```
