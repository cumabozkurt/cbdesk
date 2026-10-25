# Kurulum ve indirme

Bu ağaçtaki 0.0.0 sürümü bir yayın değildir. Bu site bir kurulum aracı,
paket veya mağaza kaydı barındırmaz. Bu kayıt bir dosya yayımlamaz.

## İndirme

Daha sonraki bir `v*` etiketi, GitHub'ın imzasız ön sürüm dosyaları
eklemesini sağlayabilir. Bir bakımcı `.github/workflows/release.yml` iş
akışını elle başlatıp etiketi de adlandırabilir. Bu değişiklikte o etiket
itilmemiştir. İş akışı çalışana kadar sürüm sayfası boş olabilir:

<https://github.com/cumabozkurt/cbdesk/releases>

Bu dosyalar, iş akışı çalıştıktan sonra, imzasız bir alfadır. 1.0.0
sürümü veya bir sürüm adayı değildir. Windows SmartScreen ve macOS
Gatekeeper uyarı verebilir veya engelleyebilir. Bu derlemede noter onayı,
Apple geliştirici hesabı ve Authenticode sertifikası yoktur. Bu ikililer
için gerçek bir cihaz denemesi ve bir performans ölçümü kaydedilmedi.
30 kare/saniye ölçülmedi. Crate sürümü hâlâ 0.0.0 yazıyor olabilir.

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
  (`CODE_SIGNING_ALLOWED=NO`). Bir platform işi başarısız olursa ön sürüm,
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
