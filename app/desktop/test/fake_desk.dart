import 'package:cbdesk/desk_api.dart';

/// In-memory desk for widget tests. It does not load the Rust library.
class FakeDesk implements DeskApi {
  /// Starts at home, in Turkish, with the indicator off.
  FakeDesk({DeskView? view}) : view = view ?? sampleView();

  /// Mutable screen data.
  DeskView view;

  /// Last connect text.
  String? connected;

  /// Last approve call.
  List<String>? approved;

  /// Whether the last approve opted into high risk.
  bool? explicitHighRisk;

  /// How many times reject was called.
  int rejects = 0;

  /// How many times block was called.
  int blocks = 0;

  /// How many times disconnect was called.
  int disconnects = 0;

  /// Last pointer sample.
  String? pointer;

  /// Last key transition.
  String? keyEvent;

  /// Last file body.
  List<int>? fileBytes;

  /// Whether the microphone switch is on.
  bool mic = false;

  @override
  Future<DeskView> snapshot() async => view;

  @override
  Future<List<PermissionChoice>> permissions() async {
    return const [
      PermissionChoice(name: 'view', highRisk: false),
      PermissionChoice(name: 'input', highRisk: false),
      PermissionChoice(name: 'clipboard', highRisk: false),
      PermissionChoice(name: 'audio', highRisk: false),
      PermissionChoice(name: 'file_read', highRisk: false),
      PermissionChoice(name: 'file_write', highRisk: true),
    ];
  }

  @override
  Future<void> connect(String text) async {
    final trimmed = text.trim();
    if (!trimmed.contains(' ') && trimmed.contains('-')) {
      throw DeskFailure('short_code_needs_ticket');
    }
    connected = trimmed;
    view = _copy(
      phase: 'active',
      indicatorVisible: true,
      frameWidth: 2,
      frameHeight: 1,
      frameRgba: const [255, 0, 0, 255, 0, 0, 255, 255],
      frameIsInter: true,
    );
  }

  @override
  Future<void> approve(List<String> perms, bool explicitHighRisk) async {
    approved = perms;
    this.explicitHighRisk = explicitHighRisk;
    view = _copy(
      phase: 'active',
      acceptEnabled: false,
      indicatorVisible: true,
      granted: perms,
    );
  }

  @override
  Future<void> reject() async {
    rejects += 1;
    view = _copy(
      phase: 'idle',
      indicatorVisible: false,
      fraudWarning: false,
      acceptEnabled: false,
    );
  }

  @override
  Future<void> block() async {
    blocks += 1;
    view = _copy(
      phase: 'idle',
      indicatorVisible: false,
      blocked: [...view.blocked, view.peerId],
    );
  }

  @override
  Future<void> disconnect() async {
    disconnects += 1;
    view = _copy(phase: 'idle', indicatorVisible: false);
  }

  @override
  Future<void> setLocale(String locale) async {
    view = _copy(locale: locale);
  }

  @override
  Future<void> setRelay(String relay) async {
    view = _copy(relay: relay);
  }

  @override
  Future<void> refreshAudio() async {
    view = _copy(audioInput: 'Mic', audioOutput: 'Speaker');
  }

  @override
  Future<void> addContact(String name, String deviceId) async {
    view = _copy(
      contacts: [
        ...view.contacts,
        DeskContact(name: name, deviceId: deviceId),
      ],
    );
  }

  @override
  Future<void> removeContact(String deviceId) async {
    view = _copy(
      contacts: view.contacts.where((row) => row.deviceId != deviceId).toList(),
    );
  }

  @override
  Future<void> stageFile(String name) async {
    _jail(name);
    view = _copy(staged: [...view.staged, name]);
  }

  @override
  Future<void> sendPointer(int x, int y, int buttons, int wheel) async {
    pointer = 'pointer $x $y $buttons $wheel';
    view = _copy(lastInput: 'pointer $x $y');
  }

  @override
  Future<void> sendKey(int keycode, bool down, int modifiers) async {
    keyEvent = 'key $keycode ${down ? 1 : 0} $modifiers';
    view = _copy(lastInput: 'key $keycode ${down ? 1 : 0}');
  }

  @override
  Future<void> sendFile(String name, List<int> bytes) async {
    _jail(name);
    fileBytes = bytes;
    view = _copy(files: [...view.files, name]);
  }

  @override
  Future<void> setClipboard(String text) async {
    view = _copy(clipboard: text);
  }

  @override
  Future<void> sendAudio(List<int> bytes) async {
    if (bytes.length > 4096) {
      throw DeskFailure('message:audio packet is too large');
    }
    view = _copy(audioPackets: view.audioPackets + 1);
  }

  @override
  Future<void> setMic(bool enabled) async {
    mic = enabled;
  }

  @override
  Future<void> submitFrame(int width, int height, List<int> rgba) async {
    view = _copy(
      frameWidth: width,
      frameHeight: height,
      frameRgba: rgba,
      frameIsInter: true,
    );
  }

  void _jail(String name) {
    if (name.contains('..') ||
        name.startsWith('/') ||
        name.contains('\\') ||
        name.contains(':')) {
      throw DeskFailure('message:refused');
    }
  }

  DeskView _copy({
    String? locale,
    String? phase,
    bool? indicatorVisible,
    bool? acceptEnabled,
    bool? fraudWarning,
    List<String>? granted,
    int? frameWidth,
    int? frameHeight,
    List<int>? frameRgba,
    bool? frameIsInter,
    List<String>? files,
    List<DeskContact>? contacts,
    List<String>? staged,
    String? audioInput,
    String? audioOutput,
    String? relay,
    List<String>? blocked,
    String? clipboard,
    int? audioPackets,
    String? lastInput,
  }) {
    return DeskView(
      deviceId: view.deviceId,
      shortFingerprint: view.shortFingerprint,
      shortCode: view.shortCode,
      shortCodeAuthenticates: view.shortCodeAuthenticates,
      ticketHex: view.ticketHex,
      locale: locale ?? view.locale,
      phase: phase ?? view.phase,
      indicatorVisible: indicatorVisible ?? view.indicatorVisible,
      indicatorCanBeDisabled: false,
      acceptEnabled: acceptEnabled ?? view.acceptEnabled,
      fraudWarning: fraudWarning ?? view.fraudWarning,
      firstContact: view.firstContact,
      peerId: view.peerId,
      fingerprint: view.fingerprint,
      granted: granted ?? view.granted,
      frameRgba: frameRgba ?? view.frameRgba,
      frameWidth: frameWidth ?? view.frameWidth,
      frameHeight: frameHeight ?? view.frameHeight,
      frameIsInter: frameIsInter ?? view.frameIsInter,
      files: files ?? view.files,
      contacts: contacts ?? view.contacts,
      staged: staged ?? view.staged,
      audioInput: audioInput ?? view.audioInput,
      audioOutput: audioOutput ?? view.audioOutput,
      relay: relay ?? view.relay,
      error: view.error,
      blocked: blocked ?? view.blocked,
      clipboard: clipboard ?? view.clipboard,
      audioPackets: audioPackets ?? view.audioPackets,
      lastInput: lastInput ?? view.lastInput,
    );
  }
}

/// A quiet home screen.
DeskView sampleView() {
  return const DeskView(
    deviceId: 'abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz',
    shortFingerprint: '1-2-3-4',
    shortCode: '100-elma-armut',
    shortCodeAuthenticates: true,
    ticketHex: 'abcd',
    locale: 'tr',
    phase: 'idle',
    indicatorVisible: false,
    indicatorCanBeDisabled: false,
    acceptEnabled: false,
    fraudWarning: false,
    firstContact: false,
    peerId: '',
    fingerprint: '',
    granted: [],
    frameRgba: [],
    frameWidth: 0,
    frameHeight: 0,
    frameIsInter: false,
    files: [],
    contacts: [],
    staged: [],
    audioInput: 'unavailable',
    audioOutput: 'unavailable',
    relay: '',
    error: '',
    blocked: [],
    clipboard: '',
    audioPackets: 0,
    lastInput: '',
  );
}

/// A first-contact prompt. Accept stays off until the test says otherwise.
DeskView awaitingView({bool acceptEnabled = false}) {
  final base = sampleView();
  return DeskView(
    deviceId: base.deviceId,
    shortFingerprint: base.shortFingerprint,
    shortCode: base.shortCode,
    shortCodeAuthenticates: true,
    ticketHex: base.ticketHex,
    locale: 'tr',
    phase: 'awaiting',
    indicatorVisible: true,
    indicatorCanBeDisabled: false,
    acceptEnabled: acceptEnabled,
    fraudWarning: true,
    firstContact: true,
    peerId: 'peerpeerpeerpeerpeerpeerpeerpeerpeerpeerpeerpe',
    fingerprint: '9-9-9-9',
    granted: const [],
    frameRgba: const [],
    frameWidth: 0,
    frameHeight: 0,
    frameIsInter: false,
    files: const [],
    contacts: const [],
    staged: const [],
    audioInput: 'unavailable',
    audioOutput: 'unavailable',
    relay: '',
    error: '',
    blocked: const [],
    clipboard: '',
    audioPackets: 0,
    lastInput: '',
  );
}
