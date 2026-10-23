Minimal `cbdesk` host and viewer.

`cbdesk host` prints an invite ticket, then waits for consent. Pass
`--synthetic` for a barcode or `--screen` for the operating-system display.
`--approve` is the person's decision and still waits three seconds on a first
contact. An empty approval rejects immediately. The ticket does not grant
access.

`cbdesk view` dials that ticket, or finds the host with `--browse` after the
host was started with `--lan`. `--ppm` writes the first frame. `--window`
keeps showing frames until the window closes and forwards the pointer and
keyboard only after an Input grant. The command cannot hide a session.
