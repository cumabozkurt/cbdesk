# Katkı

CBDesk'e baktığınız için teşekkürler. Tam not
[CONTRIBUTING.md](https://github.com/cumabozkurt/cbdesk/blob/main/CONTRIBUTING.md)
dosyasındadır. Plan
[docs/MASTER_PLAN.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/MASTER_PLAN.md),
kilometre taşı dökümü
[docs/dev/m0-status.md](https://github.com/cumabozkurt/cbdesk/blob/main/docs/dev/m0-status.md)
içindedir.

Lisans AGPL-3.0-or-later'dır. CLA yoktur. Bir commit `Signed-off-by`
satırı ister. Bu, Geliştirici Menşe Belgesi'dir:

<https://developercertificate.org/>

```sh
git commit -s
```

Rust 1.91.0 sabitlenmiştir. Herkese açık öğelerin belgesi olmalıdır.
Çalışma alanında `unsafe` yasaktır. Yeni bir kriptografik kurgu yazmayın.
Gizli bir oturum, göstergeyi kapatan bir yol veya bileti, kısa kodu ya da
DHT kaydını yetki sayan bir yol eklemeyin.

Bu kitaptaki kullanıcı kılavuzu mdBook 0.5.4 ile kurulur:

```sh
tools/docs/build-book.sh
python3 tools/docs/check-book-links.py site
```
