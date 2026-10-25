# Kullanıcı kılavuzu

Bu sayfa, temel kilometre taşı için Türkçe kullanıcı notudur. Aynı not
depoda `docs/user/tr/README.md` dosyasındadır.

Bir sürüm olduğunda, biri ekranınızı görmeden önce size sorulur. Oturum
göstergesi görünür kalır ve kapatılamaz. Davet bileti karşı tarafın
bilgisayarınızı bulmasına yardım eder. İçeri almaz. Kısa koddaki iki sözcük,
bu cihazla eşleştiğinizi doğrular. Bir bilgisayar bulmaz ve yine onayınız
gerekir.

## Her parçanın işi

Bilet bir yer tarifidir. Süresi dolar ve ekranı görme ya da bilgisayarı
yönetme izni değildir. Kısa kod, iki tarafın aynı cihazdan söz ettiğini
denetler. Yanlış kod onay penceresini açmaz. Ana makinedeki kişinin yine
kabul etmesi gerekir.

Ekranı görmek; yerel klavyeyi kilitlemek, ekranı örtmek, terminal açmak,
bağlantı noktası iletmek ve kayıt almak için ayrı bir izindir. Her biri
ayrı onay ister. Gösterge, bunlardan birini gizlemek için kapatılamaz.

## Linux parçaları bugün ne yapar

Linux'ta ana makine işlemleri kitaplığı X11 klavye ve işaretçi kilidi, siyah
bir pencere, bir kabuk, tek bir döngü TCP iletimi, bir kayıt bildirimi ve
bir Xvfb ekranı açabilir. `cbdesk host` bunları kendiliğinden açmaz.
Windows `BlockInput` çağırır, macOS bir oturum olay tuzağı açar. Reddedilen
çağrı girişi kilitlemez. Örtü hâlâ Linux penceresidir. Kayıt bildirimi bir
metin dosyasıdır; oturumun şifreli kaydı değildir.

## Görüntü ve ses

Canlı görüntü renklidir. 1080p yazılım görüntüsü 30 kare/saniye değildir.
Bir makinede yapılan release ölçümü, 1920×1080 bir ana kareyi 201 ms'de
kodladı. Değişmeyen resim kodlanmadı. 64×64 değişen bir bölge 2 ms sürdü.
Tam kare değişikliği hâlâ saniyede yaklaşık beş karedir. `--profile
balanced` ve `--profile preview`, ekran yakalamasını aynı kodlayıcıdan
önce küçültür. Sonraki düz renkli kare ölçümü, masaüstü değil, yarı boyda
48 ms ve çeyrek boyda 12 ms sürdü. `cbdesk host --strips N` (varsayılan 1),
kodek VP8 ise bu şeritleri gönderebilir. Her şerit kendi ana karesidir,
kareler arası yol değildir ve 52 ms sayısı tek bir düz renkli kare içindir;
30 kare/saniye değildir. `--monitor N` bu çağrıdaki listeden bir ekran
seçer. Yazılmazsa birincil ekran kalır. İndeks sabit bir donanım
kimliği değildir. İkinci bir fiziksel monitör burada takılı değildi.
Mikrofon yolunu birinin duyduğu doğrulanmamıştır.
