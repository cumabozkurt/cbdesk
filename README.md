# CBDesk

CBDesk, merkeziyetsiz, sunucusuz ve açık kaynak bir uzak masaüstü uygulaması olmak üzere tasarlanmıştır. Çekirdek Rust, arayüz hedefi Flutter, lisans AGPL-3.0-or-later'dır.

CBDesk is a decentralized, serverless, open-source remote desktop application under design. The core is Rust, the UI target is Flutter, and the license is AGPL-3.0-or-later.

## Bu depo şu an ne değildir / What this repository is not yet

`cbdesk host` ve `cbdesk view` aynı yerel ağda, onaydan sonra gri tonlu bir oturum açar. Varsayılan `--frames 0` izleyici kapanana kadar akar. `--window` kareleri bir pencerede gösterir; işaretçi ve klavye yalnız `input` izninden sonra gider. `--ppm` ilk kareyi dosyaya yazar. `--lan` ve `--browse` 5353 üzerinden mDNS kullanır; port doluysa gönderim 5353'e devam eder, dinleme geçici porta düşer. Kimlik işletim sistemi anahtarlığında durur; anahtarlık yanıt vermezse süreç yeni bir kimliği yalnız bellekte tutar. Pano düz metindir, `clipboard` izni ister ve 60 KiB ile sınırlıdır. `--synthetic` barkod üretir. `--screen` ekranı okur: Linux'ta X11, `WAYLAND_DISPLAY` varken Wayland ScreenCast portalı ve PipeWire, Windows ve macOS'ta aynı yakalama kitaplığı. Kodlayıcı oxideav-vp8 ana kareleridir; her kare kendi ana karesidir, ara kare yoktur. AV1 yedek olarak durur. `cargo test` kimlik, protokol, onay, döngüsel QUIC, bilet çevirmesi, VP8 barkod akışı, pencere duman testi, pano izni ve aynı makinede mDNS ile bir PPM turu sınar. Bilet erişim vermez. Onay 30 saniye yanıtsız kalırsa bağlantı kapanır. Yankı sondası oturum açmaz. İzin verilmeden açılan bir yan akış `PROTOCOL_VIOLATION` olur. Ses, Flutter arayüzü, donanım kodlayıcı, dosya aktarımı, NAT deliği, aktarıcı ve internet üzerinden keşif sonraki dilimlerdir.

`cbdesk host` and `cbdesk view` can open a grayscale session on the local network after consent. The default `--frames 0` keeps sending until the viewer closes. `--window` shows the frames; the pointer and keyboard are forwarded only after an `input` grant. `--ppm` writes the first frame to a file. `--lan` and `--browse` use mDNS on port 5353; when that port is busy, sends still aim at 5353 and the socket listens on an ephemeral port. The device seed is stored in the OS keychain; when that store does not answer, the process keeps a new identity only in memory. Clipboard sync is plain text, requires a `clipboard` grant, and stops at 60 KiB. `--synthetic` paints a barcode. `--screen` reads the display: X11 on Linux, the Wayland ScreenCast portal and PipeWire when `WAYLAND_DISPLAY` is set, and the same capture library on Windows and macOS. The codec is oxideav-vp8 keyframes; each frame is its own keyframe, and there are no inter frames. AV1 remains the fallback. `cargo test` covers identity, the protocol, consent, a loopback QUIC echo, ticket dial, a VP8 barcode stream, a window smoke test, the clipboard grant, and an mDNS browse that writes a PPM on the same machine. The ticket does not grant access. An approval that nobody answers within 30 seconds closes. An echo probe does not open a session. A side stream opened before the grant is `PROTOCOL_VIOLATION`. Audio, the Flutter UI, a hardware codec, file transfer, NAT traversal, a relay, and discovery across the internet are later slices.

Bu kod RustDesk'ten türetilmemiştir. [ADR-015](docs/adr/015-clean-room-media.md) çatalı almamayı seçer; yakalama, kodlayıcı ve girdi temiz oda çekirdeğinde kalır. Sıralama kaydı [ADR-002](docs/adr/002-fork-sequencing.md) içindedir. M0'ın "çatal üç masaüstünde derlenir" çıkış ölçütü bu yüzden karşılanmamıştır. `tools/rebrand-check.sh` o adı yalnız atıf dosyalarında tutar.

This code is not derived from RustDesk. [ADR-015](docs/adr/015-clean-room-media.md) declines the fork and keeps capture, the codec, and input in this clean-room core. The sequencing record is [ADR-002](docs/adr/002-fork-sequencing.md). M0's "the fork builds on three desktops" exit stays unmet because of that decision. `tools/rebrand-check.sh` keeps that name in attribution files only.

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
