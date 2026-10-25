# User guide

This is the English user note for the foundation milestone. The same note
lives in the repository at `docs/user/en/README.md`.

When a build exists, it will ask before anyone sees your screen. A session
indicator stays visible and cannot be switched off. The invite ticket helps
the other person find your computer. It does not let them in. The two words
in the short code confirm that you are pairing with this device. They do
not find a computer, and you still have to approve.

## What each piece is for

The ticket is a locator. It expires, and it is not a permission to view or
control the computer. The short code is a check that the two sides are
talking about the same device. A wrong code does not open the approval
prompt. The person at the host still has to accept.

Seeing the screen is a separate permission from blocking the local
keyboard, covering the display, opening a terminal, forwarding a port, or
recording. Each of those still needs its own approval. The indicator cannot
be turned off to hide any of them.

## What the Linux pieces do today

On Linux, the host-operations library can grab the X11 keyboard and
pointer, map a black window, open a shell, forward one loopback TCP
connection, write a recording notice, and start an Xvfb display. `cbdesk
host` does not turn those on by itself. Windows calls `BlockInput` and
macOS creates a session event tap. A refused call does not block input.
The privacy cover is still the Linux window. A recording notice is a text
file, not an encrypted recording of the session.

## Picture and sound

The live picture is color. A 1080p software picture is not 30 frames per
second. A release measurement on one machine encoded one 1920×1080 keyframe
in 201 ms. An unchanged picture was not encoded. A 64×64 changed region
took 2 ms. A full-frame change is still about five frames per second.
`--profile balanced` and `--profile preview` shrink a screen capture
before that same encoder. A later solid-frame run, not a desktop, took
48 ms at half size and 12 ms at quarter size. `cbdesk host --strips N`
(default 1) can send those bands when the codec is VP8. Each band is its
own keyframe, not the inter-frame path, and the 52 ms figure is one solid
frame, not 30 frames per second. `--monitor N` selects one display
from the list for that call. Omitting it keeps the primary display.
The index is not a stable hardware id. A second physical monitor was
not attached here. Nobody has confirmed they heard the
microphone path.
