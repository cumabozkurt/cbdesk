# CBDesk

CBDesk, merkeziyetsiz, sunucusuz ve açık kaynak bir uzak masaüstü uygulaması olmak üzere tasarlanmıştır. Çekirdek Rust, arayüz hedefi Flutter, lisans AGPL-3.0-or-later'dır.

CBDesk is a decentralized, serverless, open-source remote desktop application under design. The core is Rust, the UI target is Flutter, and the license is AGPL-3.0-or-later.

## Bu depo şu an ne değildir / What this repository is not yet

Bu ağaç çalışan bir uzak masaüstü değildir. Pencere, ses, donanım kodlayıcı ve Flutter arayüzü yoktur. `cbdesk host` ile `cbdesk view`, onaydan sonra tek bir AV1 karesini PPM dosyasına yazar. `--synthetic` barkod üretir. `--screen` işletim sistemi ekranını okur: Linux'ta X11, `WAYLAND_DISPLAY` varken Wayland ScreenCast portalı ve PipeWire, Windows ve macOS'ta aynı yakalama kitaplığı. Girdi enjeksiyonu yalnız `input` izni verildikten sonra vardır; testler işaretçiyi oynatmaz. `cargo test` kimlik, protokol, onay politikası, döngüsel bir QUIC yankısı, doğrulanmış bir davet biletinin doğrudan adresinden açılan oturum, izin sonrası kare ve girdi akışı, AV1 barkod turu ve aynı makinede LAN duyurusu/tarama sınar. Bilet erişim vermez. Onay 30 saniye yanıtsız kalırsa bağlantı kapanır. Yankı sondası oturum açmaz. İzin verilmeden açılan bir yan akış `PROTOCOL_VIOLATION` olur. NAT deliği, aktarıcı ve internet üzerinden keşif yoktur. Kodlayıcı VP8 değil, AV1'dir.

This tree is not a usable remote desktop. There is no window, no audio, no hardware codec, and no Flutter UI. `cbdesk host` and `cbdesk view` write one AV1 frame to a PPM file after consent. `--synthetic` paints a barcode. `--screen` reads the operating-system display: X11 on Linux, the Wayland ScreenCast portal and PipeWire when `WAYLAND_DISPLAY` is set, and the same capture library on Windows and macOS. Input injection exists only after an `input` grant; tests do not move the pointer. `cargo test` covers identity, the protocol codec, the consent policy, a loopback QUIC echo, a session dialed from a verified invite ticket's direct addresses, a frame and an input event after those grants, an AV1 barcode round trip, and LAN announce/browse between sockets on the same machine. The ticket does not grant access. An approval that nobody answers within 30 seconds closes. An echo probe does not open a session. A side stream opened before the grant is `PROTOCOL_VIOLATION`. There is no NAT traversal, no relay, and no discovery across the internet. The codec is AV1, not VP8.

Bu kod RustDesk'ten türetilmemiştir. [ADR-015](docs/adr/015-clean-room-media.md) çatalı almamayı seçer; yakalama, AV1 ve girdi temiz oda çekirdeğinde kalır. Sıralama kaydı [ADR-002](docs/adr/002-fork-sequencing.md) içindedir. M0'ın "çatal üç masaüstünde derlenir" çıkış ölçütü bu yüzden karşılanmamıştır. `tools/rebrand-check.sh` o adı yalnız atıf dosyalarında tutar.

This code is not derived from RustDesk. [ADR-015](docs/adr/015-clean-room-media.md) declines the fork and keeps capture, AV1, and input in this clean-room core. The sequencing record is [ADR-002](docs/adr/002-fork-sequencing.md). M0's "the fork builds on three desktops" exit stays unmet because of that decision. `tools/rebrand-check.sh` keeps that name in attribution files only.

## Sunucusuz ne demektir / What "serverless" means

CBDesk'in kendi hesabı, kendi kimlik sunucusu veya kendi zorunlu aktarıcısı yoktur. İki cihaz doğrudan bağlanabilir. Doğrudan yol kurulamazsa, kullanıcının seçtiği bir topluluk aktarıcısı devreye girebilir. Aktarıcı içeriği okuyamaz; uçtan uca şifreleme zorunludur. Genel anahtar körleştirmesi henüz yoktur: DHT kaydı bu sürümde uzun dönem anahtarla imzalanır ve kör yayın kapalıdır.

CBDesk has no account, no project identity server, and no mandatory relay. Two devices can connect directly. When they cannot, a community relay the user chose may carry the packets. A relay must not be able to read the session; end-to-end encryption is mandatory. Public-key blinding is not implemented: DHT records in this tree are signed with the long-term key, and blinded publishing fails closed.

## Dolandırıcılık uyarısı / Fraud warning

Sizi telefonla arayıp bu programı kurmanızı isteyen kişiye güvenmeyin. Bankalar, kamu kurumları ve destek şirketleri uzaktan erişim istemez. Bağlantıyı reddedin.

If someone called you and told you to install this program, deny the connection. Banks, government agencies, and support companies do not ask you for remote access.

## Onay / Consent

Oturum gizli tutulamaz. Oturum göstergesinin kapatma anahtarı yoktur. Davet bileti yer tarifidir; erişim vermez. Onay penceresi 30 saniye içinde yanıtlanmazsa bağlantı reddedilir. Hızlı Destek gözetimsiz erişim açamaz. İlk temasta dosya yazma, uçbirim, yerel girdiyi engelleme ve gizlilik modu ayrıca seçilmeden verilmez.

A session cannot be hidden. The session indicator has no off switch. An invite ticket is a locator, not an authorization. If the approval window gets no answer within 30 seconds, the connection is refused. Quick Support cannot enable unattended access. On a first contact, file write, terminal, blocking local input, and privacy mode stay off unless the person at the host opts in.

## Derleme / Build

Rust 1.91.0 (`rust-toolchain.toml`). iroh 1.4.1 needs that compiler. See [ADR-006](docs/adr/006-transport-iroh.md).

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Ayrıntı: [docs/dev/building.md](docs/dev/building.md). Durum: [docs/dev/m0-status.md](docs/dev/m0-status.md). Plan: [docs/MASTER_PLAN.md](docs/MASTER_PLAN.md).

## Lisans / License

[AGPL-3.0-or-later](LICENSE). Katkılar DCO (`Signed-off-by`) ile gelir; CLA yoktur. Bkz. [CONTRIBUTING.md](CONTRIBUTING.md).
