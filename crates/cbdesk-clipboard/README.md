Clipboard sync for files and images is not implemented here. This directory
is not a workspace member.

Plain text for a granted session lives in `cbdesk-cli` and in
`ClipboardText` (`cbdesk-proto`). That message is capped at 60 KiB so it fits
a control-sized buffer. The 64 MiB ceiling in the limits module is the later
reassembly cap.
