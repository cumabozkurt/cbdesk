Desktop session state for the Flutter shell.

`Desk` keeps the device id, a displayed short code, a signed invite ticket,
consent, a block list, an address book, and one live session. The two words
confirm pairing through SPAKE2 before consent. They do not locate a
computer and they are not written into the ticket. A wrong code does not
open the prompt. The host still approves. The address book does not skip
the first-contact delay. The session indicator cannot be disabled.

With no queued picture the host encodes `capture_picture`. A queued preview
or `submit_frame` is preferred, so the 48×16 motion test does not encode a
full display. Pointer, key, clipboard text, one audio packet, and file
bytes use the granted streams. The manual clock, the preview queue, and the
input-record flag stay off the FFI surface.
