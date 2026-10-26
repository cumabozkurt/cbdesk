# Kurulum ve indirme

Bu ağaçtaki 0.0.0 sürümü bir yayın değildir. Bu site bir kurulum aracı,
paket veya mağaza kaydı barındırmaz. Bu kayıt bir dosya yayımlamaz.

## İndirme

GitHub etiketi `v0.1.0-alpha.1`, `v0.1.0-alpha.1 unsigned alpha` başlığını
taşır ve aşağıdaki dosyaları içerir. Bu site onları barındırmaz. Bu sayfa
onları yeniden derlemez. İmzasız bir alfadırlar. 1.0.0 sürümü ve planın
alfa çıkışı değildir. Windows SmartScreen ve macOS Gatekeeper uyarı
verebilir veya engelleyebilir. Bu derlemede noter onayı, Apple geliştirici
hesabı ve Authenticode sertifikası yoktur. Bu ikililer için gerçek bir
cihaz denemesi ve bir performans ölçümü kaydedilmedi. 30 kare/saniye
ölçülmedi. Crate sürümü hâlâ 0.0.0'dır.

Sürüm sayfası
<https://github.com/cumabozkurt/cbdesk/releases/tag/v0.1.0-alpha.1>
adresindedir.

| Dosya | Ne olduğu |
|---|---|
| [Windows masaüstü zip](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-desktop-0.1.0-alpha.1-windows-x64.zip) | Flutter paketi, x64 |
| [Windows CLI zip](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-x86_64-pc-windows-msvc.zip) | `cbdesk.exe` |
| [Linux masaüstü tar.gz](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-desktop-0.1.0-alpha.1-linux-x64.tar.gz) | `ubuntu-24.04` üzerinde Flutter paketi |
| [Linux CLI tar.gz](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-x86_64-unknown-linux-gnu.tar.gz) | `ubuntu-24.04` sistem kitaplıklarına bağlanır |
| [Linux CLI deb](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk_0.1.0-alpha.1_amd64.deb) | İmzasız. Flutter uygulaması değildir |
| [macOS arm64 masaüstü zip](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-desktop-0.1.0-alpha.1-macos-arm64.zip) | Yalnız Apple Silicon. Noter onaylı değildir |
| [macOS arm64 CLI](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-aarch64-apple-darwin.tar.gz) | Yalnız Apple Silicon. Noter onaylı değildir |
| [Android arm64 hata ayıklama APK](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/cbdesk-0.1.0-alpha.1-android-arm64-debug.apk) | Bir telefona kurulmadı |
| [SHA256SUMS](https://github.com/cumabozkurt/cbdesk/releases/download/v0.1.0-alpha.1/SHA256SUMS) | Özetler, imza değil |

O etiketteki `sbom.spdx.json.sign-note` dosyası `unsigned: CBDESK_SIGN_KEY is unset` yazar.

İş akışının yükleyeceği dosyalar:

- `cargo build --release --locked` ile üretilen CLI arşivleri. Linux,
  `ubuntu-24.04` üzerinde `x86_64-unknown-linux-gnu` için bir `.tar.gz`
  dosyasıdır ve o imajın sistem kitaplıklarına bağlanır. Windows,
  `x86_64-pc-windows-msvc` için `cbdesk.exe` içeren bir `.zip` dosyasıdır.
  macOS, `macos-14` üzerinde `aarch64-apple-darwin` için bir `.tar.gz`
  dosyasıdır. Yalnızca Apple Silicon'dur. Evrensel ikili değildir ve noter
  onaylı değildir.
- Flutter masaüstü paketleri: Linux release bundle `.tar.gz`, Windows
  release `.zip` ve macOS release `.app` `.zip`. macOS uygulaması yalnız
  arm64'tür. Evrensel ikili değildir ve noter onaylı değildir. Flutter
  3.47'de `--no-codesign` bayrağı yoktur. İş akışı kod imzasını kapatır
  (`CODE_SIGNING_ALLOWED=NO`). Bir platform işi başarısız olursa sürüm,
  derlenen arşivleri yine alır ve eksik olanı adlandırır.
- arm64 Android hata ayıklama APK'sı. Sürüm imzalı bir paket değildir. Bir
  telefona kurulmadı ve sistem yakalama penceresi kabul edilmedi.
- İmzalı olmayan deb ve SPDX SBOM. `CBDESK_SIGN_KEY` ayarlı değildir, bu
  yüzden sağlama notu imzasız der.
- `SHA256SUMS`. Bu dosya özetleri listeler. İmza değildir.

İmzalı bir MSI, DMG, rpm, AppImage veya Flatpak yayımlanmadı. Depoda bu
şablonlar durur. Flatpak bildirim kimliği `org.cbdesk.CBDesk` şeklindedir.
Flathub'a gönderilmedi. Windows kurulum kaynağı derlenmedi. DMG ve
AppImage betikleri çalıştırılmadı. TestFlight derlemesi ve bu ağaçta bir
Apple geliştirici hesabı yoktur.

Biri sizi arayıp uzaktan erişim programı kurmanızı söylediyse reddedin.
Bankalar, kamu kurumları ve destek şirketleri bunu istemez.

## Kaynaktan derlemek

Geliştirici derlemesi Rust 1.91.0 ister. Masaüstü kabuğu Flutter 3.47.7
ister. Adımlar depodaki
[docs/dev/building.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/building.md)
dosyasındadır. Bu derleme, kod üzerinde çalışanlar içindir. "Uzaktan
erişimi indir" denmiş birine verilmiş desteklenen bir kurulum değildir.

Sürüm betiklerindeki sağlama, `CBDESK_SIGN_KEY` deponun dışında ayarlanmazsa
imzasız kalır. Git'te özel anahtar yoktur.
