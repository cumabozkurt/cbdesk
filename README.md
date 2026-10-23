# CBDesk

CBDesk, merkeziyetsiz, sunucusuz ve açık kaynak bir uzak masaüstü uygulaması olmak üzere tasarlanmıştır. Çekirdek Rust, arayüz hedefi Flutter, lisans AGPL-3.0-or-later'dır.

CBDesk is a decentralized, serverless, open-source remote desktop application under design. The core is Rust, the UI target is Flutter, and the license is AGPL-3.0-or-later.

## Bu depo şu an ne değildir / What this repository is not yet

Bu ağaç çalışan bir uzak masaüstü değildir. Ekran yakalama, girdi ve arayüz yoktur. `cargo test` kimlik, protokol, onay politikası, döngüsel bir QUIC yankısı ve aynı makinede LAN duyurusu/tarama sınar. NAT deliği, aktarıcı ve internet üzerinden keşif yoktur.

This tree is not a usable remote desktop. There is no screen capture, input injection, or UI. `cargo test` covers identity, the protocol codec, the consent policy, a loopback QUIC echo, and LAN announce/browse between sockets on the same machine. There is no NAT traversal, no relay, and no discovery across the internet.

Bu kod RustDesk'ten türetilmemiştir. Çatal ve marka temizliği bu turda yapılmadı. Sıralama kararı [ADR-002](docs/adr/002-fork-sequencing.md) içindedir. `tools/rebrand-check.sh`, ileride içeri alınacak kodda o adın yalnız atıf dosyalarında kalmasını denetler.

This code is not derived from RustDesk. The fork and rebrand were not done in this pass. The sequencing decision is [ADR-002](docs/adr/002-fork-sequencing.md). `tools/rebrand-check.sh` keeps that name out of new code except attribution files.

## Sunucusuz ne demektir / What "serverless" means

CBDesk'in kendi hesabı, kendi kimlik sunucusu veya kendi zorunlu aktarıcısı yoktur. İki cihaz doğrudan bağlanabilir. Doğrudan yol kurulamazsa, kullanıcının seçtiği bir topluluk aktarıcısı devreye girebilir. Aktarıcı içeriği okuyamaz; uçtan uca şifreleme zorunludur. Genel anahtar körleştirmesi henüz yoktur: DHT kaydı bu sürümde uzun dönem anahtarla imzalanır ve kör yayın kapalıdır.

CBDesk has no account, no project identity server, and no mandatory relay. Two devices can connect directly. When they cannot, a community relay the user chose may carry the packets. A relay must not be able to read the session; end-to-end encryption is mandatory. Public-key blinding is not implemented: DHT records in this tree are signed with the long-term key, and blinded publishing fails closed.

## Dolandırıcılık uyarısı / Fraud warning

Sizi telefonla arayıp bu programı kurmanızı isteyen kişiye güvenmeyin. Bankalar, kamu kurumları ve destek şirketleri uzaktan erişim istemez. Bağlantıyı reddedin.

If someone called you and told you to install this program, deny the connection. Banks, government agencies, and support companies do not ask you for remote access.

## Onay / Consent

Oturum gizli tutulamaz. Oturum göstergesinin kapatma anahtarı yoktur. Davet bileti yer tarifidir; erişim vermez. Hızlı Destek gözetimsiz erişim açamaz. İlk temasta dosya yazma, uçbirim, yerel girdiyi engelleme ve gizlilik modu ayrıca seçilmeden verilmez.

A session cannot be hidden. The session indicator has no off switch. An invite ticket is a locator, not an authorization. Quick Support cannot enable unattended access. On a first contact, file write, terminal, blocking local input, and privacy mode stay off unless the person at the host opts in.

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
