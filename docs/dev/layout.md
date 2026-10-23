# Repository layout

The workspace members are the crates that compile:

| Path | Role in this milestone |
| --- | --- |
| `crates/cbdesk-proto` | `cbdesk/1` messages and limits |
| `crates/cbdesk-crypto` | Device identity and the KDFs this tree is willing to call |
| `crates/cbdesk-discovery` | Tickets, short codes, LAN records, DHT signatures |
| `crates/cbdesk-net` | Transport trait, memory hub, path and relay policy |
| `crates/cbdesk-session` | Host consent, grants, rate limits |
| `crates/cbdesk-capture` | Synthetic barcode and operating-system screen capture |
| `crates/cbdesk-codec` | VP8 inter frames, color, and AV1 stills |
| `crates/cbdesk-input` | Keyboard and pointer injection after an Input grant |
| `crates/cbdesk-fs` | Jailed resumable file transfer, uncompressed |
| `crates/cbdesk-audio` | Opus, cpal devices, and a jitter buffer |
| `crates/cbdesk-desk` | Desktop session state behind the shell |
| `crates/cbdesk-ffi` | flutter_rust_bridge surface over the desk |
| `crates/cbdesk-cli` | `cbdesk host` and `cbdesk view`, including the window |
| `crates/cbdesk-testkit` | Re-exported barcode frames, recording injector, fake DHT |

Other crate directories match the master plan and are not members. Adding one
to `[workspace].members` is part of implementing it, not part of creating the
directory.

| Path | Contents |
| --- | --- |
| `proto/cbdesk/v1/session.proto` | Readable schema. The Rust codec is what CI tests |
| `app/i18n/` | Turkish and English JSON catalogs |
| `app/desktop/` | Flutter shell, ARB catalogs, and the cargokit plugin |
| `docs/adr/` | Decision records |
| `docs/protocol/` | Protocol note for the implemented messages |
| `tools/netlab/profiles/` | N0, N3, N4 descriptions. No namespaces |
| `supply-chain/` | How to read `deny.toml` |

Generated files (`target/`) stay untracked. `Cargo.lock` is tracked.
