/// Screen data and the actions the shell can take.
///
/// Widget tests use a fake. The running app uses the Rust bridge.
library;

/// One consent checkbox supplied by the core.
class PermissionChoice {
  /// Protocol name, such as `view` or `file_write`.
  const PermissionChoice({required this.name, required this.highRisk});

  /// Protocol name.
  final String name;

  /// True when a first contact must opt in before this can be granted.
  final bool highRisk;
}

/// One address-book row.
class DeskContact {
  /// A row the person saved.
  const DeskContact({required this.name, required this.deviceId});

  /// Display name.
  final String name;

  /// Canonical device id.
  final String deviceId;
}

/// What the shell draws. Strings are data; the widgets own translations.
class DeskView {
  /// A snapshot from the core or from a test.
  const DeskView({
    required this.deviceId,
    required this.shortFingerprint,
    required this.shortCode,
    required this.shortCodeAuthenticates,
    required this.ticketHex,
    required this.locale,
    required this.phase,
    required this.indicatorVisible,
    required this.indicatorCanBeDisabled,
    required this.acceptEnabled,
    required this.fraudWarning,
    required this.firstContact,
    required this.peerId,
    required this.fingerprint,
    required this.granted,
    required this.frameRgba,
    required this.frameWidth,
    required this.frameHeight,
    required this.frameIsInter,
    required this.files,
    required this.contacts,
    required this.staged,
    required this.audioInput,
    required this.audioOutput,
    required this.relay,
    required this.error,
    required this.blocked,
    required this.clipboard,
    required this.audioPackets,
    required this.lastInput,
  });

  /// Full device id.
  final String deviceId;

  /// Display fingerprint.
  final String shortFingerprint;

  /// `NNN-WORD-WORD`. The words confirm pairing. They do not locate a peer.
  final String shortCode;

  /// True when the words can confirm pairing. The ticket is still the locator.
  final bool shortCodeAuthenticates;

  /// Signed invite ticket, hex.
  final String ticketHex;

  /// `tr` or `en`.
  final String locale;

  /// `idle`, `awaiting`, `active`, or `closed`.
  final String phase;

  /// True while consent is pending or the session is active.
  final bool indicatorVisible;

  /// Always false. The shell has no control that hides the indicator.
  final bool indicatorCanBeDisabled;

  /// False until the first-contact delay has elapsed.
  final bool acceptEnabled;

  /// True while the consent card should show the fraud warning.
  final bool fraudWarning;

  /// True when this prompt is a first contact.
  final bool firstContact;

  /// Peer device id.
  final String peerId;

  /// Peer fingerprint.
  final String fingerprint;

  /// Permissions granted on the active session.
  final List<String> granted;

  /// Decoded picture, tightly packed RGBA.
  final List<int> frameRgba;

  /// Frame width. Zero when no picture has been decoded.
  final int frameWidth;

  /// Frame height.
  final int frameHeight;

  /// True when the shown picture was decoded from an inter frame.
  final bool frameIsInter;

  /// File names inside the transfer directory.
  final List<String> files;

  /// Address book.
  final List<DeskContact> contacts;

  /// Names the jail has already accepted.
  final List<String> staged;

  /// Default input device, or `unavailable`.
  final String audioInput;

  /// Default output device, or `unavailable`.
  final String audioOutput;

  /// Operator relay URL.
  final String relay;

  /// Last local error from the core. Empty when the last action succeeded.
  final String error;

  /// Device ids this desk will not prompt for.
  final List<String> blocked;

  /// Latest clipboard text this desk applied or received.
  final String clipboard;

  /// Audio packets the host has accepted.
  final int audioPackets;

  /// Last input the host recorded or injected.
  final String lastInput;
}

/// A failure the shell can translate.
class DeskFailure implements Exception {
  /// `short_code_needs_ticket`, `blocked`, `no_prompt`, `loose_seed`, or `message:…`.
  DeskFailure(this.code);

  /// Stable code from the core.
  final String code;

  @override
  String toString() => code;
}

/// The desktop core, as the widgets see it.
abstract class DeskApi {
  /// Current screen data.
  Future<DeskView> snapshot();

  /// Consent checkboxes, including which ones are high risk.
  Future<List<PermissionChoice>> permissions();

  /// Dial a ticket. An optional second field is the short code.
  ///
  /// The code alone does not locate a computer.
  Future<void> connect(String text);

  /// Approve the current prompt.
  Future<void> approve(List<String> perms, bool explicitHighRisk);

  /// Deny the prompt.
  Future<void> reject();

  /// Reject this prompt and refuse the same device id later.
  Future<void> block();

  /// End the prompt or the session. This is the indicator's Kes button.
  Future<void> disconnect();

  /// `tr` or `en`.
  Future<void> setLocale(String locale);

  /// Store an operator relay URL. Empty clears it.
  Future<void> setRelay(String relay);

  /// Re-read the default device names.
  Future<void> refreshAudio();

  /// Remember a contact.
  Future<void> addContact(String name, String deviceId);

  /// Remove a contact.
  Future<void> removeContact(String deviceId);

  /// Jail `name` and remember it.
  Future<void> stageFile(String name);

  /// Send a pointer sample. Requires an Input grant.
  Future<void> sendPointer(int x, int y, int buttons, int wheel);

  /// Send a key transition. Requires an Input grant.
  Future<void> sendKey(int keycode, bool down, int modifiers);

  /// Send file bytes. The receiver jails [name]. Requires FileWrite.
  Future<void> sendFile(String name, List<int> bytes);

  /// Send clipboard text. Empty clears. Requires Clipboard.
  Future<void> setClipboard(String text);

  /// Send one audio packet. Requires Audio.
  Future<void> sendAudio(List<int> bytes);

  /// Open or drop the default microphone on this controller.
  Future<void> setMic(bool enabled);

  /// Queue one RGBA frame. Android calls this after system capture consent.
  Future<void> submitFrame(int width, int height, List<int> rgba);
}
