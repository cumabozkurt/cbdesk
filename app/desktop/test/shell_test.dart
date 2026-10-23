import 'package:cbdesk/shell.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'fake_desk.dart';

void main() {
  testWidgets(
    'home shows the device id and says the words confirm pairing',
    (tester) async {
      final desk = FakeDesk();
      await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
      await tester.pumpAndSettle();

      expect(find.text(desk.view.deviceId), findsOneWidget);
      expect(find.text(desk.view.shortCode), findsOneWidget);
      expect(
        find.text(
          'İki sözcük bu cihazla eşleşmeyi doğrular. Bir bilgisayar bulmazlar ve sunucunun yine onaylaması gerekir.',
        ),
        findsOneWidget,
      );
      expect(
        find.text('Bilet bir yer tarifidir, izin değildir.'),
        findsOneWidget,
      );
      expect(find.byKey(const Key('session-indicator')), findsNothing);
      expect(desk.view.indicatorCanBeDisabled, isFalse);
    },
  );

  testWidgets('a short code is refused in Turkish', (tester) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-connect')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('connect-field')),
      '100-elma-armut',
    );
    await tester.tap(find.byKey(const Key('connect-submit')));
    await tester.pumpAndSettle();

    expect(
      find.text(
        'Kısa kod bir bilgisayar bulmaz. Bileti, sonra kodu yapıştırın.',
      ),
      findsOneWidget,
    );
    expect(desk.connected, isNull);
  });

  testWidgets(
    'consent waits, focuses reject, and the indicator has no off switch',
    (tester) async {
      final desk = FakeDesk(view: awaitingView());
      await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
      await tester.pumpAndSettle();

      expect(find.byKey(const Key('session-indicator')), findsOneWidget);
      expect(find.text('Bu gösterge kapatılamaz.'), findsWidgets);
      expect(find.textContaining('Bankalar'), findsOneWidget);
      expect(
        tester.widget<FilledButton>(find.byKey(const Key('accept'))).onPressed,
        isNull,
      );
      expect(
        tester.widget<TextButton>(find.byKey(const Key('reject'))).autofocus,
        isTrue,
      );
      expect(find.byKey(const Key('hide-indicator')), findsNothing);

      await tester.tap(find.byKey(const Key('nav-settings')));
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('hide-indicator')), findsNothing);
      expect(find.byType(Switch), findsNothing);
      expect(find.text('Bu gösterge kapatılamaz.'), findsWidgets);

      await tester.ensureVisible(find.byKey(const Key('reject')));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const Key('reject')));
      await tester.pumpAndSettle();
      expect(desk.rejects, 1);
      expect(find.byKey(const Key('session-indicator')), findsNothing);
    },
  );

  testWidgets('file write stays off until the high-risk box is ticked', (
    tester,
  ) async {
    final desk = FakeDesk(view: awaitingView(acceptEnabled: true));
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();

    await tester.ensureVisible(find.byKey(const Key('perm-file_write')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('perm-file_write')));
    await tester.pumpAndSettle();
    expect(
      tester.widget<FilledButton>(find.byKey(const Key('accept'))).onPressed,
      isNull,
    );

    await tester.ensureVisible(find.byKey(const Key('explicit-high-risk')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('explicit-high-risk')));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.byKey(const Key('accept')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('accept')));
    await tester.pumpAndSettle();

    expect(desk.approved, ['file_write']);
    expect(desk.explicitHighRisk, isTrue);
    expect(find.byKey(const Key('session-indicator')), findsOneWidget);
  });

  testWidgets('disconnect is the indicator Kes button', (tester) async {
    final desk = FakeDesk(view: awaitingView(acceptEnabled: true));
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('disconnect')));
    await tester.pumpAndSettle();
    expect(desk.disconnects, 1);
    expect(find.byKey(const Key('session-indicator')), findsNothing);
  });

  testWidgets('the file jail refuses a parent path', (tester) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-files')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('file-name')), '../secret');
    await tester.tap(find.byKey(const Key('file-stage')));
    await tester.pumpAndSettle();
    expect(find.text('refused'), findsOneWidget);
    expect(desk.view.staged, isEmpty);

    await tester.enterText(find.byKey(const Key('file-name')), 'note.txt');
    await tester.tap(find.byKey(const Key('file-stage')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('staged-note.txt')), findsOneWidget);
  });

  testWidgets('the address book adds and removes a contact', (tester) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-address')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('contact-name')), 'Ada');
    await tester.enterText(
      find.byKey(const Key('contact-id')),
      'abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz',
    );
    await tester.tap(find.byKey(const Key('contact-add')));
    await tester.pumpAndSettle();
    expect(
      find.byKey(
        const Key(
          'contact-abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz',
        ),
      ),
      findsOneWidget,
    );

    await tester.tap(
      find.byKey(
        const Key(
          'remove-abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz',
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.byKey(
        const Key(
          'contact-abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz',
        ),
      ),
      findsNothing,
    );
  });

  testWidgets('settings switch to English and reject a bad relay', (
    tester,
  ) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-settings')));
    await tester.pumpAndSettle();
    expect(find.text('Mikrofon: unavailable'), findsOneWidget);

    await tester.tap(find.text('English'));
    await tester.pumpAndSettle();
    expect(find.text('Microphone: unavailable'), findsOneWidget);
    expect(find.text('This indicator cannot be turned off.'), findsWidgets);
    expect(desk.view.locale, 'en');

    await tester.enterText(
      find.byKey(const Key('relay')),
      'http://203.0.113.9',
    );
    await tester.tap(find.byKey(const Key('settings-save')));
    await tester.pumpAndSettle();
    expect(
      find.text('The relay must be empty or start with https:// or relay://.'),
      findsOneWidget,
    );

    await tester.enterText(
      find.byKey(const Key('relay')),
      'https://203.0.113.9:443',
    );
    await tester.tap(find.byKey(const Key('settings-save')));
    await tester.pumpAndSettle();
    expect(desk.view.relay, 'https://203.0.113.9:443');
  });

  testWidgets('a decoded inter frame is labeled as one', (tester) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-connect')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('connect-field')), 'abcd');
    await tester.tap(find.byKey(const Key('connect-submit')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('remote-frame')), findsOneWidget);
    expect(find.text('Ara kare'), findsOneWidget);
    expect(
      find.text(
        'Bu görüntü, çekirdeğin çözdüğü karedir. Masaüstü sunucu kabulden sonra işletim sistemi yakalamasını gönderir. Android, sistem yakalama onayından sonra kare gönderir.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('a ticket and a short code are sent together', (tester) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-connect')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('connect-field')), 'abcd');
    await tester.enterText(
      find.byKey(const Key('connect-code')),
      '100-elma-armut',
    );
    await tester.tap(find.byKey(const Key('connect-submit')));
    await tester.pumpAndSettle();
    expect(desk.connected, 'abcd 100-elma-armut');
  });

  testWidgets('the remote view forwards a click, Enter, and clipboard', (
    tester,
  ) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-connect')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('connect-field')), 'abcd');
    await tester.tap(find.byKey(const Key('connect-submit')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('remote-frame')));
    await tester.pumpAndSettle();
    expect(desk.pointer, isNotNull);

    await tester.ensureVisible(find.byKey(const Key('send-enter')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('send-enter')));
    await tester.pumpAndSettle();
    expect(desk.keyEvent, 'key 57344 0 0');

    await tester.ensureVisible(find.byKey(const Key('clipboard-field')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('clipboard-field')), 'merhaba');
    await tester.ensureVisible(find.byKey(const Key('clipboard-set')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('clipboard-set')));
    await tester.pumpAndSettle();
    expect(find.text('Pano: merhaba'), findsOneWidget);

    await tester.ensureVisible(find.byKey(const Key('mic')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('mic')));
    await tester.pumpAndSettle();
    expect(desk.mic, isTrue);
  });

  testWidgets('send writes file bytes and still refuses a parent path', (
    tester,
  ) async {
    final desk = FakeDesk();
    await tester.pumpWidget(CbDeskApp(api: desk, poll: null));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('nav-files')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('file-name')), '../secret');
    await tester.enterText(find.byKey(const Key('file-body')), 'nope');
    await tester.ensureVisible(find.byKey(const Key('file-send')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('file-send')));
    await tester.pumpAndSettle();
    expect(find.text('refused'), findsOneWidget);
    expect(desk.fileBytes, isNull);

    await tester.enterText(find.byKey(const Key('file-name')), 'note.txt');
    await tester.enterText(find.byKey(const Key('file-body')), 'hello');
    await tester.ensureVisible(find.byKey(const Key('file-send')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('file-send')));
    await tester.pumpAndSettle();
    expect(desk.fileBytes, [104, 101, 108, 108, 111]);
    expect(desk.view.files, ['note.txt']);
  });
}
