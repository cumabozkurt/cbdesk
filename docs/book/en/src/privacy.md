# Privacy

CBDesk is designed so the project does not sit in the middle of a session.
The repository text is
[PRIVACY.md](https://github.com/cumabozkurt/cbdesk/blob/main/PRIVACY.md).

There is no account, no telemetry, and no analytics endpoint in this tree.
Device names are not broadcast unless a later settings screen records an
explicit opt-in. The LAN announcement omits the name by default.

The Ed25519 seed stays on the device. A short code's words are a password
for the pairing check. They are not written into the invite ticket. The
ticket carries addresses and a signature. It does not carry a permission
grant.

A relay you configure can see addresses and traffic volume. QUIC payloads
on the iroh path are encrypted. The project does not operate that relay.
DHT records are linkable to the device id until a blinding design is
accepted. Do not treat today's DHT helper as an unlinkable directory.

A recording notice writes a text file and may show a desktop notification.
It is not an encrypted recording of the screen.
