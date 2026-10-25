# Güvenlik

Bu sayfa, kodun izlediği modelin özetidir. Dış bir denetim değildir. Uzun
taslak
[docs/threat-model.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/threat-model.md)
dosyasındadır.

## Ana makinedeki kişi karar verir

Bir oturum, birinin bileti, kısa kodu veya yerel ağ duyurusu olduğu için
başlamaz. Bunlar karşı tarafın bu bilgisayarı bulmasına yardım eder. Yine
ana makine onaylar ve onay, izinleri listeler. İlk temas, Kabul
kullanılabilene kadar bekler. Oturum göstergesi kapatılamaz. Gizli bir
oturum ayarı yoktur.

Dosya yazma, terminal, yerel girişi kilitleme ve gizlilik örtüsü gibi yüksek
riskli izinler, ekranı görmekten ayrıdır. Hızlı destek, gözetimsiz erişimi
açamaz.

## Ağ yolu

iroh yolunu kullanan oturumlar QUIC'tir. Bu yoldaki yükler şifrelidir.
Sizin kurduğunuz bir aktarıcı adresleri ve ne kadar veri aktığını görebilir.
Parolaya veya oturum içeriğine güvenilmez. Bu proje bir aktarıcı çalıştırmaz
ve bilinen genel aktarıcı sonekleri varsayılan olarak reddedilir. Tek
makinedeki bir döngü bağlantısı, NAT deliğinin çalıştığının kanıtı değildir.
Canlı bir simetrik NAT ölçülmemiştir.

## Parolalar ve kimlik

Cihaz kimliği bir Ed25519 açık anahtarıdır. Tohum cihazda kalır. Kalıcı
parola kitaplığı `opaque-ke` 4.0.1 kullanır. Komut satırı ana makinesi bir
parola kaydetmez. TOTP bir kitaplık olarak vardır ve onay istemine bağlı
değildir. Ne bu parola bileşiminin ne de kısa kod değiş tokuşunun dış
incelemesi vardır.

Yayımlanan DHT kayıtları cihaz kimliğine bağlanabilir. Kör bir anahtarla
yayımlamak kapalıdır ve hata döner. Bu tasarım henüz kabul edilmemiştir.

## Sorun bildirmek

GitHub'ın özel güvenlik bildirimini kullanın:

<https://github.com/cumabozkurt/cbdesk/security>

Bir cihaz tohumunu veya başkasının bilgisayarına yönelik bir saldırıyı herkese
açık bir konuya yazmayın. Yayımlanmış bir sürüm yoktur. Kapsam varsayılan
daldır. Yanıt hedefleri
[SECURITY.md](https://github.com/cumabozkurt/cbdesk/blob/main/SECURITY.md)
içindedir. Bunlar gönüllü hedeflerdir, bir hizmet sözleşmesi değildir.
