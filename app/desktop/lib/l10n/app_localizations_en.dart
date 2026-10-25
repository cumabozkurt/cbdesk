// ignore: unused_import
import 'package:intl/intl.dart' as intl;

import 'app_localizations.dart';

// ignore_for_file: type=lint

/// The translations for English (`en`).
class AppLocalizationsEn extends AppLocalizations {
  AppLocalizationsEn([String locale = 'en']) : super(locale);

  @override
  String get appTitle => 'CBDesk';

  @override
  String get navHome => 'Home';

  @override
  String get navConnect => 'Connect';

  @override
  String get navRemote => 'Remote';

  @override
  String get navFiles => 'Files';

  @override
  String get navAddress => 'Address book';

  @override
  String get navSettings => 'Settings';

  @override
  String get thisDevice => 'This device';

  @override
  String get deviceId => 'Device id';

  @override
  String get shortFingerprint => 'Fingerprint';

  @override
  String get shortCode => 'Short code';

  @override
  String get shortCodeDoesNotAuthenticate =>
      'The two words confirm pairing with this device. They do not locate a computer, and the host still has to approve.';

  @override
  String get ticketLocator => 'Invite ticket';

  @override
  String get ticketIsNotAGrant => 'A ticket is a locator, not a permission.';

  @override
  String phaseLabel(String phase) {
    return 'State: $phase';
  }

  @override
  String get connectTitle => 'Connect';

  @override
  String get connectHint => 'Paste the invite ticket';

  @override
  String get connectCodeHint => 'Short code, after the ticket';

  @override
  String get connectButton => 'Connect';

  @override
  String get errorShortCode =>
      'A short code does not locate a computer. Paste the ticket, then the code.';

  @override
  String get errorBlocked => 'That device is blocked.';

  @override
  String get errorNoPrompt => 'There is no consent prompt.';

  @override
  String get errorLooseSeed => 'The identity file is readable by other users.';

  @override
  String errorMessage(String detail) {
    return '$detail';
  }

  @override
  String get consentTitle => 'Connection request';

  @override
  String get fraudWarning =>
      'If someone called you and told you to install this program, reject the connection. Banks, government agencies, and support companies do not ask you for remote access.';

  @override
  String get consentDelay =>
      'On a first contact, Accept stays off for a few seconds.';

  @override
  String get consentBody =>
      'A device wants the permissions you tick. Be sure who is on the other side before you accept.';

  @override
  String get accept => 'Accept';

  @override
  String get reject => 'Reject';

  @override
  String get block => 'Block';

  @override
  String get permView => 'View the screen';

  @override
  String get permInput => 'Mouse and keyboard';

  @override
  String get permClipboard => 'Clipboard text';

  @override
  String get permAudio => 'Audio';

  @override
  String get permFileRead => 'Read files';

  @override
  String get permFileWrite => 'Write files';

  @override
  String get highRiskOptIn =>
      'I am opting in to the high-risk permissions I ticked.';

  @override
  String get indicatorTitle => 'Session';

  @override
  String get indicatorCannotDisable => 'This indicator cannot be turned off.';

  @override
  String get disconnect => 'Disconnect';

  @override
  String peerLabel(String id) {
    return 'Peer: $id';
  }

  @override
  String get remoteTitle => 'Remote view';

  @override
  String get remoteHonest =>
      'This picture is the frame the core decoded. A desktop host sends the operating-system capture after you accept. Android sends a frame only after the system capture consent.';

  @override
  String get pointerHint =>
      'A click on the picture is sent after an Input grant.';

  @override
  String get sendEnter => 'Send Enter';

  @override
  String lastInputLabel(String text) {
    return 'Last input: $text';
  }

  @override
  String get clipboardHint => 'Clipboard text';

  @override
  String get setClipboard => 'Set clipboard';

  @override
  String clipboardLabel(String text) {
    return 'Clipboard: $text';
  }

  @override
  String get micLabel => 'Microphone';

  @override
  String get micHonest =>
      'This switch opens the default microphone and sends packets after an Audio grant. A missing device does not fail the session. This build does not record that anyone heard the audio.';

  @override
  String get micDenied => 'The microphone stays off until you allow it.';

  @override
  String audioPacketsLabel(int count) {
    return 'Audio packets: $count';
  }

  @override
  String get remoteEmpty => 'No picture yet. The host still has to accept.';

  @override
  String get interFrame => 'Inter frame';

  @override
  String get keyFrame => 'Keyframe';

  @override
  String get filesTitle => 'Files';

  @override
  String get fileHint => 'Name inside the transfer folder';

  @override
  String get stageFile => 'Stage';

  @override
  String get fileBodyHint => 'Text to send as file bytes';

  @override
  String get sendFile => 'Send';

  @override
  String get staged => 'Staged';

  @override
  String get jailFiles => 'In the folder';

  @override
  String get addressTitle => 'Address book';

  @override
  String get contactName => 'Name';

  @override
  String get contactId => 'Device id';

  @override
  String get addContact => 'Add';

  @override
  String get remove => 'Remove';

  @override
  String get settingsTitle => 'Settings';

  @override
  String get localeLabel => 'Language';

  @override
  String get relayLabel => 'Relay';

  @override
  String get relayHint => 'Empty, or an https:// or relay:// URL you run';

  @override
  String get save => 'Save';

  @override
  String get relayInvalid =>
      'The relay must be empty or start with https:// or relay://.';

  @override
  String audioInput(String name) {
    return 'Microphone: $name';
  }

  @override
  String audioOutput(String name) {
    return 'Speaker: $name';
  }

  @override
  String get audioHonest =>
      'These are the default device names. The remote view switch sends microphone packets after an Audio grant.';

  @override
  String get desktopCaptureNote =>
      'This desktop host captures the display inside the core after the other side connects and you accept.';

  @override
  String get shareScreen => 'Share this screen';

  @override
  String get shareDenied =>
      'Screen capture stays off until you accept the system dialog.';

  @override
  String get refreshAudio => 'Refresh devices';

  @override
  String get loading => 'Loading';

  @override
  String grantedLabel(String names) {
    return 'Granted: $names';
  }

  @override
  String get languageTurkish => 'Türkçe';

  @override
  String get languageEnglish => 'English';
}
