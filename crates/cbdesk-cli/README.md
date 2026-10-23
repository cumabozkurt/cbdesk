Minimal `cbdesk` host and viewer.

`cbdesk host` prints an invite ticket, then waits for consent. Pass
`--synthetic` for a barcode or `--screen` for the operating-system display.
`--approve` is the person's decision and still waits three seconds on a first
contact. An empty approval rejects immediately. The ticket does not grant
access.

`cbdesk view` dials that ticket and writes one AV1 frame as a PPM. There is
no window, and the command cannot hide a session.
