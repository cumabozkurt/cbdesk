# M0 status

This note is the status of the master plan. The plan text in
`docs/MASTER_PLAN.md` is unchanged, including its checkboxes.

The repository is not a RustDesk fork. ADR-002 records why the empty tree
starts at the modular crate layout instead of importing that history in this
pass. Nothing in the Rust sources is derived from that project.

## Done in this pass

- Master plan stored at `docs/MASTER_PLAN.md`.
- Cargo workspace, Rust 1.91 pin (ADR-006), rustfmt, clippy, and `cargo-deny` policy.
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
  short codes, LAN packet and mDNS TXT shape, runtime announce/browse on those
  encodings, DHT records. Blinded publishing returns an error. Public networks
  do not send.
- `cbdesk-net`: `Transport` trait, in-memory connections, candidate ranking,
  reconnect backoff, path state machine, relay suffix policy, direct QUIC,
  ticket dial, and the consent handshake.
- `cbdesk-session`: consent delay, 30 second approval timeout, unattended
  rules, quick-support ban, high-risk opt-in, indicator that cannot be
  disabled, auth rate limits.
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
  `cbdesk-crypto` and `cbdesk-proto`; those pins keep transitive crates on
  releases this tree has compiled. iroh adds a second `ed25519-dalek` and a
  second `getrandom`, which `cargo-deny` warns about and does not fail.
- No second maintainer, so the two-reviewer rule for crypto and networking is
  not enforceable yet.
- No external security review. The threat model is a draft written with the
  code.

## M1 items started

- Device-id generation in process. No OS keychain.
- Session messages for hello, grant, ticket, and DHT. No video or input path.
- `QuicEndpoint` on iroh 1.4.1 via `presets::Minimal` and
  `RelayMode::Disabled`. No n0 discovery service is installed. T-NET-001 is a
  loopback echo; the endpoint resolver must record no DNS names. It is not a
  packet capture of the whole machine.
- A verified invite ticket dials `direct_addrs` through that endpoint. Relay
  URLs on the ticket are not contacted. The connection is handed to
  `HostSession`. The ticket is not a grant.
- LAN announce and browse. Public networks do not send. The datagrams are the
  tested mDNS TXT record and the `CBDS` broadcast body. CI uses loopback
  sockets, not port 5353 and not the multicast group.
- Consent policy in the library, including the 30 second approval timeout.
  No window. The production wait is `wait_for_approval`. Loopback tests fire
  that same close path without sleeping 30 wall-clock seconds.
- Synthetic capturer. No VP8 encoder and no Linux/Windows end-to-end run.
  A grant still does not send a frame.

## Test ids this pass actually locks

| Id | What the test covers | What it does not cover |
| --- | --- | --- |
| T-NET-001 | Loopback QUIC echo on an endpoint with no n0 relays, no n0 discovery, and a resolver that recorded no names. Forbidden relay suffixes still fail before bind. A verified ticket dial does not query DNS for its relay URL | A capture of every packet the runner sends, including traffic to GitHub |
| T-AUTH-018 | An unanswered approval closes at 30 seconds and the controller receives `TIMEOUT_USER_RESPONSE`. The 30 second sleep is the production wait; the loopback test uses that close path with a ready deadline | A visible approval window |
| T-AUTH-011 | Fifth failure locks 60 seconds, later failures double up to one hour | Password UI |
| T-AUTH-012 | Counters are per device id and per IPv4 /24 or IPv6 /56 | Distributed clients |
| T-AUTH-015 | A known trusted key can skip the prompt when the policy says so, and the grant is the safe set | A real trust store |
| T-AUTH-019 | One consent prompt per minute; three rejects silence the peer for 24 hours | A real window |
| T-PERM-002 | `authorize` refuses a permission that was not granted | File messages |
| T-PERM-006 | Media and `authorize` fail before the grant. An echo probe is answered with `PROTOCOL_VIOLATION` and is not reflected | A video stream |
| T-UI-004 | First contact cannot be approved before 3 seconds | A widget |
| T-UI-005 | `indicator_can_be_disabled` is constant false | Settings UI, CLI |
| T-UI-007 / T-I18N-001 | Catalog keys match across Turkish and English, including every known error | A rendered UI |
| T-I18N-004 | Turkish short-code casing | A spoken usability test |

## Recommended next run

Keep the fork import as its own reviewable change. Next, after `authorize(View)`
succeeds, send one synthetic barcode frame on its own QUIC stream. Before the
grant, that stream is `PROTOCOL_VIOLATION`. No operating-system capture, no
encoder, and no RustDesk import in that run.

Still out of that run: SPAKE2, OPAQUE, DHT blinding, and the fork import.
ADR-007 and ADR-008 are still Proposed. The OS keychain and a multicast join
on port 5353 can follow the synthetic frame.
