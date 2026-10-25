import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_localizations/flutter_localizations.dart';
import 'package:intl/intl.dart' as intl;

import 'app_localizations_en.dart';
import 'app_localizations_tr.dart';

// ignore_for_file: type=lint

/// Callers can lookup localized strings with an instance of AppLocalizations
/// returned by `AppLocalizations.of(context)`.
///
/// Applications need to include `AppLocalizations.delegate()` in their app's
/// `localizationDelegates` list, and the locales they support in the app's
/// `supportedLocales` list. For example:
///
/// ```dart
/// import 'l10n/app_localizations.dart';
///
/// return MaterialApp(
///   localizationsDelegates: AppLocalizations.localizationsDelegates,
///   supportedLocales: AppLocalizations.supportedLocales,
///   home: MyApplicationHome(),
/// );
/// ```
///
/// ## Update pubspec.yaml
///
/// Please make sure to update your pubspec.yaml to include the following
/// packages:
///
/// ```yaml
/// dependencies:
///   # Internationalization support.
///   flutter_localizations:
///     sdk: flutter
///   intl: any # Use the pinned version from flutter_localizations
///
///   # Rest of dependencies
/// ```
///
/// ## iOS Applications
///
/// iOS applications define key application metadata, including supported
/// locales, in an Info.plist file that is built into the application bundle.
/// To configure the locales supported by your app, you’ll need to edit this
/// file.
///
/// First, open your project’s ios/Runner.xcworkspace Xcode workspace file.
/// Then, in the Project Navigator, open the Info.plist file under the Runner
/// project’s Runner folder.
///
/// Next, select the Information Property List item, select Add Item from the
/// Editor menu, then select Localizations from the pop-up menu.
///
/// Select and expand the newly-created Localizations item then, for each
/// locale your application supports, add a new item and select the locale
/// you wish to add from the pop-up menu in the Value field. This list should
/// be consistent with the languages listed in the AppLocalizations.supportedLocales
/// property.
abstract class AppLocalizations {
  AppLocalizations(String locale)
    : localeName = intl.Intl.canonicalizedLocale(locale.toString());

  final String localeName;

  static AppLocalizations of(BuildContext context) {
    return Localizations.of<AppLocalizations>(context, AppLocalizations)!;
  }

  static const LocalizationsDelegate<AppLocalizations> delegate =
      _AppLocalizationsDelegate();

  /// A list of this localizations delegate along with the default localizations
  /// delegates.
  ///
  /// Returns a list of localizations delegates containing this delegate along with
  /// GlobalMaterialLocalizations.delegate, GlobalCupertinoLocalizations.delegate,
  /// and GlobalWidgetsLocalizations.delegate.
  ///
  /// Additional delegates can be added by appending to this list in
  /// MaterialApp. This list does not have to be used at all if a custom list
  /// of delegates is preferred or required.
  static const List<LocalizationsDelegate<dynamic>> localizationsDelegates =
      <LocalizationsDelegate<dynamic>>[
        delegate,
        GlobalMaterialLocalizations.delegate,
        GlobalCupertinoLocalizations.delegate,
        GlobalWidgetsLocalizations.delegate,
      ];

  /// A list of this localizations delegate's supported locales.
  static const List<Locale> supportedLocales = <Locale>[
    Locale('en'),
    Locale('tr'),
  ];

  /// No description provided for @appTitle.
  ///
  /// In en, this message translates to:
  /// **'CBDesk'**
  String get appTitle;

  /// No description provided for @navHome.
  ///
  /// In en, this message translates to:
  /// **'Home'**
  String get navHome;

  /// No description provided for @navConnect.
  ///
  /// In en, this message translates to:
  /// **'Connect'**
  String get navConnect;

  /// No description provided for @navRemote.
  ///
  /// In en, this message translates to:
  /// **'Remote'**
  String get navRemote;

  /// No description provided for @navFiles.
  ///
  /// In en, this message translates to:
  /// **'Files'**
  String get navFiles;

  /// No description provided for @navAddress.
  ///
  /// In en, this message translates to:
  /// **'Address book'**
  String get navAddress;

  /// No description provided for @navSettings.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get navSettings;

  /// No description provided for @thisDevice.
  ///
  /// In en, this message translates to:
  /// **'This device'**
  String get thisDevice;

  /// No description provided for @deviceId.
  ///
  /// In en, this message translates to:
  /// **'Device id'**
  String get deviceId;

  /// No description provided for @shortFingerprint.
  ///
  /// In en, this message translates to:
  /// **'Fingerprint'**
  String get shortFingerprint;

  /// No description provided for @shortCode.
  ///
  /// In en, this message translates to:
  /// **'Short code'**
  String get shortCode;

  /// No description provided for @shortCodeDoesNotAuthenticate.
  ///
  /// In en, this message translates to:
  /// **'The two words confirm pairing with this device. They do not locate a computer, and the host still has to approve.'**
  String get shortCodeDoesNotAuthenticate;

  /// No description provided for @ticketLocator.
  ///
  /// In en, this message translates to:
  /// **'Invite ticket'**
  String get ticketLocator;

  /// No description provided for @ticketIsNotAGrant.
  ///
  /// In en, this message translates to:
  /// **'A ticket is a locator, not a permission.'**
  String get ticketIsNotAGrant;

  /// No description provided for @phaseLabel.
  ///
  /// In en, this message translates to:
  /// **'State: {phase}'**
  String phaseLabel(String phase);

  /// No description provided for @connectTitle.
  ///
  /// In en, this message translates to:
  /// **'Connect'**
  String get connectTitle;

  /// No description provided for @connectHint.
  ///
  /// In en, this message translates to:
  /// **'Paste the invite ticket'**
  String get connectHint;

  /// No description provided for @connectCodeHint.
  ///
  /// In en, this message translates to:
  /// **'Short code, after the ticket'**
  String get connectCodeHint;

  /// No description provided for @connectButton.
  ///
  /// In en, this message translates to:
  /// **'Connect'**
  String get connectButton;

  /// No description provided for @errorShortCode.
  ///
  /// In en, this message translates to:
  /// **'A short code does not locate a computer. Paste the ticket, then the code.'**
  String get errorShortCode;

  /// No description provided for @errorBlocked.
  ///
  /// In en, this message translates to:
  /// **'That device is blocked.'**
  String get errorBlocked;

  /// No description provided for @errorNoPrompt.
  ///
  /// In en, this message translates to:
  /// **'There is no consent prompt.'**
  String get errorNoPrompt;

  /// No description provided for @errorLooseSeed.
  ///
  /// In en, this message translates to:
  /// **'The identity file is readable by other users.'**
  String get errorLooseSeed;

  /// No description provided for @errorMessage.
  ///
  /// In en, this message translates to:
  /// **'{detail}'**
  String errorMessage(String detail);

  /// No description provided for @consentTitle.
  ///
  /// In en, this message translates to:
  /// **'Connection request'**
  String get consentTitle;

  /// No description provided for @fraudWarning.
  ///
  /// In en, this message translates to:
  /// **'If someone called you and told you to install this program, reject the connection. Banks, government agencies, and support companies do not ask you for remote access.'**
  String get fraudWarning;

  /// No description provided for @consentDelay.
  ///
  /// In en, this message translates to:
  /// **'On a first contact, Accept stays off for a few seconds.'**
  String get consentDelay;

  /// No description provided for @consentBody.
  ///
  /// In en, this message translates to:
  /// **'A device wants the permissions you tick. Be sure who is on the other side before you accept.'**
  String get consentBody;

  /// No description provided for @accept.
  ///
  /// In en, this message translates to:
  /// **'Accept'**
  String get accept;

  /// No description provided for @reject.
  ///
  /// In en, this message translates to:
  /// **'Reject'**
  String get reject;

  /// No description provided for @block.
  ///
  /// In en, this message translates to:
  /// **'Block'**
  String get block;

  /// No description provided for @permView.
  ///
  /// In en, this message translates to:
  /// **'View the screen'**
  String get permView;

  /// No description provided for @permInput.
  ///
  /// In en, this message translates to:
  /// **'Mouse and keyboard'**
  String get permInput;

  /// No description provided for @permClipboard.
  ///
  /// In en, this message translates to:
  /// **'Clipboard text'**
  String get permClipboard;

  /// No description provided for @permAudio.
  ///
  /// In en, this message translates to:
  /// **'Audio'**
  String get permAudio;

  /// No description provided for @permFileRead.
  ///
  /// In en, this message translates to:
  /// **'Read files'**
  String get permFileRead;

  /// No description provided for @permFileWrite.
  ///
  /// In en, this message translates to:
  /// **'Write files'**
  String get permFileWrite;

  /// No description provided for @highRiskOptIn.
  ///
  /// In en, this message translates to:
  /// **'I am opting in to the high-risk permissions I ticked.'**
  String get highRiskOptIn;

  /// No description provided for @indicatorTitle.
  ///
  /// In en, this message translates to:
  /// **'Session'**
  String get indicatorTitle;

  /// No description provided for @indicatorCannotDisable.
  ///
  /// In en, this message translates to:
  /// **'This indicator cannot be turned off.'**
  String get indicatorCannotDisable;

  /// No description provided for @disconnect.
  ///
  /// In en, this message translates to:
  /// **'Disconnect'**
  String get disconnect;

  /// No description provided for @peerLabel.
  ///
  /// In en, this message translates to:
  /// **'Peer: {id}'**
  String peerLabel(String id);

  /// No description provided for @remoteTitle.
  ///
  /// In en, this message translates to:
  /// **'Remote view'**
  String get remoteTitle;

  /// No description provided for @remoteHonest.
  ///
  /// In en, this message translates to:
  /// **'This picture is the frame the core decoded. A desktop host sends the operating-system capture after you accept. Android sends a frame only after the system capture consent.'**
  String get remoteHonest;

  /// No description provided for @pointerHint.
  ///
  /// In en, this message translates to:
  /// **'A click on the picture is sent after an Input grant.'**
  String get pointerHint;

  /// No description provided for @sendEnter.
  ///
  /// In en, this message translates to:
  /// **'Send Enter'**
  String get sendEnter;

  /// No description provided for @lastInputLabel.
  ///
  /// In en, this message translates to:
  /// **'Last input: {text}'**
  String lastInputLabel(String text);

  /// No description provided for @clipboardHint.
  ///
  /// In en, this message translates to:
  /// **'Clipboard text'**
  String get clipboardHint;

  /// No description provided for @setClipboard.
  ///
  /// In en, this message translates to:
  /// **'Set clipboard'**
  String get setClipboard;

  /// No description provided for @clipboardLabel.
  ///
  /// In en, this message translates to:
  /// **'Clipboard: {text}'**
  String clipboardLabel(String text);

  /// No description provided for @micLabel.
  ///
  /// In en, this message translates to:
  /// **'Microphone'**
  String get micLabel;

  /// No description provided for @micHonest.
  ///
  /// In en, this message translates to:
  /// **'This switch opens the default microphone and sends packets after an Audio grant. A missing device does not fail the session. This build does not record that anyone heard the audio.'**
  String get micHonest;

  /// No description provided for @micDenied.
  ///
  /// In en, this message translates to:
  /// **'The microphone stays off until you allow it.'**
  String get micDenied;

  /// No description provided for @audioPacketsLabel.
  ///
  /// In en, this message translates to:
  /// **'Audio packets: {count}'**
  String audioPacketsLabel(int count);

  /// No description provided for @remoteEmpty.
  ///
  /// In en, this message translates to:
  /// **'No picture yet. The host still has to accept.'**
  String get remoteEmpty;

  /// No description provided for @interFrame.
  ///
  /// In en, this message translates to:
  /// **'Inter frame'**
  String get interFrame;

  /// No description provided for @keyFrame.
  ///
  /// In en, this message translates to:
  /// **'Keyframe'**
  String get keyFrame;

  /// No description provided for @filesTitle.
  ///
  /// In en, this message translates to:
  /// **'Files'**
  String get filesTitle;

  /// No description provided for @fileHint.
  ///
  /// In en, this message translates to:
  /// **'Name inside the transfer folder'**
  String get fileHint;

  /// No description provided for @stageFile.
  ///
  /// In en, this message translates to:
  /// **'Stage'**
  String get stageFile;

  /// No description provided for @fileBodyHint.
  ///
  /// In en, this message translates to:
  /// **'Text to send as file bytes'**
  String get fileBodyHint;

  /// No description provided for @sendFile.
  ///
  /// In en, this message translates to:
  /// **'Send'**
  String get sendFile;

  /// No description provided for @staged.
  ///
  /// In en, this message translates to:
  /// **'Staged'**
  String get staged;

  /// No description provided for @jailFiles.
  ///
  /// In en, this message translates to:
  /// **'In the folder'**
  String get jailFiles;

  /// No description provided for @addressTitle.
  ///
  /// In en, this message translates to:
  /// **'Address book'**
  String get addressTitle;

  /// No description provided for @contactName.
  ///
  /// In en, this message translates to:
  /// **'Name'**
  String get contactName;

  /// No description provided for @contactId.
  ///
  /// In en, this message translates to:
  /// **'Device id'**
  String get contactId;

  /// No description provided for @addContact.
  ///
  /// In en, this message translates to:
  /// **'Add'**
  String get addContact;

  /// No description provided for @remove.
  ///
  /// In en, this message translates to:
  /// **'Remove'**
  String get remove;

  /// No description provided for @settingsTitle.
  ///
  /// In en, this message translates to:
  /// **'Settings'**
  String get settingsTitle;

  /// No description provided for @localeLabel.
  ///
  /// In en, this message translates to:
  /// **'Language'**
  String get localeLabel;

  /// No description provided for @relayLabel.
  ///
  /// In en, this message translates to:
  /// **'Relay'**
  String get relayLabel;

  /// No description provided for @relayHint.
  ///
  /// In en, this message translates to:
  /// **'Empty, or an https:// or relay:// URL you run'**
  String get relayHint;

  /// No description provided for @save.
  ///
  /// In en, this message translates to:
  /// **'Save'**
  String get save;

  /// No description provided for @relayInvalid.
  ///
  /// In en, this message translates to:
  /// **'The relay must be empty or start with https:// or relay://.'**
  String get relayInvalid;

  /// No description provided for @audioInput.
  ///
  /// In en, this message translates to:
  /// **'Microphone: {name}'**
  String audioInput(String name);

  /// No description provided for @audioOutput.
  ///
  /// In en, this message translates to:
  /// **'Speaker: {name}'**
  String audioOutput(String name);

  /// No description provided for @audioHonest.
  ///
  /// In en, this message translates to:
  /// **'These are the default device names. The remote view switch sends microphone packets after an Audio grant.'**
  String get audioHonest;

  /// No description provided for @desktopCaptureNote.
  ///
  /// In en, this message translates to:
  /// **'This desktop host captures the display inside the core after the other side connects and you accept.'**
  String get desktopCaptureNote;

  /// No description provided for @shareScreen.
  ///
  /// In en, this message translates to:
  /// **'Share this screen'**
  String get shareScreen;

  /// No description provided for @shareDenied.
  ///
  /// In en, this message translates to:
  /// **'Screen capture stays off until you accept the system dialog.'**
  String get shareDenied;

  /// No description provided for @refreshAudio.
  ///
  /// In en, this message translates to:
  /// **'Refresh devices'**
  String get refreshAudio;

  /// No description provided for @loading.
  ///
  /// In en, this message translates to:
  /// **'Loading'**
  String get loading;

  /// No description provided for @grantedLabel.
  ///
  /// In en, this message translates to:
  /// **'Granted: {names}'**
  String grantedLabel(String names);

  /// Native name of the Turkish UI locale.
  ///
  /// In en, this message translates to:
  /// **'Türkçe'**
  String get languageTurkish;

  /// Native name of the English UI locale.
  ///
  /// In en, this message translates to:
  /// **'English'**
  String get languageEnglish;
}

class _AppLocalizationsDelegate
    extends LocalizationsDelegate<AppLocalizations> {
  const _AppLocalizationsDelegate();

  @override
  Future<AppLocalizations> load(Locale locale) {
    return SynchronousFuture<AppLocalizations>(lookupAppLocalizations(locale));
  }

  @override
  bool isSupported(Locale locale) =>
      <String>['en', 'tr'].contains(locale.languageCode);

  @override
  bool shouldReload(_AppLocalizationsDelegate old) => false;
}

AppLocalizations lookupAppLocalizations(Locale locale) {
  // Lookup logic when only language code is specified.
  switch (locale.languageCode) {
    case 'en':
      return AppLocalizationsEn();
    case 'tr':
      return AppLocalizationsTr();
  }

  throw FlutterError(
    'AppLocalizations.delegate failed to load unsupported locale "$locale". This is likely '
    'an issue with the localizations generation tool. Please file an issue '
    'on GitHub with a reproducible sample app and the gen-l10n configuration '
    'that was used.',
  );
}
