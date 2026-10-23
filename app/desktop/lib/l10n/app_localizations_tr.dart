// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for Turkish (`tr`).
class AppLocalizationsTr extends AppLocalizations {
  AppLocalizationsTr([String locale = 'tr']) : super(locale);

  @override
  String get appTitle => 'CBDesk';

  @override
  String get navHome => 'Ana sayfa';

  @override
  String get navConnect => 'Bağlan';

  @override
  String get navRemote => 'Uzak ekran';

  @override
  String get navFiles => 'Dosyalar';

  @override
  String get navAddress => 'Adres defteri';

  @override
  String get navSettings => 'Ayarlar';

  @override
  String get thisDevice => 'Bu cihaz';

  @override
  String get deviceId => 'Cihaz kimliği';

  @override
  String get shortFingerprint => 'Parmak izi';

  @override
  String get shortCode => 'Kısa kod';

  @override
  String get shortCodeDoesNotAuthenticate =>
      'Bu kısa kod kimseyi doğrulamaz. Parola değildir.';

  @override
  String get ticketLocator => 'Davet bileti';

  @override
  String get ticketIsNotAGrant => 'Bilet bir yer tarifidir, izin değildir.';

  @override
  String phaseLabel(String phase) {
    return 'Durum: $phase';
  }

  @override
  String get connectTitle => 'Bağlan';

  @override
  String get connectHint => 'Davet biletini yapıştırın';

  @override
  String get connectButton => 'Bağlan';

  @override
  String get errorShortCode =>
      'Kısa kod kimseyi doğrulamaz. Bileti yapıştırın.';

  @override
  String get errorBlocked => 'Bu cihaz engelli.';

  @override
  String get errorNoPrompt => 'Onay isteği yok.';

  @override
  String get errorLooseSeed =>
      'Kimlik dosyasını başka kullanıcılar okuyabiliyor.';

  @override
  String errorMessage(String detail) {
    return '$detail';
  }

  @override
  String get consentTitle => 'Bağlantı isteği';

  @override
  String get fraudWarning =>
      'Sizi telefonla arayıp bu programı kurduran biri varsa bağlantıyı reddedin. Bankalar, devlet kurumları ve teknik destek firmaları sizden uzaktan erişim istemez.';

  @override
  String get consentDelay =>
      'İlk bağlantıda Kabul et düğmesi birkaç saniye kapalı kalır.';

  @override
  String get consentBody =>
      'Bir cihaz, işaretlediğiniz izinleri istiyor. Kabul etmeden önce karşınızdakinin kim olduğundan emin olun.';

  @override
  String get accept => 'Kabul et';

  @override
  String get reject => 'Reddet';

  @override
  String get block => 'Engelle';

  @override
  String get permView => 'Ekranı görsün';

  @override
  String get permInput => 'Fare ve klavye';

  @override
  String get permClipboard => 'Pano metni';

  @override
  String get permAudio => 'Ses';

  @override
  String get permFileRead => 'Dosya okusun';

  @override
  String get permFileWrite => 'Dosya yazsın';

  @override
  String get highRiskOptIn =>
      'İşaretlediğim yüksek riskli izinlere bilerek onay veriyorum.';

  @override
  String get indicatorTitle => 'Oturum';

  @override
  String get indicatorCannotDisable => 'Bu gösterge kapatılamaz.';

  @override
  String get disconnect => 'Kes';

  @override
  String peerLabel(String id) {
    return 'Karşı taraf: $id';
  }

  @override
  String get remoteTitle => 'Uzak ekran';

  @override
  String get remoteHonest =>
      'Bu görüntü, çekirdeğin çözdüğü karedir. Bu kabuk tam ekran yakalaması göstermez.';

  @override
  String get remoteEmpty =>
      'Henüz görüntü yok. Karşı tarafın kabul etmesi gerekir.';

  @override
  String get interFrame => 'Ara kare';

  @override
  String get keyFrame => 'Ana kare';

  @override
  String get filesTitle => 'Dosyalar';

  @override
  String get fileHint => 'Aktarım klasörünün içindeki ad';

  @override
  String get stageFile => 'Hazırla';

  @override
  String get staged => 'Hazırlananlar';

  @override
  String get jailFiles => 'Klasörde';

  @override
  String get addressTitle => 'Adres defteri';

  @override
  String get contactName => 'Ad';

  @override
  String get contactId => 'Cihaz kimliği';

  @override
  String get addContact => 'Ekle';

  @override
  String get remove => 'Kaldır';

  @override
  String get settingsTitle => 'Ayarlar';

  @override
  String get localeLabel => 'Dil';

  @override
  String get relayLabel => 'Aktarıcı';

  @override
  String get relayHint =>
      'Boş, ya da sizin çalıştırdığınız bir https:// veya relay:// adresi';

  @override
  String get save => 'Kaydet';

  @override
  String get relayInvalid =>
      'Aktarıcı boş olmalı ya da https:// veya relay:// ile başlamalı.';

  @override
  String audioInput(String name) {
    return 'Mikrofon: $name';
  }

  @override
  String audioOutput(String name) {
    return 'Hoparlör: $name';
  }

  @override
  String get audioHonest =>
      'Bu ekran cihaz adlarını gösterir. Mikrofon ve hoparlör sesi cbdesk host --mic ve --speaker yoludur.';

  @override
  String get refreshAudio => 'Cihazları yenile';

  @override
  String get loading => 'Yükleniyor';

  @override
  String grantedLabel(String names) {
    return 'Verilenler: $names';
  }
}
