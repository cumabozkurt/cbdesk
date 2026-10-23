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
    if (text.contains('-') && !text.startsWith('aa')) {
      throw DeskFailure('short_code');
    }
    connected = text;
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
    if (name.contains('..') ||
        name.startsWith('/') ||
        name.contains('\\') ||
        name.contains(':')) {
      throw DeskFailure('message:refused');
    }
    view = _copy(staged: [...view.staged, name]);
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
    List<DeskContact>? contacts,
    List<String>? staged,
    String? audioInput,
    String? audioOutput,
    String? relay,
    List<String>? blocked,
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
      files: view.files,
      contacts: contacts ?? view.contacts,
      staged: staged ?? view.staged,
      audioInput: audioInput ?? view.audioInput,
      audioOutput: audioOutput ?? view.audioOutput,
      relay: relay ?? view.relay,
      error: view.error,
      blocked: blocked ?? view.blocked,
    );
  }
}

/// A quiet home screen.
DeskView sampleView() {
  return const DeskView(
    deviceId: 'abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz',
    shortFingerprint: '1-2-3-4',
    shortCode: '100-elma-armut',
    shortCodeAuthenticates: false,
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
  );
}

/// A first-contact prompt. Accept stays off until the test says otherwise.
DeskView awaitingView({bool acceptEnabled = false}) {
  final base = sampleView();
  return DeskView(
    deviceId: base.deviceId,
    shortFingerprint: base.shortFingerprint,
    shortCode: base.shortCode,
    shortCodeAuthenticates: false,
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
  );
}
