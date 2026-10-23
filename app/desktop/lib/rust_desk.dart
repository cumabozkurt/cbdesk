import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

import 'desk_api.dart';
import 'src/rust/api.dart';

/// The running shell's core. Widget tests do not construct this.
class RustDeskApi implements DeskApi {
  /// [desktop] is the opaque handle from [Desktop.open].
  RustDeskApi(this._desktop);

  final Desktop _desktop;

  @override
  Future<DeskView> snapshot() {
    return _guard(() async {
      final snap = await _desktop.snapshot();
      return DeskView(
        deviceId: snap.deviceId,
        shortFingerprint: snap.shortFingerprint,
        shortCode: snap.shortCode,
        shortCodeAuthenticates: snap.shortCodeAuthenticates,
        ticketHex: snap.ticketHex,
        locale: snap.locale,
        phase: snap.phase,
        indicatorVisible: snap.indicatorVisible,
        indicatorCanBeDisabled: snap.indicatorCanBeDisabled,
        acceptEnabled: snap.acceptEnabled,
        fraudWarning: snap.fraudWarning,
        firstContact: snap.firstContact,
        peerId: snap.peerId,
        fingerprint: snap.fingerprint,
        granted: snap.granted,
        frameRgba: snap.frameRgba,
        frameWidth: snap.frameWidth,
        frameHeight: snap.frameHeight,
        frameIsInter: snap.frameIsInter,
        files: snap.files,
        contacts: [
          for (final row in snap.contacts)
            DeskContact(name: row.name, deviceId: row.deviceId),
        ],
        staged: snap.staged,
        audioInput: snap.audioInput,
        audioOutput: snap.audioOutput,
        relay: snap.relay,
        error: snap.error,
        blocked: snap.blocked,
      );
    });
  }

  @override
  Future<List<PermissionChoice>> permissions() {
    return _guard(() async {
      final rows = await permissionOffers();
      return [
        for (final row in rows)
          PermissionChoice(name: row.name, highRisk: row.highRisk),
      ];
    });
  }

  @override
  Future<void> connect(String text) =>
      _guard(() => _desktop.connect(text: text));

  @override
  Future<void> approve(List<String> perms, bool explicitHighRisk) {
    return _guard(
      () => _desktop.approve(perms: perms, explicitHighRisk: explicitHighRisk),
    );
  }

  @override
  Future<void> reject() => _guard(_desktop.reject);

  @override
  Future<void> block() => _guard(_desktop.block);

  @override
  Future<void> disconnect() => _guard(_desktop.disconnect);

  @override
  Future<void> setLocale(String locale) =>
      _guard(() => _desktop.setLocale(locale: locale));

  @override
  Future<void> setRelay(String relay) =>
      _guard(() => _desktop.setRelay(relay: relay));

  @override
  Future<void> refreshAudio() => _guard(_desktop.refreshAudio);

  @override
  Future<void> addContact(String name, String deviceId) {
    return _guard(() => _desktop.addContact(name: name, deviceId: deviceId));
  }

  @override
  Future<void> removeContact(String deviceId) {
    return _guard(() => _desktop.removeContact(deviceId: deviceId));
  }

  @override
  Future<void> stageFile(String name) =>
      _guard(() => _desktop.stageFile(name: name));
}

Future<T> _guard<T>(Future<T> Function() body) async {
  try {
    return await body();
  } on AnyhowException catch (err) {
    throw DeskFailure(err.message);
  } catch (err) {
    throw DeskFailure('message:$err');
  }
}
