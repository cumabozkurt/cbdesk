import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'desk_api.dart';
import 'l10n/app_localizations.dart';

/// A text field that keeps its name when the hint is hidden.
InputDecoration _labeled(String text) {
  return InputDecoration(labelText: text, hintText: text);
}

/// Desktop shell. [poll] is null in tests so no timer stays armed.
class CbDeskApp extends StatefulWidget {
  /// [api] is the Rust bridge, or a fake in tests.
  const CbDeskApp({
    super.key,
    required this.api,
    this.poll = const Duration(seconds: 1),
    this.shareScreen,
    this.ensureMic,
  });

  /// Core the screens call.
  final DeskApi api;

  /// How often to re-read the snapshot. Null disables the timer.
  final Duration? poll;

  /// Android system capture dialog. Null on desktop, where the core captures.
  final Future<bool> Function()? shareScreen;

  /// Android microphone permission. Null where the core opens the device.
  final Future<bool> Function()? ensureMic;

  @override
  State<CbDeskApp> createState() => _CbDeskAppState();
}

class _CbDeskAppState extends State<CbDeskApp> {
  DeskView? _view;
  List<PermissionChoice> _offers = const [];
  String? _actionError;
  int _section = 0;
  Timer? _timer;
  final Set<String> _ticked = {};
  bool _explicitHighRisk = false;
  final _connect = TextEditingController();
  final _code = TextEditingController();
  final _fileName = TextEditingController();
  final _fileBody = TextEditingController();
  final _clipboard = TextEditingController();
  final _contactName = TextEditingController();
  final _contactId = TextEditingController();
  final _relay = TextEditingController();
  final _remoteFocus = FocusNode();
  bool _mic = false;

  @override
  void initState() {
    super.initState();
    unawaited(_reload());
    final poll = widget.poll;
    if (poll != null) {
      _timer = Timer.periodic(poll, (_) => unawaited(_reload()));
    }
  }

  @override
  void dispose() {
    _timer?.cancel();
    _connect.dispose();
    _code.dispose();
    _fileName.dispose();
    _fileBody.dispose();
    _clipboard.dispose();
    _contactName.dispose();
    _contactId.dispose();
    _relay.dispose();
    _remoteFocus.dispose();
    super.dispose();
  }

  Future<void> _reload() async {
    try {
      final view = await widget.api.snapshot();
      final offers = await widget.api.permissions();
      if (!mounted) {
        return;
      }
      setState(() {
        _view = view;
        _offers = offers;
        if (_relay.text.isEmpty) {
          _relay.text = view.relay;
        }
      });
    } on DeskFailure catch (err) {
      if (!mounted) {
        return;
      }
      setState(() => _actionError = err.code);
    }
  }

  Future<void> _run(Future<void> Function() body, {int? section}) async {
    try {
      await body();
      if (section != null) {
        _section = section;
      }
      _actionError = null;
    } on DeskFailure catch (err) {
      _actionError = err.code;
    }
    await _reload();
  }

  @override
  Widget build(BuildContext context) {
    final view = _view;
    return MaterialApp(
      onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
      locale: Locale(view?.locale ?? 'tr'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) {
        return Stack(
          children: [
            child ?? const SizedBox.shrink(),
            if (view != null && view.indicatorVisible) ...[
              const Positioned.fill(
                child: ExcludeSemantics(
                  child: IgnorePointer(child: _SessionBorder()),
                ),
              ),
              Positioned(
                top: 12,
                right: 12,
                child: Semantics(
                  key: const Key('session-indicator-live'),
                  container: true,
                  liveRegion: true,
                  label: AppLocalizations.of(context).indicatorTitle,
                  child: _Indicator(
                    view: view,
                    onDisconnect: () => unawaited(_run(widget.api.disconnect)),
                  ),
                ),
              ),
            ],
          ],
        );
      },
      home: view == null
          ? const _Loading()
          : _Home(
              view: view,
              offers: _offers,
              section: _section,
              actionError: _actionError,
              ticked: _ticked,
              explicitHighRisk: _explicitHighRisk,
              connect: _connect,
              code: _code,
              fileName: _fileName,
              fileBody: _fileBody,
              clipboard: _clipboard,
              mic: _mic,
              contactName: _contactName,
              contactId: _contactId,
              relay: _relay,
              remoteFocus: _remoteFocus,
              onSection: (index) => setState(() => _section = index),
              onToggle: (name, value) => setState(() {
                if (value) {
                  _ticked.add(name);
                } else {
                  _ticked.remove(name);
                }
              }),
              onExplicit: (value) => setState(() => _explicitHighRisk = value),
              onConnect: () => unawaited(
                _run(
                  () => widget.api.connect(
                    _join(_connect.text, _code.text),
                  ),
                  section: 2,
                ),
              ),
              onApprove: () => unawaited(
                _run(
                  () => widget.api.approve(_ticked.toList(), _explicitHighRisk),
                ),
              ),
              onReject: () => unawaited(_run(widget.api.reject)),
              onBlock: () => unawaited(_run(widget.api.block)),
              onStage: () => unawaited(
                _run(() => widget.api.stageFile(_fileName.text.trim())),
              ),
              onSendFile: () => unawaited(
                _run(
                  () => widget.api.sendFile(
                    _fileName.text.trim(),
                    utf8.encode(_fileBody.text),
                  ),
                ),
              ),
              onPointer: (x, y, buttons, wheel) => unawaited(
                _run(() => widget.api.sendPointer(x, y, buttons, wheel)),
              ),
              onKey: (keycode, down) => unawaited(
                _run(
                  () => widget.api.sendKey(
                    keycode,
                    down,
                    _modifiers(HardwareKeyboard.instance),
                  ),
                ),
              ),
              onClipboard: () => unawaited(
                _run(() => widget.api.setClipboard(_clipboard.text)),
              ),
              onMic: (enabled) => unawaited(_setMic(enabled)),
              onShare: widget.shareScreen == null
                  ? null
                  : () => unawaited(_share()),
              onAddContact: () => unawaited(
                _run(
                  () => widget.api.addContact(
                    _contactName.text.trim(),
                    _contactId.text.trim(),
                  ),
                ),
              ),
              onRemoveContact: (id) =>
                  unawaited(_run(() => widget.api.removeContact(id))),
              onLocale: (locale) =>
                  unawaited(_run(() => widget.api.setLocale(locale))),
              onSaveRelay: () => unawaited(_saveRelay()),
              onRefreshAudio: () => unawaited(_run(widget.api.refreshAudio)),
            ),
    );
  }

  Future<void> _setMic(bool enabled) async {
    if (enabled) {
      final ensure = widget.ensureMic;
      if (ensure != null && !await ensure()) {
        if (!mounted) {
          return;
        }
        setState(() => _actionError = 'mic_denied');
        return;
      }
    }
    await _run(() => widget.api.setMic(enabled));
    if (!mounted) {
      return;
    }
    if (_actionError == null) {
      setState(() => _mic = enabled);
    }
  }

  Future<void> _share() async {
    final share = widget.shareScreen;
    if (share == null) {
      return;
    }
    final accepted = await share();
    if (!mounted) {
      return;
    }
    setState(() => _actionError = accepted ? null : 'share_denied');
  }

  Future<void> _saveRelay() async {
    final text = _relay.text.trim();
    if (!_relayOk(text)) {
      setState(() => _actionError = 'relay');
      return;
    }
    await _run(() => widget.api.setRelay(text));
  }
}

String _join(String ticket, String code) {
  final trimmedTicket = ticket.trim();
  final trimmedCode = code.trim();
  if (trimmedTicket.isEmpty) {
    return trimmedCode;
  }
  if (trimmedCode.isEmpty) {
    return trimmedTicket;
  }
  return '$trimmedTicket $trimmedCode';
}

int _modifiers(HardwareKeyboard keyboard) {
  var mods = 0;
  if (keyboard.isShiftPressed) {
    mods |= 1;
  }
  if (keyboard.isControlPressed) {
    mods |= 2;
  }
  if (keyboard.isAltPressed) {
    mods |= 4;
  }
  if (keyboard.isMetaPressed) {
    mods |= 8;
  }
  return mods;
}

int? _keycode(LogicalKeyboardKey key) {
  if (key == LogicalKeyboardKey.enter || key == LogicalKeyboardKey.numpadEnter) {
    return 0xE000;
  }
  if (key == LogicalKeyboardKey.escape) {
    return 0xE001;
  }
  if (key == LogicalKeyboardKey.backspace) {
    return 0xE002;
  }
  if (key == LogicalKeyboardKey.tab) {
    return 0xE003;
  }
  if (key == LogicalKeyboardKey.arrowLeft) {
    return 0xE004;
  }
  if (key == LogicalKeyboardKey.arrowRight) {
    return 0xE005;
  }
  if (key == LogicalKeyboardKey.arrowUp) {
    return 0xE006;
  }
  if (key == LogicalKeyboardKey.arrowDown) {
    return 0xE007;
  }
  final label = key.keyLabel;
  if (label.length == 1) {
    final code = label.codeUnitAt(0);
    if (code >= 32 && code < 127) {
      return code;
    }
  }
  return null;
}

bool _relayOk(String text) {
  return text.isEmpty ||
      text.startsWith('https://') ||
      text.startsWith('relay://');
}

class _Loading extends StatelessWidget {
  const _Loading();

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: Center(child: Text(AppLocalizations.of(context).loading)),
    );
  }
}

class _SessionBorder extends StatelessWidget {
  const _SessionBorder();

  @override
  Widget build(BuildContext context) {
    return const DecoratedBox(
      decoration: BoxDecoration(
        border: Border.fromBorderSide(
          BorderSide(color: Color(0xFFE65100), width: 8),
        ),
      ),
    );
  }
}

class _Indicator extends StatelessWidget {
  const _Indicator({required this.view, required this.onDisconnect});

  final DeskView view;
  final VoidCallback onDisconnect;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return Material(
      key: const Key('session-indicator'),
      elevation: 6,
      color: const Color(0xFFFFF3E0),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              l10n.indicatorTitle,
              style: Theme.of(context).textTheme.titleMedium,
            ),
            Text(l10n.indicatorCannotDisable),
            Text(l10n.phaseLabel(view.phase)),
            if (view.peerId.isNotEmpty) Text(l10n.peerLabel(view.fingerprint)),
            const SizedBox(height: 8),
            FilledButton(
              key: const Key('disconnect'),
              onPressed: onDisconnect,
              child: Text(l10n.disconnect),
            ),
          ],
        ),
      ),
    );
  }
}

class _Home extends StatelessWidget {
  const _Home({
    required this.view,
    required this.offers,
    required this.section,
    required this.actionError,
    required this.ticked,
    required this.explicitHighRisk,
    required this.connect,
    required this.code,
    required this.fileName,
    required this.fileBody,
    required this.clipboard,
    required this.mic,
    required this.contactName,
    required this.contactId,
    required this.relay,
    required this.onSection,
    required this.onToggle,
    required this.onExplicit,
    required this.onConnect,
    required this.onApprove,
    required this.onReject,
    required this.onBlock,
    required this.onStage,
    required this.onSendFile,
    required this.onPointer,
    required this.onKey,
    required this.onClipboard,
    required this.onMic,
    required this.onShare,
    required this.onAddContact,
    required this.onRemoveContact,
    required this.onLocale,
    required this.onSaveRelay,
    required this.onRefreshAudio,
    required this.remoteFocus,
  });

  final DeskView view;
  final List<PermissionChoice> offers;
  final int section;
  final String? actionError;
  final Set<String> ticked;
  final bool explicitHighRisk;
  final TextEditingController connect;
  final TextEditingController code;
  final TextEditingController fileName;
  final TextEditingController fileBody;
  final TextEditingController clipboard;
  final bool mic;
  final TextEditingController contactName;
  final TextEditingController contactId;
  final TextEditingController relay;
  final FocusNode remoteFocus;
  final ValueChanged<int> onSection;
  final void Function(String name, bool value) onToggle;
  final ValueChanged<bool> onExplicit;
  final VoidCallback onConnect;
  final VoidCallback onApprove;
  final VoidCallback onReject;
  final VoidCallback onBlock;
  final VoidCallback onStage;
  final VoidCallback onSendFile;
  final void Function(int x, int y, int buttons, int wheel) onPointer;
  final void Function(int keycode, bool down) onKey;
  final VoidCallback onClipboard;
  final ValueChanged<bool> onMic;
  final VoidCallback? onShare;
  final VoidCallback onAddContact;
  final ValueChanged<String> onRemoveContact;
  final ValueChanged<String> onLocale;
  final VoidCallback onSaveRelay;
  final VoidCallback onRefreshAudio;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final pages = [
      _DevicePage(view: view, onShare: onShare),
      _ConnectPage(ticket: connect, code: code, onConnect: onConnect),
      _RemotePage(
        view: view,
        focusNode: remoteFocus,
        clipboard: clipboard,
        mic: mic,
        onPointer: onPointer,
        onKey: onKey,
        onClipboard: onClipboard,
        onMic: onMic,
      ),
      _FilesPage(
        view: view,
        name: fileName,
        body: fileBody,
        onStage: onStage,
        onSend: onSendFile,
      ),
      _AddressPage(
        view: view,
        name: contactName,
        deviceId: contactId,
        onAdd: onAddContact,
        onRemove: onRemoveContact,
      ),
      _SettingsPage(
        view: view,
        relay: relay,
        onLocale: onLocale,
        onSave: onSaveRelay,
        onRefresh: onRefreshAudio,
      ),
    ];
    return Scaffold(
      body: Row(
        children: [
          NavigationRail(
            selectedIndex: section,
            onDestinationSelected: onSection,
            labelType: NavigationRailLabelType.all,
            destinations: [
              NavigationRailDestination(
                icon: Icon(
                  Icons.home_outlined,
                  key: const Key('nav-home'),
                  semanticLabel: l10n.navHome,
                ),
                label: Text(l10n.navHome),
              ),
              NavigationRailDestination(
                icon: Icon(
                  Icons.link,
                  key: const Key('nav-connect'),
                  semanticLabel: l10n.navConnect,
                ),
                label: Text(l10n.navConnect),
              ),
              NavigationRailDestination(
                icon: Icon(
                  Icons.desktop_windows_outlined,
                  key: const Key('nav-remote'),
                  semanticLabel: l10n.navRemote,
                ),
                label: Text(l10n.navRemote),
              ),
              NavigationRailDestination(
                icon: Icon(
                  Icons.folder_outlined,
                  key: const Key('nav-files'),
                  semanticLabel: l10n.navFiles,
                ),
                label: Text(l10n.navFiles),
              ),
              NavigationRailDestination(
                icon: Icon(
                  Icons.contacts_outlined,
                  key: const Key('nav-address'),
                  semanticLabel: l10n.navAddress,
                ),
                label: Text(l10n.navAddress),
              ),
              NavigationRailDestination(
                icon: Icon(
                  Icons.settings_outlined,
                  key: const Key('nav-settings'),
                  semanticLabel: l10n.navSettings,
                ),
                label: Text(l10n.navSettings),
              ),
            ],
          ),
          const VerticalDivider(width: 1),
          Expanded(
            child: Stack(
              children: [
                pages[section],
                if (view.phase == 'awaiting')
                  Center(
                    child: _Consent(
                      view: view,
                      offers: offers,
                      ticked: ticked,
                      explicitHighRisk: explicitHighRisk,
                      onToggle: onToggle,
                      onExplicit: onExplicit,
                      onApprove: onApprove,
                      onReject: onReject,
                      onBlock: onBlock,
                    ),
                  ),
                if (actionError != null)
                  Align(
                    alignment: Alignment.bottomCenter,
                    child: Material(
                      key: const Key('error'),
                      color: Theme.of(context).colorScheme.errorContainer,
                      child: Padding(
                        padding: const EdgeInsets.all(12),
                        child: Text(_errorText(l10n, actionError!)),
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

String _errorText(AppLocalizations l10n, String code) {
  switch (code) {
    case 'short_code':
    case 'short_code_needs_ticket':
      return l10n.errorShortCode;
    case 'mic_denied':
      return l10n.micDenied;
    case 'share_denied':
      return l10n.shareDenied;
    case 'blocked':
      return l10n.errorBlocked;
    case 'no_prompt':
      return l10n.errorNoPrompt;
    case 'loose_seed':
      return l10n.errorLooseSeed;
    case 'relay':
      return l10n.relayInvalid;
    default:
      final detail = code.startsWith('message:')
          ? code.substring('message:'.length)
          : code;
      return l10n.errorMessage(detail);
  }
}

class _DevicePage extends StatelessWidget {
  const _DevicePage({required this.view, required this.onShare});

  final DeskView view;
  final VoidCallback? onShare;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          l10n.thisDevice,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        const SizedBox(height: 16),
        Text(l10n.deviceId),
        SelectableText(view.deviceId, key: const Key('device-id')),
        const SizedBox(height: 12),
        Text(l10n.shortFingerprint),
        SelectableText(view.shortFingerprint, key: const Key('fingerprint')),
        const SizedBox(height: 12),
        Text(l10n.shortCode),
        SelectableText(view.shortCode, key: const Key('short-code')),
        const SizedBox(height: 8),
        Text(
          l10n.shortCodeDoesNotAuthenticate,
          key: const Key('short-code-warning'),
        ),
        const SizedBox(height: 16),
        Text(l10n.ticketLocator),
        SelectableText(view.ticketHex, key: const Key('ticket')),
        Text(l10n.ticketIsNotAGrant),
        const SizedBox(height: 12),
        Text(l10n.phaseLabel(view.phase)),
        const SizedBox(height: 16),
        if (onShare == null)
          Text(l10n.desktopCaptureNote, key: const Key('desktop-capture'))
        else
          FilledButton(
            key: const Key('share-screen'),
            onPressed: onShare,
            child: Text(l10n.shareScreen),
          ),
      ],
    );
  }
}

class _ConnectPage extends StatelessWidget {
  const _ConnectPage({
    required this.ticket,
    required this.code,
    required this.onConnect,
  });

  final TextEditingController ticket;
  final TextEditingController code;
  final VoidCallback onConnect;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          l10n.connectTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        const SizedBox(height: 16),
        TextField(
          key: const Key('connect-field'),
          controller: ticket,
          decoration: _labeled(l10n.connectHint),
        ),
        const SizedBox(height: 12),
        TextField(
          key: const Key('connect-code'),
          controller: code,
          decoration: _labeled(l10n.connectCodeHint),
        ),
        const SizedBox(height: 12),
        FilledButton(
          key: const Key('connect-submit'),
          onPressed: onConnect,
          child: Text(l10n.connectButton),
        ),
      ],
    );
  }
}

class _RemotePage extends StatelessWidget {
  const _RemotePage({
    required this.view,
    required this.focusNode,
    required this.clipboard,
    required this.mic,
    required this.onPointer,
    required this.onKey,
    required this.onClipboard,
    required this.onMic,
  });

  final DeskView view;
  final FocusNode focusNode;
  final TextEditingController clipboard;
  final bool mic;
  final void Function(int x, int y, int buttons, int wheel) onPointer;
  final void Function(int keycode, bool down) onKey;
  final VoidCallback onClipboard;
  final ValueChanged<bool> onMic;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final hasFrame =
        view.frameWidth > 0 &&
        view.frameHeight > 0 &&
        view.frameRgba.isNotEmpty;
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          l10n.remoteTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        const SizedBox(height: 8),
        Text(l10n.remoteHonest),
        const SizedBox(height: 12),
        if (!hasFrame)
          Text(l10n.remoteEmpty, key: const Key('remote-empty'))
        else ...[
          Text(
            view.frameIsInter ? l10n.interFrame : l10n.keyFrame,
            key: const Key('frame-kind'),
          ),
          const SizedBox(height: 8),
          Text(l10n.pointerHint),
          const SizedBox(height: 8),
          Semantics(
            label: l10n.remoteHonest,
            image: true,
            button: true,
            child: KeyboardListener(
              focusNode: focusNode,
              onKeyEvent: (event) {
                final code = _keycode(event.logicalKey);
                if (code == null) {
                  return;
                }
                final down = event is KeyDownEvent;
                if (event is! KeyDownEvent && event is! KeyUpEvent) {
                  return;
                }
                onKey(code, down);
              },
              child: GestureDetector(
                key: const Key('remote-frame'),
                onTapDown: (details) => _at(details.localPosition, 1),
                onTapUp: (details) => _at(details.localPosition, 0),
                onSecondaryTapDown: (details) => _at(details.localPosition, 2),
                onPanUpdate: (details) => _at(details.localPosition, 1),
                child: SizedBox(
                  width: view.frameWidth * 4,
                  height: view.frameHeight * 4,
                  child: CustomPaint(
                    painter: _RgbaPainter(
                      view.frameRgba,
                      view.frameWidth,
                      view.frameHeight,
                    ),
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(height: 8),
          OutlinedButton(
            key: const Key('send-enter'),
            onPressed: () {
              onKey(0xE000, true);
              onKey(0xE000, false);
            },
            child: Text(l10n.sendEnter),
          ),
        ],
        const SizedBox(height: 16),
        Text(l10n.lastInputLabel(view.lastInput), key: const Key('last-input')),
        const SizedBox(height: 12),
        TextField(
          key: const Key('clipboard-field'),
          controller: clipboard,
          decoration: _labeled(l10n.clipboardHint),
        ),
        const SizedBox(height: 8),
        FilledButton(
          key: const Key('clipboard-set'),
          onPressed: onClipboard,
          child: Text(l10n.setClipboard),
        ),
        Text(l10n.clipboardLabel(view.clipboard), key: const Key('clipboard-value')),
        const SizedBox(height: 12),
        SwitchListTile(
          key: const Key('mic'),
          value: mic,
          onChanged: onMic,
          title: Text(l10n.micLabel),
          subtitle: Text(l10n.micHonest),
        ),
        Text(
          l10n.audioPacketsLabel(view.audioPackets),
          key: const Key('audio-packets'),
        ),
      ],
    );
  }

  void _at(Offset local, int buttons) {
    final width = view.frameWidth;
    final height = view.frameHeight;
    if (width <= 0 || height <= 0) {
      return;
    }
    final x = (local.dx / 4).floor().clamp(0, width - 1);
    final y = (local.dy / 4).floor().clamp(0, height - 1);
    onPointer(x, y, buttons, 0);
  }
}

class _RgbaPainter extends CustomPainter {
  _RgbaPainter(this.rgba, this.width, this.height);

  final List<int> rgba;
  final int width;
  final int height;

  @override
  void paint(Canvas canvas, Size size) {
    if (width <= 0 || height <= 0) {
      return;
    }
    final scaleX = size.width / width;
    final scaleY = size.height / height;
    final paint = Paint();
    for (var y = 0; y < height; y++) {
      for (var x = 0; x < width; x++) {
        final index = (y * width + x) * 4;
        if (index + 3 >= rgba.length) {
          return;
        }
        paint.color = Color.fromARGB(
          rgba[index + 3],
          rgba[index],
          rgba[index + 1],
          rgba[index + 2],
        );
        canvas.drawRect(
          Rect.fromLTWH(x * scaleX, y * scaleY, scaleX + 0.5, scaleY + 0.5),
          paint,
        );
      }
    }
  }

  @override
  bool shouldRepaint(_RgbaPainter oldDelegate) {
    return oldDelegate.rgba != rgba ||
        oldDelegate.width != width ||
        oldDelegate.height != height;
  }
}

class _FilesPage extends StatelessWidget {
  const _FilesPage({
    required this.view,
    required this.name,
    required this.body,
    required this.onStage,
    required this.onSend,
  });

  final DeskView view;
  final TextEditingController name;
  final TextEditingController body;
  final VoidCallback onStage;
  final VoidCallback onSend;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          l10n.filesTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        const SizedBox(height: 12),
        TextField(
          key: const Key('file-name'),
          controller: name,
          decoration: _labeled(l10n.fileHint),
        ),
        const SizedBox(height: 8),
        FilledButton(
          key: const Key('file-stage'),
          onPressed: onStage,
          child: Text(l10n.stageFile),
        ),
        const SizedBox(height: 8),
        TextField(
          key: const Key('file-body'),
          controller: body,
          decoration: _labeled(l10n.fileBodyHint),
        ),
        const SizedBox(height: 8),
        FilledButton(
          key: const Key('file-send'),
          onPressed: onSend,
          child: Text(l10n.sendFile),
        ),
        const SizedBox(height: 16),
        Text(l10n.staged),
        for (final staged in view.staged)
          Text(staged, key: Key('staged-$staged')),
        const SizedBox(height: 12),
        Text(l10n.jailFiles),
        for (final file in view.files) Text(file),
      ],
    );
  }
}

class _AddressPage extends StatelessWidget {
  const _AddressPage({
    required this.view,
    required this.name,
    required this.deviceId,
    required this.onAdd,
    required this.onRemove,
  });

  final DeskView view;
  final TextEditingController name;
  final TextEditingController deviceId;
  final VoidCallback onAdd;
  final ValueChanged<String> onRemove;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          l10n.addressTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        TextField(
          key: const Key('contact-name'),
          controller: name,
          decoration: _labeled(l10n.contactName),
        ),
        TextField(
          key: const Key('contact-id'),
          controller: deviceId,
          decoration: _labeled(l10n.contactId),
        ),
        const SizedBox(height: 8),
        FilledButton(
          key: const Key('contact-add'),
          onPressed: onAdd,
          child: Text(l10n.addContact),
        ),
        for (final contact in view.contacts)
          ListTile(
            title: Text(contact.name, key: Key('contact-${contact.deviceId}')),
            subtitle: Text(contact.deviceId),
            trailing: IconButton(
              key: Key('remove-${contact.deviceId}'),
              onPressed: () => onRemove(contact.deviceId),
              icon: const Icon(Icons.delete_outline),
              tooltip: l10n.remove,
            ),
          ),
      ],
    );
  }
}

class _SettingsPage extends StatelessWidget {
  const _SettingsPage({
    required this.view,
    required this.relay,
    required this.onLocale,
    required this.onSave,
    required this.onRefresh,
  });

  final DeskView view;
  final TextEditingController relay;
  final ValueChanged<String> onLocale;
  final VoidCallback onSave;
  final VoidCallback onRefresh;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text(
          l10n.settingsTitle,
          style: Theme.of(context).textTheme.headlineMedium,
        ),
        const SizedBox(height: 8),
        Text(l10n.indicatorCannotDisable, key: const Key('indicator-locked')),
        const SizedBox(height: 12),
        Text(l10n.localeLabel),
        SegmentedButton<String>(
          key: const Key('locale'),
          segments: const [
            ButtonSegment(value: 'tr', label: Text('Türkçe')),
            ButtonSegment(value: 'en', label: Text('English')),
          ],
          selected: {view.locale},
          onSelectionChanged: (selected) => onLocale(selected.first),
        ),
        const SizedBox(height: 16),
        Text(l10n.relayLabel),
        TextField(
          key: const Key('relay'),
          controller: relay,
          decoration: _labeled(l10n.relayHint),
        ),
        const SizedBox(height: 8),
        FilledButton(
          key: const Key('settings-save'),
          onPressed: onSave,
          child: Text(l10n.save),
        ),
        const SizedBox(height: 16),
        Text(l10n.audioInput(view.audioInput), key: const Key('audio-input')),
        Text(
          l10n.audioOutput(view.audioOutput),
          key: const Key('audio-output'),
        ),
        Text(l10n.audioHonest),
        const SizedBox(height: 8),
        OutlinedButton(onPressed: onRefresh, child: Text(l10n.refreshAudio)),
      ],
    );
  }
}

class _Consent extends StatelessWidget {
  const _Consent({
    required this.view,
    required this.offers,
    required this.ticked,
    required this.explicitHighRisk,
    required this.onToggle,
    required this.onExplicit,
    required this.onApprove,
    required this.onReject,
    required this.onBlock,
  });

  final DeskView view;
  final List<PermissionChoice> offers;
  final Set<String> ticked;
  final bool explicitHighRisk;
  final void Function(String name, bool value) onToggle;
  final ValueChanged<bool> onExplicit;
  final VoidCallback onApprove;
  final VoidCallback onReject;
  final VoidCallback onBlock;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context);
    final needsHighRisk = offers.any(
      (offer) => offer.highRisk && ticked.contains(offer.name),
    );
    final acceptOn = view.acceptEnabled && (!needsHighRisk || explicitHighRisk);
    return Align(
      alignment: Alignment.center,
      child: SizedBox(
        width: 440,
        height: 520,
        child: Card(
          key: const Key('consent'),
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(20),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  l10n.consentTitle,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                if (view.fraudWarning) ...[
                  const SizedBox(height: 8),
                  Text(l10n.fraudWarning, key: const Key('fraud-warning')),
                ],
                const SizedBox(height: 8),
                Text(l10n.consentBody),
                if (!view.acceptEnabled) Text(l10n.consentDelay),
                if (view.peerId.isNotEmpty)
                  Text(l10n.peerLabel(view.fingerprint)),
                for (final offer in offers)
                  CheckboxListTile(
                    key: Key('perm-${offer.name}'),
                    value: ticked.contains(offer.name),
                    onChanged: (value) => onToggle(offer.name, value ?? false),
                    title: Text(_permLabel(l10n, offer.name)),
                    controlAffinity: ListTileControlAffinity.leading,
                  ),
                if (needsHighRisk)
                  CheckboxListTile(
                    key: const Key('explicit-high-risk'),
                    value: explicitHighRisk,
                    onChanged: (value) => onExplicit(value ?? false),
                    title: Text(l10n.highRiskOptIn),
                    controlAffinity: ListTileControlAffinity.leading,
                  ),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  children: [
                    FilledButton(
                      key: const Key('accept'),
                      onPressed: acceptOn ? onApprove : null,
                      child: Text(l10n.accept),
                    ),
                    TextButton(
                      key: const Key('reject'),
                      autofocus: true,
                      onPressed: onReject,
                      child: Text(l10n.reject),
                    ),
                    TextButton(
                      key: const Key('block'),
                      onPressed: onBlock,
                      child: Text(l10n.block),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

String _permLabel(AppLocalizations l10n, String name) {
  switch (name) {
    case 'view':
      return l10n.permView;
    case 'input':
      return l10n.permInput;
    case 'clipboard':
      return l10n.permClipboard;
    case 'audio':
      return l10n.permAudio;
    case 'file_read':
      return l10n.permFileRead;
    case 'file_write':
      return l10n.permFileWrite;
    default:
      return name;
  }
}
