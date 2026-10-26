# Değişiklik günlüğü

Bu dosya Keep a Changelog biçimindedir. Proje anlamsal sürümleme kullanır.
0.0.0 sürümü bir yayın değildir.

Bu metin [`CHANGELOG.md`](CHANGELOG.md) kaydının Türkçesidir. İki dosya
ayrışırsa İngilizce dosya esas alınır.

## [Yayımlanmamış]

### Düzeltildi

- macOS Flutter sürüm derlemesi yalnız arm64 hedefler. Aksi halde sürüm
  derlemesi `ARCHS=arm64 x86_64` kurar ve `x86_64-apple-darwin` kurulu
  olmadığı için `zeroize`, `cfg-if` ve `libc` E0463 ile düşer. İş akışı
  `aarch64-apple-darwin` ekler ve `ARCHS=arm64` geçirir. Masaüstü `.app`
  evrensel ikili değildir. Var olan bir etiketi adlandıran elle çalıştırma
  o sürümü yerinde günceller.
- macOS masaüstü sürüm adımı artık `--no-codesign` geçirmez. Flutter 3.47
  bu bayrağı reddeder. İş akışı `flutter build macos --release` çalıştırır
  ve imzayı `CODE_SIGNING_ALLOWED=NO` ile kapatır. Uygulama noter onaylı
  değildir. Yayımlama işi, iş akışı iptal edilmedikçe çalışır, derlenen
  arşivleri yükler ve eksik platformu sürüm notlarında adlandırır.
- İndeksli yakalama testi, ekran gerekirken geçici bir X bağlantısını
  yeniden dener. Başka bir yakalama istemcisi bağlıyken Xvfb el sıkışmayı
  sıfırlayabilir. Liste dışı indeks yine reddedilir.
- macOS olay tuzağı CoreGraphics ve CoreFoundation'ı ayrı `extern`
  bloklarında bağlar. Üst üste binmiş bir `#[link]` çifti
  `clippy::duplicated_attributes` ile düşer.
- Oturum göstergesi pencere testi, canlı bölge etiketinin tamamını kabul
  eder. `6a11de6` üzerindeki Actions, etiketin tam olarak `Oturum` olmasını
  istiyordu. Linux olmayan derlemeler, olmayan kapağın `mapped_size`
  alanını artık çağırmaz ve Unix PTY bayrakları macOS'ta `mut` değildir.
  ffmpeg taslak yardımcıları yalnız Unix'te derlenir; `0abb194` üzerinde
  Windows clippy bunu reddetmişti.

### Eklendi

- `.github/workflows/release.yml` Linux, Windows ve macOS için
  `cargo build --release --locked` ile imzasız CLI arşivleri, Flutter
  masaüstü paketleri, arm64 hata ayıklama Android APK'sı, var olan imzasız
  deb ve bir SBOM derler. Bir `v*` etiketi veya etiketi adlandıran elle
  çalıştırma bir GitHub ön sürümü açar ve bu dosyalarla `SHA256SUMS`
  dosyasını yükler. Yalnız yayımlama işinin `contents: write` izni vardır.
  Dosyalar imzasız bir alfadır. SmartScreen ve Gatekeeper uyarı verebilir
  veya engelleyebilir. Bu kayıt bir etiket itmez, iş akışını çalıştırmaz
  ve bir dosya yayımlamaz. Bir telefona kurulum ve bu ikililerin performans
  koşusu kaydedilmedi. 0.0.0 sürümü hâlâ bir yayın değildir.
- `cbdesk host --monitor N`, `list_monitors` içindeki o indeksteki ekranı
  yakalar. İndeks o çağrıdaki konumdur, sabit bir donanım kimliği değildir.
  Yazılmazsa birincil ekran kalır. Liste dışı indeks reddedilir.
  `--monitor` ile `--synthetic` reddedilir. Burada ikinci bir fiziksel
  monitör takılı değildi. Masaüstü oturumu hâlâ `capture_picture` çağırır.
- Kullanıcı kılavuzunun Türkçe ve İngilizce bir sınırlar sayfası vardır.
  Sahibinde kalan işi ve bu ağaçta bulunmayan işi listeler. Plan kutuları
  işaretsiz kalır. Bu kayıtta yayımlanmış bir dosya hâlâ yoktur.
- `cbdesk host --strips N` (varsayılan 1, en çok 16) bir VP8 görüntüsünü
  yatay ana kare bantları olarak gönderebilir. `canvas_width`,
  `canvas_height`, `strip_index` ve `strip_count` başlık alanları sıfırken
  yazılmaz. İzleyici bir bandı boyamadan önce boş bir tuval ayırır.
  Varsayılan hasar kodlayıcısında kalır. Diğer kodekler bayrağı yok sayar.
  Masaüstü sunucusu bu bayrağı almaz. Bu bantlar ara kare yolu değildir ve
  30 kare/saniye değildir. `docs/dev/media-measurement.md` içindeki 52 ms,
  hâlâ tek dolu 1920×1080 karedir.
- Dil anahtarı `Türkçe` ve `English` adlarını Türkçe ve İngilizce
  kataloglardan okur. İki katalog da bu yerel adları kullanır.
- Kabuk, metin alanlarına var olan katalog dizelerini etiket olarak verir,
  oturum kenarını anlambilim ağacının dışında bırakır ve uzak resmi var
  olan uzak görünüm cümlesiyle adlandırır. Gösterge canlı bölge olarak
  kalır ve hâlâ kapatılamaz. Yeni katalog anahtarı yoktur.
- HTML içeren pano metni, gönderilmeden veya yazılmadan önce düz metne
  indirilir. `cbdesk-secret` olan bir satır paylaşılmaz. Bu işaret parola
  tespiti değildir. Dosya yapıştırma yoktur.
- `list_monitors` ve `capture_picture_on`, yakalayıcının bildirdiği bir
  ekranı seçer. `capture_picture` ve `cbdesk host --screen` birincil
  ekranda kalır. Burada ikinci bir fiziksel monitör takılı değildi.
- Oturum çözücüleri `garbage_does_not_panic` içinde rastgele baytlarla
  çalıştırılır. Bu cargo-fuzz veya OSS-Fuzz değildir. Yok sayılan bir test,
  64×64 bir görüntünün dört VP8 şeridini süreler. Bkz.
  `docs/dev/fuzz-and-bench.md`. Criterion bir bağımlılık değildir.
- `cbdesk-relayd` döngüsel bir iroh aktarıcısına bağlanır, kendinden imzalı
  bir `https://` adresi yazar ve başka adresi reddeder. Bir test UDP
  kapalıyken bir sondayı yankılar. Bu bir simetrik NAT ölçümü değildir.
  Docker taslağı derlenmedi ve bir imaj yayımlanmadı. n0 aktarıcı
  varsayılanları kapalı kalır.
- Yazılım VP8, hasar karolarını `wide` 1.7.1 `u8x16` ile karşılaştırabilir
  (SSE2 veya NEON, bu crate'te `unsafe` yok) ve yatay şeritleri birkaç
  iş parçacığında kodlayabilir. `--profile balanced` ve `--profile preview`
  bir `--screen` yakalamasını kutu süzgeciyle 2 veya 4 kat küçültür.
  Varsayılan `full` profil ölçeklemez ve `--synthetic` diğer ikisini
  reddeder. Bu makinede tek dolu 1920×1080 görüntünün bir sürüm koşusu bir
  ana kare olarak 198 ms, dört paralel şerit ana karesi olarak 52 ms, yarı
  boyda 48 ms ve çeyrek boyda 12 ms sürdü. Şeritler canlı oturumda
  gönderilmez. Bu 30 kare/saniye değildir. Sayılar
  `docs/dev/media-measurement.md` içindedir.
- `docs/book` altında Türkçe ve İngilizce bir mdBook kullanıcı kılavuzu ve
  iki dilli bir giriş sayfası vardır. `.github/workflows/pages.yml` bunu
  `main` dalına her itmede derler ve GitHub Pages ile yayımlar. Site ikili
  dosya barındırmaz.
- Flatpak şablon kimliği `org.cbdesk.CBDesk` şeklindedir. Flathub'a
  gönderilmedi.
- Yazılım VP8 yolunda hasar izleme. Değişmeyen görüntüler kodlanmaz. Küçük
  kirli bir bölge, `damage_width` dolu ve `keyframe` temizlenmiş ayrı bir
  ana karedir. Bu makinedeki sürüm ölçümü
  `docs/dev/media-measurement.md` içindedir: 1920×1080 ana kare 201 ms,
  tekrar için bildirilen süre 1 ms'nin altında, 64×64 bölge 2 ms. Bu 30
  kare/saniye değildir. `hw-vaapi`, `hw-mediafoundation`, `hw-videotoolbox`
  ve `hw-mediacodec` ffmpeg'e kabuk açar ve varsayılan olarak kapalıdır.
  Süreç içi bir donanım bağlamı bağlı değildir ve başarılı bir donanım
  kodlaması kaydedilmedi.
- `cbdesk-hostops` onaylı bir X11 tutması ve siyah pencere, bir kayıt
  bildirim dosyası, bir Unix sözde uçbirimi veya Windows `cmd.exe`
  boruları ve döngüsel bir TCP iletimi yapar. `open_virtual_display`
  Linux'ta `Xvfb` başlatır. Canlı `cbdesk host` oturumu bunları çağırmaz.
  Windows `BlockInput` çağırır ve macOS bir oturum olay tuzağı kurar;
  ikisi de bırakılınca salınır. Reddedilen çağrı `Unavailable` olur ve bu
  çalışma alanı iki sistemde de başarılı bir tutma kaydetmedi. Gizlilik
  kapağı hâlâ X11'dir. ConPTY ve imzalı bir sanal ekran yoktur.
- `cbdesk-controller`, macOS CI'ın `aarch64-apple-ios` için tür
  denetlediği izleyici tarafı crate'tir. İmzalı bir iOS derlemesi değildir.
  `tools/android/device-check.sh` yalnız bir cihaz varken `adb` ile
  konuşur. Bir telefon denenmedi.
- İmzasız sürüm şablonları: `cargo metadata` ile SPDX, `CBDESK_SIGN_KEY`
  yokken imzasız kalan bir sağlama betiği, bit düzeyinde yeniden
  üretilebilirlik iddia etmeyen bir derleme girdi kaydı, deb / AppImage /
  DMG / WiX / Flatpak taslakları ve bir `workflow_dispatch` sürüm iş
  akışı. Git'te anahtar yoktur ve bir kurulum imzalanmadı.
  RUSTSEC-2026-0387, `opaque-ke` 4.0.1 `crypto-bigint` 0.5.5'ten
  ayrılamadığı için `deny.toml` ve `.cargo/audit.toml` içinde yok sayılır.
  Aritmetik yamanmamıştır.
- Flutter oturum göstergesi, var olan Session / Oturum dizesiyle etiketli
  bir anlambilim canlı bölgesidir. Gezinme simgeleri var olan etiketleri
  kullanır. Yeni katalog anahtarı eklenmedi. Bir ekran okuyucu
  kullanılmadı.
- `cbdesk net-check` ve `cbdesk doctor` yerel UDP bağlamalarını, CGNAT'ı
  (`100.64.0.0/10`) ve bir adres sınıfını bildirir. Simetrik NAT, UPnP,
  DHT ve delik açma ölçülmedi diye yazılır. `--loopback` bu makinede bir
  iroh yolu kaydeder ve o yolun NAT deliği olmadığını söyler. Bir eşleme
  sınıflandırıcısı uç noktaya bağlı durumu birim testinde kapsar. Bkz.
  `docs/dev/nat-measurement.md`.
- Kör anahtar altında DHT yayını hâlâ hata verir ve kayıt açılmaz. Uzun
  dönem kayıtların cihaz kimliğine bağlanabildiği belgelenmiştir. Yalıtık
  kip bir aktarıcıyı ve bir DHT yayını reddeder. ADR-007 öneri durumunda
  kalır.
- Kalıcı parolalar `opaque-ke` 4.0.1 ve Argon2id kullanır (19 MiB, 2 geçiş,
  1 şerit). Parola, sunucu işlevlerinin bir argümanı değildir. Uzunluk en
  az 10 karakterdir ve zxcvbn puanı en az 3'tür. CLI parola kaydetmez.
  TOTP HMAC-SHA1'dir (RFC 6238) ve onaya bağlı değildir. Ne OPAQUE
  bileşiminin ne de SPAKE2 transkriptinin dış incelemesi vardır.
- Bir yazılım Opus döngüsü, aygıt açmadan bir tonu kodlar, tamponlar ve
  çözer. 1920×1080 yazılım VP8 süresi `docs/dev/media-measurement.md`
  içindedir ve 30 kare/saniye değildir. 4:4:4 reddedilir. Donanım
  özellikleri sonraki bir maddede anlatılır.
- İzin ön ayarları, bellekte bir BLAKE3 erişim günlüğü, oturum süreleri,
  taraf değiştirme, beyaz tahta çizgileri ve gönderilmeyen bir Wake-on-LAN
  yükü için kitaplık kapıları. Gizlilik, girdiyi engelleme, kayıt,
  uçbirim, tünel ve sanal ekranın artık `cbdesk-hostops` ve
  `cbdesk-capture` içinde işletim sistemi işlevleri vardır. Canlı oturum
  onları çağırmaz. İmzalı kurulumlar, dış denetim ve TUF bitmiş değildir.
  Bkz. `docs/dev/deployment.md` ve `docs/dev/wayland-screencast.md`.
- Yayımlanmış `spake2` 0.4.0 crate'i ile kısa kod eşlemesi. Parola iki
  küçük harfli sözcüktür. Yuva ve iki cihaz anahtarı yalnız transkripttedir.
  Sözcükler bilete yazılmaz. Masaüstü sunucusu onaydan önce bu değiş tokuşu
  ister. Yanlış kod istemi açmaz. Sunucu yine onaylar. CLI sunucusu bu
  değiş tokuşu çalıştırmaz. Transkriptin dış incelemesi yoktur.
- Flutter kabuğu canlı oturumu gönderir. Masaüstü sunucusu, kuyrukta resim
  yokken `capture_picture` kodlar. Uzak görünüm, Input izninden sonra bir
  tıklama ve Enter, Clipboard izninden sonra pano metni ve Audio izninden
  sonra bir mikrofon anahtarı iletir. Dosya paneli, kafes adı kabul
  ettikten sonra baytları QUIC üzerinden gönderir. Eksik bir mikrofon
  oturumu bitirmez. Bu ağaç birinin sesi duyduğunu kaydetmez. Android
  yakalama sistem penceresini kullanır ve o pencere kabul edilmeden sanal
  ekran oluşturmaz. Uzun kenar en çok 960 pikseldir.
- CI, Windows ve macOS hata ayıklama kabuklarını ve bir arm64 Android hata
  ayıklama APK'sını derler ve her birinin `libcbdesk_ffi` bağladığını
  denetler. Flutter eklenti dizini bir sembolik bağdır. Crate yolu,
  derleme `crates/cbdesk-ffi` aramadan önce fiziksel dizine çözülür.
  macOS pod'u AudioToolbox, CoreAudio, CoreMedia, SystemConfiguration ve
  Carbon'u o arşivle bağlar. Simge denetimi, uygulamanın içindeki bağlı
  `cbdesk_ffi` çatısını okur. Xcode 15.4'ün Apple `nm` aracı, statik
  arşivin içindeki Rust 1.91 nesne dosyalarını okuyamaz. macOS paketi,
  Actions bitene kadar kanıtlanmış sayılmaz.
- VP8 ara kareler ve renk. İlk kare ana karedir. Aynı boyuttaki sonraki
  kare, oxideav-vp8 elmas aramasından (tam, yarım ve çeyrek piksel) hareket
  dengelemeli bir ara karedir. 48×16 bir test, nicelik 96'da beyaz bir
  bloğu 16 piksel kaydırır ve kayan sütunları denetler. Nicelik 0'daki
  barkod testi o barkodu bir ara karede de okur. Canlı yol yakalamanın çift
  boyutunu korur. Tempo 33 ms hedefler ve kodlama süresi, kayıp veya
  gecikme söylediğinde niceliği yükseltir. Bu günlüğün sonraki bir notu,
  30 kare/saniye olmayan bir 1080p süresi kaydeder.
- Betikli bir işaretçi ve Enter, QUIC üzerinden gider ve sunucu bunları
  Input izninden sonra yazar. Pencere aynı eşlemeyi kullanır. Sunucu boş
  bir onay gönderir ki izleyici ardından kapanırsa son olay yine okunsun.
- Sürdürülebilir dosya aktarımı. Parçalar, yanda bir bitmap ile
  sıkıştırmasız BLAKE3 parçalarıdır. `file_write` yüksek riskli kalır.
  CLI `--put` yolu en çok 32 MiB okur. zstd bağlı değildir.
- Audio izninden sonra rusty-opus ile Opus sesi. `--tone` sentetik bir
  paket gönderir ve aygıt açmaz. `cbdesk host --mic` ve `--speaker` ile
  karşılık gelen izleyici bayrakları, varsayılan aygıtları cpal 0.15.3 ile
  açar. Eksik aygıt hatadır. Bu ağaç birinin akışı duyduğunu kaydetmez.
  Seğirme tamponu 40 ms bekler ve 120 ms'den eski sesi düşürür.
- Operatörün yapılandırdığı bir aktarıcı. Yasak sonekler ve n0
  varsayılanları hâlâ reddedilir. Varsayılan uçnoktanın
  aktarıcı listesi boştur. Bir aktarıcı kurulunca HTTPS ağ raporu
  sondaları yalnız o aktarıcıya gider ki bir ev aktarıcısı seçilebilsin.
  Döngüsel bir aktarıcı testi UDP kapalıyken bir sonda taşır. Canlı bir
  simetrik NAT hâlâ ölçülmemiştir. Bu proje bir aktarıcı çalıştırmaz.
- Çalışma alanı yerleşimi ve ilk crate'ler: protokol kodeği, kimlik, keşif
  kayıtları, bellekte taşıma, sunucu oturum ilkesi ve test çiftleri.
- n0 aktarıcıları ve n0 keşfi kurulmamış, iroh 1.4.1 üzerinde doğrudan QUIC
  uçnoktası. Rust o sürümün `rust-version` değeri olan 1.91.0'a sabitlenir.
  T-NET-001, çözümleyicisinden bir ad sorulmaması gereken bir döngüsel
  yankıdır.
- Doğrulanmış bir davet bileti kendi doğrudan adreslerini arar. Biletin
  üzerindeki aktarıcı adresleri aranmaz. Sunucu o bağlantıyı yalnız
  onaydan sonra verir. Yanıtsız bir onay 30 saniyede
  `TIMEOUT_USER_RESPONSE` ile kapanır. Bir yankı sondası
  `PROTOCOL_VIOLATION` olur ve oturum vermez.
- Sınanan mDNS TXT kayıtları ve `CBDS` yayın gövdesi üzerinde LAN duyurusu
  ve göz atma. Herkese açık ağlar ve kapatılmış bir kullanıcı hiçbir şey
  göndermez.
- `app/desktop` içinde bir Flutter masaüstü kabuğu (Flutter 3.47.7,
  flutter_rust_bridge 2.12.0). Ana sayfa cihaz kimliğini ve kısa kodu
  gösterir. Diğer ekranlar bağlan, onay, uzak görünüm, dosya paneli, adres
  defteri ve ayarlardır; Türkçe ve İngilizce. Onay, dolandırıcılık
  uyarısını gösterir, ilk temasta bekler ve yüksek riskli bir izin için
  açık bir seçim ister. Reddet odaklanmış başlar. Oturum göstergesi
  sayfaların üstünde boyanmış bir kenar ve bir kopma panelidir ve kapatma
  anahtarı yoktur. Uzak görünüm çözülen karedir. Masaüstü sunucusu, kuyrukta
  resim yokken işletim sistemi yakalamasını gönderir. Dosya paneli baytları
  QUIC üzerinden gönderir. Mikrofon anahtarı varsayılan aygıtı açar. İki
  kısa kod sözcüğü eşleşmeyi doğrular ve bir bilgisayar bulmaz. Bilet yine
  yalnız yer tarifidir. Pencere testleri sahte bir masa kullanır ve yerel
  kitaplığı yüklemez.
- GitHub Actions; rustfmt, cargo-deny, RustSec denetimi, clippy ve testleri
  Linux, Windows ve macOS'ta, Flutter analyze ve pencere testlerini de bu
  üçünde çalıştırır. Ubuntu, Linux hata ayıklama paketini ve bir arm64
  Android hata ayıklama APK'sını derler. Windows ve macOS kendi hata
  ayıklama paketlerini derler ve `libcbdesk_ffi` bağını denetler.
- ADR 001–015, tehdit modeli taslağı ve katkı belgeleri.
- Şimdiye kadar var olan hata ve onay dizeleri için Türkçe ve İngilizce
  ileti katalogları.
- `authorize(View)` sonrasında açılan, video karesi başına tek yönlü bir
  QUIC akışı. İzin verilmeden açılan bir yan akış `PROTOCOL_VIOLATION`
  olur. Bir girdi olayı kendi tek yönlü akışını kullanır ve yalnız
  `authorize(Input)` sonrasında kabul edilir.
- Tek gri tonlu durağan görüntü için yazılım AV1 (rav1e, birleştirici ve
  git araçları kapalı; rav1d-safe AGPL-3.0-only altında).
- oxideav-vp8 0.2.7 ile yazılım VP8 (MIT, varsayılan özellikler kapalı,
  sistem libvpx'i yok). İki eş de VP8 sunarsa görüşme onu yeğler. AV1
  durağan görüntü yedeğidir.
- xcap 0.9.5 ile işletim sistemi ekran yakalama ve Input izninden sonra
  enigo ile klavye ve işaretçi enjeksiyonu. Testler girdi olaylarını
  kaydeder ve işaretçiyi oynatmaz.
- `cbdesk host` izleyici kapanana kadar kare akıtır (`--frames 0`).
  `cbdesk view --window` bunları winit ve softbuffer ile gösterir ve
  işaretçi ile klavyeyi yalnız Input izninden sonra iletir. `--ppm` hâlâ
  ilk kareyi yazar. `--screen` ekranı yakalar. Bilet hâlâ yalnız yer
  tarifidir.
- Clipboard izninden sonra iki yönlü bir akışta düz metin pano eşlemesi,
  60 KiB ile sınırlı. Dosyalar ve görüntüler bu ileti değildir.
- Cihaz tohumu işletim sistemi anahtarlığından yüklenir (Secret Service,
  Keychain veya Credential Manager). Depo yanıt vermezse o süreç için
  bellekte bir kimlik kalır. Tohum bir dosyaya yazılmaz.
- LAN duyurusu ve göz atma, port paylaşılabiliyorsa `224.0.0.251:5353`
  grubuna katılabilir. Dolu bir 5353, gruba göndermeye devam eder ve geçici
  bir portta dinler. `--lan`, operatörün özel bir profile katılmasıdır.
- ADR-015 temiz oda çekirdeğini korur. M0 çatal çıkışı bilerek
  karşılanmamıştır.

Bu ağaç, ADR-015'in andığı diğer projeden türetilmemiştir. Bkz. ADR-002 ve
ADR-015.
