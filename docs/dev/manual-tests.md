# Manual tests

Ubuntu CI and this workspace open the viewer window under Xvfb for about
600 ms and paint one local barcode frame. That smoke test does not click,
type, or approve a prompt. A separate run on this Linux machine started
`cbdesk host --lan` and `cbdesk view --browse --ppm` and read the barcode
back. Windows and macOS have not been clicked by hand. There is no installer.

When those exist, the master plan's "M" rows (approval window over a
full-screen application, session indicator, fraud warning, Turkish layout)
need a person. Record the OS version and a screenshot in the pull request.
Do not mark those rows done from a unit test.

Automated stand-ins that exist today are listed in `docs/dev/m0-status.md`.
