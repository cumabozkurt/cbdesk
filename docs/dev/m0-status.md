# M0 status

This note is the status of the master plan. The plan text in
`docs/MASTER_PLAN.md` is unchanged, including its checkboxes.

The repository is not a RustDesk fork. ADR-002 records why the empty tree
starts at the modular crate layout instead of importing that history in this
pass. Nothing in the Rust sources is derived from that project.

## Done in this pass

- Master plan stored at `docs/MASTER_PLAN.md`.
- Cargo workspace, Rust 1.83 pin, rustfmt, clippy, and `cargo-deny` policy.
- GitHub Actions: rustfmt, cargo-deny, RustSec audit, DCO, catalog check,
  rebrand scan, and clippy plus tests on `ubuntu-24.04`, `windows-2022`, and
  `macos-14`.
- Contribution documents: `CONTRIBUTING.md`, `GOVERNANCE.md`, `SECURITY.md`,
  `PRIVACY.md`, `TRADEMARK.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, `NOTICE`.
- ADRs 001–014.
- Threat model v0.1 and a protocol note for the messages that exist.
- Turkish and English catalogs for consent, the fraud warning, and every
  `ErrorCode` string.
- `cbdesk-proto`: hand-written protobuf codec for `cbdesk/1`, unknown fields
  preserved, size limits, no hidden-session flag.
- `cbdesk-crypto`: Ed25519 device ids, z-base-32, short fingerprint, X25519,
  HKDF, Argon2id backup parameters. RFC test vectors are locked in tests.
- `cbdesk-discovery`: invite tickets (15 minute cap, replay cache, no grant),
  short codes, LAN packet and mDNS TXT shape, DHT records. Blinded publishing
  returns an error.
- `cbdesk-net`: `Transport` trait, in-memory connections, candidate ranking,
  reconnect backoff, path state machine, relay suffix policy.
- `cbdesk-session`: consent delay, unattended rules, quick-support ban,
  high-risk opt-in, indicator that cannot be disabled, auth rate limits.
- `cbdesk-testkit`: synthetic barcode frames, recording injector, poisonable
  fake DHT.
- Netlab profile files for N0, N3, and N4. They do not create namespaces.
- Crate directories from the layout that have no code yet contain a README
  and are not workspace members.

## M0 items still open

The M0 exit criterion is not met. The plan's exit is a fork that builds on
three desktops. There is no fork here. CI being green on this workspace is the
substitute, not that exit.

- GitHub organization, branch protection, the DCO GitHub App, and Discussions.
  The owner checklist is `docs/dev/github-setup.md`.
- Trademark search (R-012). `docs/brand/` says the search was not done.
- UI wireframes S-01 through S-05.
- Import of the other code base, copyright headers from that import, and a
  build of that imported tree.
- `ubuntu-22.04`, Linux ARM, Windows ARM, macOS x86_64, Android, and iOS are
  not in CI.
- CodeQL, gitleaks, OSV-Scanner, `cargo-machete`, and `cargo-semver-checks`
  are not wired. Machete would also flag the intentional version pins in
  `cbdesk-crypto` and `cbdesk-proto`; those pins exist so Rust 1.83 can still
  resolve the graph.
- No second maintainer, so the two-reviewer rule for crypto and networking is
  not enforceable yet.
- No external security review. The threat model is a draft written with the
  code.

## M1 items started

- Device-id generation in process. No OS keychain.
- Session messages for hello, grant, ticket, and DHT. No video or input path.
- Transport trait and policy tests. iroh is not a dependency. T-NET-001 does
  not capture packets; it rejects forbidden relay hosts before a connection
  exists.
- Consent policy in the library. No window, no 30 second approval timeout.
- Synthetic capturer. No VP8 encoder and no Linux/Windows end-to-end run.

## Test ids this pass actually locks

| Id | What the test covers | What it does not cover |
| --- | --- | --- |
| T-NET-001 | Default endpoint is empty and forbidden relay suffixes are rejected before a pipe is offered | Packet capture of a live stack |
| T-AUTH-011 | Fifth failure locks 60 seconds, later failures double up to one hour | Password UI |
| T-AUTH-012 | Counters are per device id and per IPv4 /24 or IPv6 /56 | Distributed clients |
| T-AUTH-015 | A known trusted key can skip the prompt when the policy says so, and the grant is the safe set | A real trust store |
| T-AUTH-019 | One consent prompt per minute; three rejects silence the peer for 24 hours | A real window |
| T-PERM-002 | `authorize` refuses a permission that was not granted | File messages |
| T-PERM-006 | Media and `authorize` fail before the grant | Closing the socket with `PROTOCOL_VIOLATION` |
| T-UI-004 | First contact cannot be approved before 3 seconds | A widget |
| T-UI-005 | `indicator_can_be_disabled` is constant false | Settings UI, CLI |
| T-UI-007 / T-I18N-001 | Catalog keys match across Turkish and English, including every known error | A rendered UI |
| T-I18N-004 | Turkish short-code casing | A spoken usability test |

## Recommended next run

Keep the fork import as its own reviewable change. The next implementation
run should pick one of these, not all of them:

1. Raise the Rust pin only as far as a current `iroh` release requires, record
   that in an ADR update, and add an endpoint built with the empty builder.
   Extend T-NET-001 from the configuration check to a test that fails if that
   endpoint is constructed with a forbidden relay. Do not add a packet-capture
   harness until the endpoint can actually open a socket.
2. Runtime LAN discovery on top of the packet format already tested: mDNS
   browse/announce for `_cbdesk._udp.local.` and the UDP broadcast on port
   22118, still with public-network suppression.
3. A small Flutter or even CLI consent prompt that calls `HostSession` and
   cannot render a hidden session. No screen capture in that run.

Do not implement SPAKE2, OPAQUE, or DHT blinding in that run. ADR-007 and
ADR-008 are still Proposed.
