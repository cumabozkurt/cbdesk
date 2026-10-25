<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Owner to-do list / Sahip yapılacaklar listesi

These items need the owner's own accounts, devices, money or legal identity.
Nobody else can do them, and nothing here was attempted.
Bu işler sahibin kendi hesabı, cihazı, parası veya yasal kimliği gerektirir.
Hiçbiri denenmedi.

Order of priority / Öncelik sırası: 1 → 8.

---

## English

### 1. Create the GitHub organization (optional but planned)
1. github.com → profile menu → *Your organizations* → *New organization* →
   Free plan. Name it e.g. `cbdesk-org` (the handle `cbdesk` showed as unused
   on 2026-10-25, but see `trademark-prelim.md` before taking it).
2. Repository *Settings → General → Transfer ownership* → new owner = the org.
   GitHub keeps redirects, stars and issues.
3. Re-apply branch protection from `docs/dev/github-setup.md` (it is per
   repository and is kept on transfer, but verify), and enable 2FA
   requirement in org settings.
4. Add a second maintainer, then create a team and update `CODEOWNERS` to
   require two reviews on `crates/cbdesk-crypto` and `crates/cbdesk-net`.

### 2. Install the DCO GitHub App (optional second layer)
Open https://github.com/apps/dco → *Configure* → select the repository.
The in-repo check (`tools/check-dco.sh`) already enforces `Signed-off-by`.
Use `git commit -s` for every commit.

### 3. Apple Developer Program (iOS controller, macOS notarization)
1. https://developer.apple.com/programs/enroll/ with your Apple ID (USD 99 per
   year; individuals need ID verification; an organization needs a D-U-N-S
   number).
2. Certificates, Identifiers & Profiles: create a *Developer ID Application*
   certificate (macOS outside the store) and an *Apple Distribution*
   certificate (iOS). Register the bundle id `org.cbdesk.CBDesk`.
3. App Store Connect: create an app record, then an App Store Connect API key
   (Users and Access → Integrations).
4. Run `packaging/macos/make-dmg.sh` on a Mac, sign with
   `codesign --options runtime`, notarize with `xcrun notarytool submit`,
   then `xcrun stapler staple`.
5. Put the secrets in GitHub *Settings → Secrets and variables → Actions*
   (never in git), then wire them to `release.yml`.

### 4. Google Play Console (Android)
1. https://play.google.com/console/signup (USD 25 one-time; personal accounts
   created after Nov 2023 must run a closed test with at least 12 testers for
   14 days before production).
2. Create the app, package name `org.cbdesk.cbdesk` (must match the Gradle
   application id), declare the *screen capture / accessibility* use honestly
   in Data safety and the permissions declaration.
3. Generate an upload key: `keytool -genkey -v -keystore upload.jks -keyalg RSA
   -keysize 2048 -validity 10000 -alias upload`. Keep it offline and back it up.
4. Build `flutter build appbundle --release` in `app/desktop` and upload the
   `.aab` to an internal testing track first. Enable Play App Signing.

### 5. Code-signing certificates
- Windows: buy an OV or EV code-signing certificate (or use Azure Trusted
  Signing, which needs identity validation). Sign with `signtool sign /fd
  SHA256 /tr <timestamp-url> /td SHA256`. Compile `packaging/windows/cbdesk.wxs`
  with WiX v4 first (it was never compiled).
- Linux: generate a release GPG key (`gpg --full-generate-key`), publish the
  public key, set `CBDESK_SIGN_KEY`, run `tools/release/sign.sh`.
- Store private keys in a hardware token or GitHub encrypted secrets only.
- Decide the update-root plan (ADR-013, TUF) before shipping any updater.

### 6. Real-device tests
1. Android: install the debug APK on a phone, run
   `CBDESK_ANDROID_DEVICE=1 tools/android/device-check.sh`, accept the system
   screen-capture dialog, confirm the session indicator is visible. Record
   results in `docs/dev/manual-tests.md`.
2. iOS: after step 3, run `tools/ios/check-controller.sh` on a Mac and install
   a TestFlight build on an iPhone.
3. Wayland: follow `docs/dev/wayland-screencast.md` on GNOME and KDE.
4. Two machines on different networks (home Wi-Fi vs. mobile hotspot): run
   `cbdesk net-check`, then connect with a ticket; note whether the path is
   direct or relay. Test a real symmetric/CGNAT network
   (`docs/dev/nat-measurement.md`).
5. Microphone/speaker: run `--mic` / `--speaker` and confirm you actually
   hear the audio. Measure 1080p on a machine with a GPU.

### 7. External security audit
1. Pick a firm with Rust and cryptography experience (e.g. Cure53, Trail of
   Bits, NCC Group, Radically Open Security, Include Security). Open-source
   funds exist: OSTIF, the Sovereign Tech Agency, NLnet (NGI Zero grants).
2. Give them `docs/dev/threat-model.md`, the ADRs, and the scope list:
   `cbdesk-crypto`, `cbdesk-net`, `cbdesk-session`, the OPAQUE and SPAKE2
   compositions, and the DHT-blinding design (ADR-007, still unbuilt).
3. Budget typically in the tens of thousands of USD; ask for a fix-review
   round and a public report.

### 8. 72-hour soak test
1. Two machines (or VMs) on different networks, one running
   `cbdesk host`, the other `cbdesk view` with a synthetic or real screen.
2. Run for 72 hours with `RUST_LOG=info`, record CPU, memory (RSS) and
   reconnect counts every minute (`pidstat`, `ps`, or Prometheus
   node_exporter). Pass: no crash, RSS growth under 10 %, reconnect works after
   a forced network drop.
3. Write the result in `docs/dev/manual-tests.md`.

### 9. Smaller decisions
- Read `docs/dev/trademark-prelim.md`; decide whether to keep the name.
- Fix the Flatpak id mismatch (`com.cbdesk.CBDesk` vs `org.cbdesk.CBDesk`).
- Merge future Dependabot PRs; update `opaque-ke` and remove the
  RUSTSEC-2026-0387 ignores when a compatible release exists.

---

## Türkçe

### 1. GitHub organizasyonu oluştur (isteğe bağlı, planda var)
1. github.com → profil menüsü → *Your organizations* → *New organization* →
   Free plan. Ad örneği: `cbdesk-org` (`cbdesk` adı 25.10.2026'da boş
   görünüyordu, ama almadan önce `trademark-prelim.md` dosyasını oku).
2. Depo *Settings → General → Transfer ownership* → yeni sahip = organizasyon.
   GitHub yönlendirmeleri, yıldızları ve issue'ları korur.
3. `docs/dev/github-setup.md` içindeki dal korumasını kontrol et (depoya
   bağlıdır, transferde korunur ama doğrula) ve organizasyon ayarlarında 2FA
   zorunluluğunu aç.
4. İkinci bir bakımcı ekle, takım oluştur ve `CODEOWNERS` dosyasını
   `crates/cbdesk-crypto` ile `crates/cbdesk-net` için iki onay isteyecek
   şekilde güncelle.

### 2. DCO GitHub App'i kur (isteğe bağlı ikinci katman)
https://github.com/apps/dco → *Configure* → depoyu seç. Depodaki kontrol
(`tools/check-dco.sh`) `Signed-off-by` satırını zaten zorunlu kılıyor. Her
commit için `git commit -s` kullan.

### 3. Apple Developer Program (iOS denetleyici, macOS notarizasyon)
1. https://developer.apple.com/programs/enroll/ adresinde Apple ID ile kayıt
   (yıllık 99 USD; bireysel için kimlik doğrulama, kurumsal için D-U-N-S no).
2. Certificates, Identifiers & Profiles: *Developer ID Application*
   (mağaza dışı macOS) ve *Apple Distribution* (iOS) sertifikaları oluştur.
   `org.cbdesk.CBDesk` paket kimliğini kaydet.
3. App Store Connect: uygulama kaydı oluştur, ardından API anahtarı üret
   (Users and Access → Integrations).
4. Mac'te `packaging/macos/make-dmg.sh` çalıştır, `codesign --options runtime`
   ile imzala, `xcrun notarytool submit` ile notarize et, sonra
   `xcrun stapler staple`.
5. Gizli anahtarları GitHub *Settings → Secrets and variables → Actions*
   altına koy (git'e asla), sonra `release.yml`'e bağla.

### 4. Google Play Console (Android)
1. https://play.google.com/console/signup (tek seferlik 25 USD; Kasım 2023
   sonrası açılan kişisel hesaplarda yayına almadan önce en az 12 test
   kullanıcısıyla 14 gün kapalı test gerekir).
2. Uygulamayı oluştur, paket adı `org.cbdesk.cbdesk` (Gradle application id ile
   aynı olmalı). Ekran yakalama/erişilebilirlik kullanımını Data safety ve
   izin beyanında dürüstçe bildir.
3. Yükleme anahtarı üret: `keytool -genkey -v -keystore upload.jks -keyalg RSA
   -keysize 2048 -validity 10000 -alias upload`. Çevrimdışı sakla, yedekle.
4. `app/desktop` içinde `flutter build appbundle --release` çalıştır, `.aab`
   dosyasını önce dahili test kanalına yükle. Play App Signing'i aç.

### 5. Kod imzalama sertifikaları
- Windows: OV veya EV kod imzalama sertifikası al (ya da kimlik doğrulaması
  gereken Azure Trusted Signing). `signtool sign /fd SHA256 /tr <zaman-damgası-url>
  /td SHA256` ile imzala. Önce `packaging/windows/cbdesk.wxs` dosyasını WiX v4
  ile derle (hiç derlenmedi).
- Linux: sürüm GPG anahtarı üret (`gpg --full-generate-key`), açık anahtarı
  yayımla, `CBDESK_SIGN_KEY` ayarla, `tools/release/sign.sh` çalıştır.
- Özel anahtarları yalnızca donanım anahtarında veya GitHub şifreli
  secret'larında tut.
- Güncelleyici yayımlamadan önce güncelleme kökü planını (ADR-013, TUF) belirle.

### 6. Gerçek cihaz testleri
1. Android: debug APK'yı telefona kur,
   `CBDESK_ANDROID_DEVICE=1 tools/android/device-check.sh` çalıştır, sistemin
   ekran yakalama iznini onayla, oturum göstergesinin göründüğünü doğrula.
   Sonuçları `docs/dev/manual-tests.md` dosyasına yaz.
2. iOS: 3. adımdan sonra Mac'te `tools/ios/check-controller.sh` çalıştır ve
   iPhone'a TestFlight sürümü kur.
3. Wayland: `docs/dev/wayland-screencast.md` adımlarını GNOME ve KDE'de izle.
4. Farklı ağlardaki iki makine (ev Wi-Fi ve mobil hotspot): `cbdesk net-check`
   çalıştır, bilet ile bağlan, yolun doğrudan mı röle mi olduğunu not et. Gerçek
   simetrik NAT/CGNAT ağında dene (`docs/dev/nat-measurement.md`).
5. Mikrofon/hoparlör: `--mic` / `--speaker` ile sesi gerçekten duyduğunu
   doğrula. 1080p performansını GPU'lu bir makinede ölç.

### 7. Dış güvenlik denetimi
1. Rust ve kriptografi deneyimi olan bir firma seç (ör. Cure53, Trail of Bits,
   NCC Group, Radically Open Security, Include Security). Açık kaynak için
   fonlar var: OSTIF, Sovereign Tech Agency, NLnet (NGI Zero).
2. `docs/dev/threat-model.md`, ADR'ler ve kapsam listesini ver:
   `cbdesk-crypto`, `cbdesk-net`, `cbdesk-session`, OPAQUE ve SPAKE2
   bileşimleri, DHT körleme tasarımı (ADR-007, henüz yazılmadı).
3. Bütçe genelde onbinlerce USD; düzeltme doğrulama turu ve herkese açık rapor
   iste.

### 8. 72 saatlik dayanıklılık testi
1. Farklı ağlarda iki makine (veya VM): biri `cbdesk host`, diğeri
   `cbdesk view` çalıştırsın.
2. `RUST_LOG=info` ile 72 saat çalıştır; CPU, bellek (RSS) ve yeniden bağlanma
   sayısını dakikada bir kaydet (`pidstat`, `ps` veya Prometheus
   node_exporter). Başarı: çökme yok, RSS artışı %10'un altında, ağ kesilince
   yeniden bağlanma çalışıyor.
3. Sonucu `docs/dev/manual-tests.md` dosyasına yaz.

### 9. Küçük kararlar
- `docs/dev/trademark-prelim.md` dosyasını oku; adı koruyup korumayacağına karar ver.
- Flatpak kimlik uyuşmazlığını düzelt (`com.cbdesk.CBDesk` / `org.cbdesk.CBDesk`).
- Dependabot PR'larını birleştir; uyumlu sürüm çıkınca `opaque-ke`'yi güncelle
  ve RUSTSEC-2026-0387 yok saymalarını kaldır.
