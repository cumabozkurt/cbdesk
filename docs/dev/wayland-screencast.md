<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Wayland ScreenCast test plan

CI does not run this path. Ubuntu CI captures through Xvfb, which is X11.
`WAYLAND_DISPLAY` is unset there, so the ScreenCast branch is compiled and
not executed. A portal dialog also needs a person. Do not mark this plan
passed from a unit test.

## What has to be installed

- A Wayland session (`XDG_SESSION_TYPE=wayland`, `WAYLAND_DISPLAY` set).
- `xdg-desktop-portal` and the backend for that desktop (GNOME or KDE).
- PipeWire (`libpipewire-0.3`).
- A running desktop, not a headless SSH session. The portal shows a picker.

Record `XDG_CURRENT_DESKTOP`, the portal version (`xdg-desktop-portal --version`),
and the PipeWire version before the run.

## Steps

1. Start `cbdesk host --bind 0.0.0.0:22116 --screen --approve view` in the
   Wayland session.
2. Accept the portal dialog and pick a monitor. Cancelling the dialog must
   surface `screen capture is unavailable`, not a synthetic barcode.
3. From another machine, or another seat that can dial the ticket, run
   `cbdesk view --ticket HEX --window`.
4. Confirm the window shows the chosen monitor, and that the session
   indicator on the host stays visible. There is no control that hides it.
5. Stop the view. The host capture should end. The portal session should
   not keep streaming after the process exits.
6. Repeat on a second desktop environment if one is available. GNOME and
   KDE do not implement the portal the same way. A pass on one does not
   cover the other.
7. Lock the screen and try again. Failure here is a result to write down,
   not a reason to skip the note.

## Pass record

Write the desktop, portal version, whether the picker appeared, whether a
frame arrived, and whether cancel produced `Unavailable`. A missing portal
is a failed environment, not a passed capture.

Input injection on Wayland is a separate check and is not part of this plan.
