import 'dart:io';

import 'package:flutter/widgets.dart';

import 'rust_desk.dart';
import 'shell.dart';
import 'src/rust/api.dart';
import 'src/rust/frb_generated.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final desktop = await Desktop.open(dir: _dataDir());
  runApp(CbDeskApp(api: RustDeskApi(desktop)));
}

String _dataDir() {
  final home =
      Platform.environment['HOME'] ??
      Platform.environment['USERPROFILE'] ??
      Directory.current.path;
  return '$home/.local/share/cbdesk';
}
