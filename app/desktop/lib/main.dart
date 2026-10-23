import 'dart:io';

import 'package:flutter/widgets.dart';

import 'bridge_desk.dart';
import 'capture_bridge.dart';
import 'shell.dart';
import 'src/rust/api.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final desktop = await Desktop.open(dir: await _dataDir());
  final api = NativeDesk(desktop);
  Future<bool> Function()? share;
  Future<bool> Function()? mic;
  if (Platform.isAndroid) {
    final screen = ScreenShare(api);
    screen.listen();
    share = screen.start;
    mic = ScreenShare.requestMic;
  }
  runApp(CbDeskApp(api: api, shareScreen: share, ensureMic: mic));
}

Future<String> _dataDir() async {
  if (Platform.isAndroid) {
    final dir = await ScreenShare.filesDir();
    if (dir != null && dir.isNotEmpty) {
      return dir;
    }
  }
  final home =
      Platform.environment['HOME'] ??
      Platform.environment['USERPROFILE'] ??
      Directory.current.path;
  return '$home/.local/share/cbdesk';
}
