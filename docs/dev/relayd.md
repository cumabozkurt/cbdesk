# cbdesk-relayd

`cbdesk-relayd` is a loopback iroh relay. It binds `127.0.0.1` by default,
or `::1` when `--bind ::1` is passed. Any other address is refused before
a socket is opened. It prints one `https://` URL on stdout. The certificate
is self-signed and generated in the process. CBDesk skips certificate
checks only when every configured relay host is a literal loopback address,
so this URL works with `cbdesk host --relay` on the same machine and a
non-loopback self-signed URL does not.

n0 relay defaults stay off. The binary does not look up `iroh.link`,
`n0.computer`, `n0.io`, or `iroh.network`.

A test in `cbdesk-relayd` echoes one probe through this process with UDP
turned off. That is the same kind of loopback check the net crate already
runs against iroh's test helper. It is not a measurement of a symmetric NAT,
and it is not a hole punch.

`packaging/relayd/Dockerfile` is a sketch. It was not built here and no
image was published.

The live host still does not start this process by itself. An operator
runs it and passes the printed URL.
