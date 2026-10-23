Desktop session state for the Flutter shell.

`Desk` keeps the device id, a displayed short code, a signed invite ticket,
consent, a block list, an address book, and a jailed file name. After
consent it sends one 48×16 motion-compensated VP8 pair. That picture is not
an operating-system capture. The short code does not authenticate. The
address book does not skip the first-contact delay. The session indicator
cannot be disabled. The manual clock and the test-only consent arm stay off
the FFI surface. Audio device names come from a probe and do not start a
stream.
