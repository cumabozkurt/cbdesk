# Gizlilik

CBDesk, projenin bir oturumun ortasında durmaması için tasarlanmıştır.
Depodaki metin
[PRIVACY.md](https://github.com/cumabozkurt/cbdesk/blob/main/PRIVACY.md)
dosyasındadır.

Bu ağaçta hesap, telemetri ve bir analiz ucu yoktur. Cihaz adları, sonraki
bir ayar ekranı açık bir onayı kaydetmedikçe duyurulmaz. Yerel ağ duyurusu
adı varsayılan olarak taşımaz.

Ed25519 tohumu cihazda kalır. Kısa kodun sözcükleri, eşleşme denetiminin
parolasıdır. Davet biletine yazılmazlar. Bilet adresler ve bir imza taşır.
Bir izin vermez.

Kurduğunuz bir aktarıcı adresleri ve trafik hacmini görebilir. iroh
yolundaki QUIC yükleri şifrelidir. Projeyi o aktarıcı çalıştırmaz. Körleştirme
tasarımı kabul edilene kadar DHT kayıtları cihaz kimliğine bağlanabilir.
Bugünkü DHT yardımcısını bağlantısız bir dizin saymayın.

Kayıt bildirimi bir metin dosyası yazar ve bir masaüstü bildirimi
gösterebilir. Ekranın şifreli kaydı değildir.
