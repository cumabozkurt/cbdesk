<!-- SPDX-License-Identifier: AGPL-3.0-or-later -->

# NAT and hole-punch measurement

This note records what this tree measured and what it did not. It is not a
field survey of ISPs.

## What was measured

`cbdesk net-check` and `cbdesk doctor` call the same report. The report binds
UDP on `0.0.0.0:0` and `[::]:0`, then connects a UDP socket to the
documentation addresses `192.0.2.1:9` and `[2001:db8::1]:9` so the kernel
selects a source address. Those calls do not transmit a datagram.

The address class is one of:

| Class | Meaning |
| --- | --- |
| `udp-blocked` | Neither UDP bind succeeded |
| `cgnat` | The IPv4 source is in `100.64.0.0/10`. The external mapping is still unknown |
| `open-internet` | A source address on this machine is a public unicast address. This is not a hole-punch result |
| `private-unmeasured` | The source is loopback, private, link-local, or missing |

`--anonymous` omits the source addresses. `--json` prints the same fields.
UPnP, DHT reachability, and hole punching are reported as not probed.

`--loopback` binds two iroh endpoints on this machine, with relays disabled,
and reads the path list after the handshake. The unit test
`loopback_paths_are_direct_and_labeled_as_not_a_nat_measurement` requires a
direct path, zero relay paths, and a selected direct path. The report text
says this is not a NAT hole punch.

## Symmetric NAT rule

`classify_mappings` is the decision rule, fed by the test, not by a live NAT:

- One local port, two remotes, the same external port: endpoint-independent.
- One local port, two remotes, different external ports: endpoint-dependent
  (symmetric). Direct hole punching often fails, so a relay is required.

No capture in this run observed a public mapping. Symmetric NAT on a real
network is unmeasured. T-NET-014 is not closed.

## What this does not claim

- A success rate for hole punching.
- Behavior of a particular ISP or CGNAT gateway beyond the address test above.
- That iroh's loopback path proves NAT traversal.
- A shipped relay process. The relay test still uses iroh's test helper.
