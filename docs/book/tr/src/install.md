# Kurulum ve indirme

İndirilecek bir dosya yoktur. 0.0.0 sürümü bir yayın değildir. Bu site bir
kurulum aracı, paket deposu veya mağaza kaydı barındırmaz.

İmzalı bir MSI, DMG, deb, rpm, AppImage veya Flatpak yayımlanmadı. Depoda
yalnız imzasız şablonlar vardır. Flatpak bildirim kimliği
`org.cbdesk.CBDesk` şeklindedir. Flathub'a gönderilmedi. Windows kurulum
kaynağı derlenmedi. DMG ve AppImage betikleri çalıştırılmadı. CI'da bir
hata ayıklama Android APK'sı üretilebilir; bir telefona kurulmadı. iOS
denetleyici crate'i macOS'ta tür denetiminden geçebilir. TestFlight derlemesi
ve bu ağaçta bir Apple geliştirici hesabı yoktur.

## Kaynaktan derlemek

Geliştirici derlemesi Rust 1.91.0 ister. Masaüstü kabuğu Flutter 3.47.7
ister. Adımlar depodaki
[docs/dev/building.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/building.md)
dosyasındadır. Bu derleme, kod üzerinde çalışanlar içindir. "Uzaktan
erişimi indir" denmiş birine verilmiş desteklenen bir kurulum değildir.

Sürüm betiklerindeki sağlama, `CBDESK_SIGN_KEY` deponun dışında ayarlanmazsa
imzasız kalır. Git'te özel anahtar yoktur.
