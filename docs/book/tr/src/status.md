# Durum ve sınırlar

`docs/MASTER_PLAN.md` içindeki plan kutuları hâlâ işaretli değildir. Onları
eşleyen döküm
[docs/dev/m0-status.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/m0-status.md)
dosyasındadır. Bu sayfa, kılavuzu okuyan biri için kısa biçimdir.
Sahibinde kalan ve bu ağaçta bulunmayan iş [sınırlar](limits.md)
sayfasındadır.

## Testlerin gerçekten çalıştırdığı

- Onaydan sonra `cbdesk host` ve `cbdesk view` aynı yerel ağda bir oturum
  açabilir. Görüntü renklidir. Sentetik kaynak gri bir barkoddur.
- Flutter kabuğu cihaz kimliğini, bağlanmayı, onayı, kapatma anahtarı
  olmayan göstergeyi, çözülen kareyi, dosya baytlarını, adres defterini ve
  ayarları Türkçe ve İngilizce gösterir. Pencere öğesi testleri sahte bir
  masa kullanır.
- Bilet çevirmesi, döngüsel bir QUIC yankısı, pano metni, bir dosya koyma
  ve sentetik bir Opus tonu testlerle kapsanır. Eksik bir mikrofon hatadır
  ve sesin duyulduğunu taklit etmez.
- Linux CI, X11 yakalama yolunu Xvfb altında çalıştırır. Özel bir Xvfb,
  ana makine işlemleri kitaplığındaki giriş kilidini ve siyah pencereyi de
  kapsar.
- CI, Ubuntu, Windows ve macOS'ta hata ayıklama kabuğunu ve arm64 bir
  Android hata ayıklama APK'sını kurar. macOS CI, denetleyici crate'ini
  iOS cihaz hedefi için tür denetiminden geçirir.

## Denenmeyenler

- Android'i bir telefonda kimse tıklamadı ve sistem yakalama penceresi
  kabul edilmedi. İmzalı bir iOS derlemesi yoktur.
- Wayland ScreenCast derlenir ve CI'da çalıştırılmadı.
- Canlı bir simetrik NAT ölçülmedi. Döngü yolu bir delik delme değildir.
  Netlab profilleri ağ ad alanları oluşturmaz.
- Mikrofonu birinin duyduğu doğrulanmadı. Bir makinede 1080p yazılım ana
  karesi 201 ms sürdü. Sonraki düz renkli ölçüm tek ana karede 198 ms,
  dört paralel şeritte 52 ms, çeyrek boyda 12 ms sürdü. `--strips` 1'den
  büyükse bu şeritler bir VP8 oturumunda gidebilir. Varsayılan göndermez.
  Bu, 30 kare/saniye değildir. Başarılı bir donanım kodlaması kaydedilmedi.
- Bu işlevler kendi başlarına sınansa da canlı ana makine X11 kilidini,
  gizlilik penceresini, kabuğu veya tüneli tutmaz.
- Linux ile Windows arasında bir oturum çalıştırılmadı. 72 saatlik bir
  dayanıklılık denemesi yok. İkinci bir makine ikilileri karşılaştırmadı.
  Dış güvenlik denetimi yok. İmzalı paket ve mağaza kaydı yok.
- `v0.1.0-alpha.1` etiketi GitHub'da imzasız bir alfadır. Bu çalışma alanı
  o arşivleri paketlemedi ve bu sayfa onları yeniden derlemez. Etiket
  1.0.0 değildir.
- GitHub organizasyon ayarları, dal koruması, marka araştırması ve imza
  anahtarları sahibinde kalır.

Plandaki M0 çıkışı, üç masaüstünde derlenen bir çataldır. Bu depo o çatal
değildir. Yeşil bir CI koşusu o çıkış değildir.
