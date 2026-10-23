import 'package:flutter_rust_bridge/flutter_rust_bridge.dart';

import 'desk_api.dart';
import 'src/rust/api.dart';

/// The running shell's core. Widget tests do not construct this.
class NativeDesk implements DeskApi {
  /// [desktop] is the opaque handle from [Desktop.open].
  NativeDesk(this._desktop);

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
        clipboard: snap.clipboard,
        audioPackets: snap.audioPackets,
        lastInput: snap.lastInput,
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

  @override
  Future<void> sendPointer(int x, int y, int buttons, int wheel) {
    return _guard(
      () => _desktop.sendPointer(x: x, y: y, buttons: buttons, wheel: wheel),
    );
  }

  @override
  Future<void> sendKey(int keycode, bool down, int modifiers) {
    return _guard(
      () => _desktop.sendKey(keycode: keycode, down: down, modifiers: modifiers),
    );
  }

  @override
  Future<void> sendFile(String name, List<int> bytes) {
    return _guard(() => _desktop.sendFile(name: name, bytes: bytes));
  }

  @override
  Future<void> setClipboard(String text) =>
      _guard(() => _desktop.setClipboard(text: text));

  @override
  Future<void> sendAudio(List<int> bytes) =>
      _guard(() => _desktop.sendAudio(bytes: bytes));

  @override
  Future<void> setMic(bool enabled) =>
      _guard(() => _desktop.setMic(enabled: enabled));

  @override
  Future<void> submitFrame(int width, int height, List<int> rgba) {
    return _guard(
      () => _desktop.submitFrame(width: width, height: height, rgba: rgba),
    );
  }
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
