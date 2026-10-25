`cbdesk` host and viewer for a consented color session.

`cbdesk host` prints an invite ticket, then waits for consent. Pass
`--synthetic` for a barcode or `--screen` for the primary display.
`--monitor N` selects an index from `list_monitors` for that call. The
index is not a stable hardware id. An index past the list is refused.
A second physical monitor was not attached here.
`--approve` is the person's decision and still waits three seconds on a
first contact. An empty approval rejects immediately. The ticket does not
grant access. `file_write` also needs `--allow-high-risk` and
`--files-root`.

`cbdesk view` dials that ticket, or finds the host with `--browse` after the
host was started with `--lan`. `--ppm` writes the first frame. `--window`
keeps showing frames until the window closes and forwards the pointer and
keyboard only after an Input grant. `--put` uploads one file. `--audio`
waits for one Opus packet. `--mic` opens the default microphone and
`--speaker` opens the default speaker. `--tone` does not open a device, and
it cannot be combined with `--mic`. A missing device is an error. `--relay`
names a relay this process may use. The command cannot hide a session.
