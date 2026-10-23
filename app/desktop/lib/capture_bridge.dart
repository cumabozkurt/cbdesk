import 'package:flutter/services.dart';

import 'desk_api.dart';

const _channel = MethodChannel('org.cbdesk/capture');

/// Android screen share. The system dialog is the consent.
///
/// [start] returns true only after that dialog is accepted and the capture
/// service has a projection. A denial returns false and does not read pixels.
class ScreenShare {
  /// [api] receives frames after consent.
  ScreenShare(this._api);

  final DeskApi _api;
  var _sending = false;

  /// Listen for frames. Call this before [start].
  void listen() {
    _channel.setMethodCallHandler((call) async {
      if (call.method != 'frame') {
        return;
      }
      final args = Map<Object?, Object?>.from(call.arguments as Map);
      final width = args['width'] as int;
      final height = args['height'] as int;
      final rgba = args['rgba'];
      if (rgba is! Uint8List || _sending) {
        return;
      }
      _sending = true;
      try {
        await _api.submitFrame(width, height, rgba);
      } on DeskFailure {
        // A refused size leaves the previous frame in place.
      } finally {
        _sending = false;
      }
    });
  }

  /// Open the system capture dialog. False means the person declined.
  Future<bool> start() async {
    final ok = await _channel.invokeMethod<bool>('start');
    return ok ?? false;
  }

  /// Stop the projection. Safe when capture was never accepted.
  Future<void> stop() async {
    await _channel.invokeMethod<void>('stop');
  }

  /// App files directory. The desk stores its identity here.
  static Future<String?> filesDir() {
    return _channel.invokeMethod<String>('filesDir');
  }

  /// System microphone permission. False leaves the microphone closed.
  static Future<bool> requestMic() async {
    final ok = await _channel.invokeMethod<bool>('requestMic');
    return ok ?? false;
  }
}
