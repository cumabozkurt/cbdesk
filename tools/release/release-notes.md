@@TAG@@ is an unsigned alpha. It is not version 1.0.0 and not a release candidate.

These files are not signed. Windows SmartScreen and macOS Gatekeeper can warn or block them. There is no notarization, no Apple Developer account, and no Authenticode certificate in this build.

No real-device test was recorded for this workflow. The Android file is a debug APK and was not installed on a phone. The system capture dialog was not accepted. macOS is an Apple Silicon build from `macos-14`, not a universal binary. Linux is built on `ubuntu-24.04` and the CLI links system libraries from that image. A 30 frame-per-second stream was not measured. Numbers in `docs/dev/media-measurement.md` are one machine and one synthetic picture, not this download.

The deb, the CLI archives, the Flutter desktop bundles, the debug APK, and the SBOM are produced by `.github/workflows/release.yml`. The SBOM is a package list. `SHA256SUMS` lists digests. It is not a signature. `CBDESK_SIGN_KEY` is unset, so the checksum note says unsigned. No private key is in git.

@@TAG@@ imzasız bir alfadır. 1.0.0 sürümü veya bir sürüm adayı değildir.

Bu dosyalar imzalı değildir. Windows SmartScreen ve macOS Gatekeeper uyarı verebilir veya engelleyebilir. Bu derlemede noter onayı, Apple geliştirici hesabı ve Authenticode sertifikası yoktur.

Bu iş akışı için gerçek bir cihaz denemesi kaydedilmedi. Android dosyası bir hata ayıklama APK'sıdır ve bir telefona kurulmadı. Sistem yakalama penceresi kabul edilmedi. macOS, `macos-14` üzerinde Apple Silicon derlemesidir; evrensel ikili değildir. Linux `ubuntu-24.04` üzerinde derlenir ve CLI o imajın sistem kitaplıklarına bağlanır. 30 kare/saniye ölçülmedi. `docs/dev/media-measurement.md` içindeki sayılar bir makine ve bir sentetik resimdir; bu indirme değildir.

deb, CLI arşivleri, Flutter masaüstü paketleri, hata ayıklama APK'sı ve SBOM `.github/workflows/release.yml` tarafından üretilir. SBOM bir paket listesidir. `SHA256SUMS` özetleri listeler. İmza değildir. `CBDESK_SIGN_KEY` ayarlı değildir, bu yüzden sağlama notu imzasız der. Git'te özel anahtar yoktur.
