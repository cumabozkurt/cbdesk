<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# Deployment notes that were not executed

No installer was built or signed in this pass. The sketches below are not
procedures that were run, and they are not a release.

## Packages

Windows MSI, macOS pkg, deb, rpm, Flatpak, winget, Homebrew, AUR, F-Droid,
and a Docker image of `cbdesk-relayd` are not published. This repository
does not ship a relay daemon. GitHub Releases, a TUF root, and an update
URL are not configured. ADR-013 stays the reason: no root key is checked
into git, and no updater runs.

## Organization rollout

Group Policy, Intune, Jamf, and Ansible would install a signed package and
would not disable the session indicator. Those tools were not pointed at a
package, because there is no signed package. An organization certificate
format is not parsed. ADR-011 stays Proposed.

## iOS

An iOS controller needs an Apple Developer membership, a Mac with Xcode, and
a device or TestFlight. This agent did not add an iOS target. A stub project
would not be a controller and could not be signed here.

## Virtual display

`open_virtual_display` fails. Windows indirect-display signing has not
started. A headless host still needs a real display or it returns
`Unavailable`.

## Audit and soak

An independent security audit has not been commissioned. A 72-hour soak and
a reproducible-build check were not run. Antivirus vendors were not
notified. Those are owner or external steps, not code that this tree can
close by itself.
