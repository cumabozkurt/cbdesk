# CBDesk

CBDesk is not ready to install. This page is the English user note for the
foundation milestone.

When a build exists, it will ask before anyone sees your screen. A session
indicator stays visible and cannot be switched off. The invite ticket helps
the other person find your computer. It does not let them in. The two words
in the short code confirm that you are pairing with this device. They do
not find a computer, and you still have to approve.

If someone phoned you and told you to install remote-access software, refuse.
Banks, government agencies, and support companies do not ask for that.

There is no account to create and no CBDesk server that your session is
required to pass through. A relay, if you configure one, is a machine you
choose. This project does not run it.

Seeing the screen is a separate permission from blocking the local keyboard,
covering the display, opening a terminal, forwarding a port, or recording.
Each of those still needs its own approval, and the indicator cannot be
turned off. The Linux pieces that exist (an X11 grab, a black window, a
shell, a loopback tunnel, a notice file, and an Xvfb display) are not
switched on by `cbdesk host` yet. Windows calls `BlockInput` and macOS
creates a session event tap; a refused call does not block input, and the
cover is still the Linux window. Nothing here is a signed install. A
1080p software picture is not 30 frames per second.
