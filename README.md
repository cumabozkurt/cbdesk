# CBDesk

CBDesk, merkeziyetsiz, sunucusuz ve açık kaynak bir uzak masaüstü uygulaması olmak üzere tasarlanmıştır. Çekirdek Rust, arayüz hedefi Flutter, lisans AGPL-3.0-or-later'dır.

CBDesk is a decentralized, serverless, open-source remote desktop application under design. The core is Rust, the UI target is Flutter, and the license is AGPL-3.0-or-later.

## Bu depo şu an ne değildir / What this repository is not yet

`cbdesk host` ve `cbdesk view` onaydan sonra aynı yerel ağda bir oturum açar. Görüntü renklidir (YUV). Sentetik kaynak gri bir barkoddur. Varsayılan `--frames 0` izleyici kapanana kadar akar. Canlı yol, yakalanan görüntünün çift enini ve boyunu korur. Tempo 33 ms'dir: kodlama bu süreyi aşarsa VP8 niceliği yükselir ve sonraki aralık atlanır. Bu depoda 1080p süre ölçümü yoktur. `--window` kareleri gösterir. Betikli bir işaretçi ve Enter, `input` izninden sonra ana makinenin stdout satırı olur; pencere aynı eşlemeyi kullanır. `--ppm` ilk kareyi dosyaya yazar. `--lan` ve `--browse` 5353 üzerinden mDNS kullanır; port doluysa gönderim 5353'e devam eder, dinleme geçici porta düşer. Aynı makinede gözatma, çoklu yayın kaynağından önce döngü adresini dener. Kimlik işletim sistemi anahtarlığında durur; anahtarlık yanıt vermezse süreç yeni bir kimliği yalnız bellekte tutar. Pano düz metindir, `clipboard` izni ister ve 60 KiB ile sınırlıdır. `--synthetic` barkod üretir. `--screen` ekranı okur: Linux'ta X11, `WAYLAND_DISPLAY` varken Wayland ScreenCast portalı ve PipeWire, Windows ve macOS'ta aynı yakalama kitaplığı. Kodlayıcı oxideav-vp8'tir. İlk kare ana karedir; sonrakiler sıfır hareketli ara kare olabilir. Birim testi barkodu bir ara karede de okur. AV1 durağan yedektir. `--put`, en çok 32 MiB'lik bir dosyayı `--files-root` içine koyar. `file_write` yüksek risklidir ve `--allow-high-risk` ister. Kitaplık yarım dosyadan ve parça bitmap'inden sürer. zstd bağlı değildir; parçalar sıkıştırmasızdır. `--tone`, `audio` izninden sonra her tempo aralığında bir sentetik Opus paketi gönderir. Mikrofon açılmaz. `--relay URL` operatörün yazdığı aktarıcıyı kurar. Bilinen genel aktarıcı sonekleri reddedilir. Gömülü bir liste yoktur ve bu proje bir aktarıcı süreci çalıştırmaz. Döngüsel aktarıcı testi, UDP kapalıyken bir sondayı taşır. Simetrik NAT ölçülmemiştir. `cargo test` kimlik, protokol, onay, döngüsel QUIC, bilet çevirmesi, ara kareli VP8 barkodu, pencere duman testi, pano izni, dosya koyma, Opus tonu, kullanıcı aktarıcı sondası ve bu Linux makinesinde mDNS ile bir PPM turu sınar. Bilet erişim vermez. Onay 30 saniye yanıtsız kalırsa bağlantı kapanır. Yankı sondası oturum açmaz. İzin verilmeden açılan bir yan akış `PROTOCOL_VIOLATION` olur. Flutter arayüzü, donanım kodlayıcı, mikrofon ve simetrik NAT ölçümü sonraki dilimlerdir.

`cbdesk host` and `cbdesk view` can open a session on the local network after consent. The picture is color (YUV). The synthetic source is still a gray barcode. The default `--frames 0` keeps sending until the viewer closes. The live path keeps the capture's even width and height. The pacer targets 33 ms: an encode that misses that raises the VP8 quantizer and skips the next slot. This tree has no 1080p timing measurement. `--window` shows the frames. A scripted pointer and Enter become host stdout lines after an `input` grant; the window uses that same mapping. `--ppm` writes the first frame. `--lan` and `--browse` use mDNS on port 5353; when that port is busy, sends still aim at 5353 and the socket listens on an ephemeral port. Same-machine browse tries loopback before the multicast source address. The device seed is stored in the OS keychain; when that store does not answer, the process keeps a new identity only in memory. Clipboard sync is plain text, requires a `clipboard` grant, and stops at 60 KiB. `--synthetic` paints a barcode. `--screen` reads the display: X11 on Linux, the Wayland ScreenCast portal and PipeWire when `WAYLAND_DISPLAY` is set, and the same capture library on Windows and macOS. The codec is oxideav-vp8. The first frame is a keyframe and a later frame can be a zero-motion inter frame. A unit test still reads the barcode on that inter frame. AV1 remains a still-picture fallback. `--put` stores one file of at most 32 MiB under `--files-root`. `file_write` is high-risk and needs `--allow-high-risk`. The library resumes from a partial file and a chunk bitmap. zstd is not linked, so chunks are uncompressed. `--tone` sends one synthetic Opus packet per paced slot after an `audio` grant. No microphone is opened. `--relay URL` installs a relay the operator named. Known public-relay suffixes are refused. There is no built-in list, and this project does not run a relay process. A loopback relay test carries one probe with UDP off. Symmetric NAT is unmeasured. `cargo test` covers identity, the protocol, consent, a loopback QUIC echo, ticket dial, a VP8 barcode that includes an inter frame, a window smoke test, the clipboard grant, a file put, an Opus tone, a user-relay probe, and an mDNS browse that writes a PPM on this Linux machine. The ticket does not grant access. An approval that nobody answers within 30 seconds closes. An echo probe does not open a session. A side stream opened before the grant is `PROTOCOL_VIOLATION`. The Flutter UI, a hardware codec, a microphone, and a symmetric-NAT measurement are later slices.

Bu kod RustDesk'ten türetilmemiştir. [ADR-015](docs/adr/015-clean-room-media.md) çatalı almamayı seçer; yakalama, kodlayıcı ve girdi temiz oda çekirdeğinde kalır. Sıralama kaydı [ADR-002](docs/adr/002-fork-sequencing.md) içindedir. M0'ın "çatal üç masaüstünde derlenir" çıkış ölçütü bu yüzden karşılanmamıştır. `tools/rebrand-check.sh` o adı yalnız atıf dosyalarında tutar.

This code is not derived from RustDesk. [ADR-015](docs/adr/015-clean-room-media.md) declines the fork and keeps capture, the codec, and input in this clean-room core. The sequencing record is [ADR-002](docs/adr/002-fork-sequencing.md). M0's "the fork builds on three desktops" exit stays unmet because of that decision. `tools/rebrand-check.sh` keeps that name in attribution files only.

## Sunucusuz ne demektir / What "serverless" means

CBDesk'in kendi hesabı, kendi kimlik sunucusu veya kendi zorunlu aktarıcısı yoktur. İki cihaz doğrudan bağlanabilir. `--relay`, operatörün çalıştırdığı bir aktarıcıyı adlandırabilir. Bu depo o süreci başlatmaz ve genel bir aktarıcı listesi taşımaz. Aktarıcı içeriği okuyamaz; uçtan uca şifreleme zorunludur. Genel anahtar körleştirmesi henüz yoktur: DHT kaydı bu sürümde uzun dönem anahtarla imzalanır ve kör yayın kapalıdır.

CBDesk has no account, no project identity server, and no mandatory relay. Two devices can connect directly. `--relay` can name a relay the operator runs. This repository does not start that process and does not ship a public relay list. A relay must not be able to read the session; end-to-end encryption is mandatory. Public-key blinding is not implemented: DHT records in this tree are signed with the long-term key, and blinded publishing fails closed.

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
