# Sahibinde kalan ve bulunmayan iş

Bu sayfa hâlâ açık olan işi adlandırır. `docs/MASTER_PLAN.md` içindeki
kutular işaretli kalır. Döküm
[docs/dev/m0-status.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/m0-status.md)
dosyasındadır. Testlerin çalıştırdığı şeyler [durum](status.md) sayfasındadır.

## Sahip

Bunları bu deponun dışındaki bir kişi, bir hesap ya da bir makine yapar.
Ağaçta hiçbirinin tamamlanmış kopyası yoktur.

- GitHub organizasyon ayarları, dal koruması, CODEOWNERS, GitHub DCO
  uygulaması ve Discussions. CI yalnızca yerel bir DCO betiği çalıştırır.
- Marka araştırması ve bir alan adı. `docs/brand/` araştırmanın
  yapılmadığını yazar.
- Bağımsız bir güvenlik denetimi. Sipariş edilmemiştir.
- Antivirüs üreticilerine bir bildirim. Gönderilmemiştir.
- İmza anahtarları, imzalı bir kurulum dosyası, bir Apple Developer hesabı
  ve TestFlight. macOS CI, denetleyici crate'ini iOS cihaz hedefi için tür
  denetiminden geçirir. İmzalı bir iOS derlemesi yoktur.
- Bir Flathub gönderimi. Flatpak şablonunun kimliği `org.cbdesk.CBDesk`
  biçimindedir. Flathub'da listelenmemiştir.
- Birinin telefonda Android sistem yakalama penceresini kabul etmesi. CI
  arm64 bir hata ayıklama APK'sı kurar. O pencere burada kabul edilmedi.
- Sürüm ikilisinin özetini çıkaran ikinci bir makine.
  `tools/release/reproducible.sh`, `SOURCE_DATE_EPOCH`, `rustc -vV` ve
  `Cargo.lock` özetini kaydeder.

## Yok

Bu ağaçta bunlar yoktur. Yeşil bir CI koşusu onları eklemez.

- Plandaki M0 çıkışı, üç masaüstünde derlenen bir çatal. Bu depo boş
  crate'lerden başladı. ADR-015 yakalamayı, kodeki ve girişi bu çekirdekte
  tutar.
- 30 kare/saniye akış ve camdan cama bir süre bütçesi. Bir makine, düz
  renkli 1920×1080 bir yazılım ana karesini 198 ms'de kodladı. Aynı düz
  renkli karenin dört paralel şerit ana karesi 52 ms sürdü. Çeyrek boy
  önizleme 12 ms sürdü. `cbdesk host --strips` 1'den büyükse bu şeritler
  bir VP8 oturumunda gidebilir. Varsayılan hasar kodlayıcısında kalır.
  Masaüstü ana makinesi bu bayrağı almaz. Her şerit kendi ana karesidir,
  kareler arası yol değildir. Ayrıntılı bir masaüstü ölçülmedi. Başarılı
  bir donanım kodlaması kaydedilmedi.
- Ölçülmüş canlı bir simetrik NAT. Döngü yolu döngü yolu olarak kalır.
  Netlab profilleri ağ ad alanları oluşturmaz.
- Körleştirilmiş DHT. Körleştirilmiş yayın kapalı başarısız olur. ADR-007
  Proposed olarak durur.
- Herkese açık bir aktarıcı ve yayımlanmış bir imaj. `cbdesk-relayd`
  yalnızca döngü adresine bağlanır, kendinden imzalı bir `https://` adresi
  basar ve UDP kapalıyken bir yoklamayı yankılar. Bu, simetrik NAT ölçümü
  değildir. `packaging/relayd/Dockerfile` kurulmamıştır. n0 varsayılanları
  kapalı kalır.
- Hızlı destek. Oturum ilkesi bunu yasaklar.
- Canlı `cbdesk host` sürecinin X11 kilidini, gizlilik penceresini, kabuğu
  veya tüneli tutması. Bu işlevler kendi başlarına vardır ve sınanır.
  Windows `BlockInput` çağırır, macOS bir oturum olay tuzağı açar.
  Reddedilen çağrı veya boş bir tuzak girişi kilitlemez. Bu çalışma
  alanında başarılı bir kilitleme kaydedilmedi. Örtü penceresi hâlâ
  X11'dir. Canlı ana makine bunları çağırmaz.
- İkinci bir fiziksel monitör ve masaüstü oturumunda bir monitör indeksi.
  `cbdesk host --monitor N` bu çağrıda `list_monitors` içindeki indeksi
  seçer. Yazılmazsa birincil ekran kalır. İndeks sabit bir donanım kimliği
  değildir. Liste dışı indeks reddedilir. Masaüstü oturumu hâlâ
  `capture_picture` çağırır. İkinci bir ekran burada takılı değildi.
- Bir dosya listesini yapıştırmak. Pano metni vardır. Dosya yapıştırma
  yoktur.
- ConPTY, imzalı bir sanal ekran ve bir Windows dolaylı ekran sürücüsü.
  Linux, ana makine işlemleri kitaplığından Xvfb başlatabilir. Canlı ana
  makine onu da çağırmaz.
- cargo-fuzz, libFuzzer, OSS-Fuzz ve Criterion. Bağlı değildirler ve
  çalıştırılmadılar.
- 72 saatlik bir dayanıklılık denemesi, Linux ile Windows arasında bir
  oturum, bir kişiyle kullanılabilirlik turu, bir beta kanalı ve sürüm
  1.0.0. Sürüm 0.0.0'dır.
- Bir Wayland ScreenCast koşusu. Elle plan
  `docs/dev/wayland-screencast.md` dosyasındadır. CI onu çalıştırmadı.
- Mikrofon yolunun duyulduğunu doğrulayan bir dinleme. Eksik bir mikrofon
  hatadır.

## Hâlâ yok sayılan

`opaque-ke` 4.0.1, `crypto-bigint` 0.5.5'e bağlı olduğu sürece
RUSTSEC-2026-0387, `deny.toml` ve `.cargo/audit.toml` içinde yok sayılır.
Bu yok sayma, ağacında 0.5.5 kalmayan kararlı bir `opaque-ke` sürümüne
geçen kayıtta kalkar. Silmeden önce
`cargo tree -i crypto-bigint -p cbdesk-crypto` ile doğrulayın.
