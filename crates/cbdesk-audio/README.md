Synthetic Opus for a consented session.

Encoding uses rusty-opus at 48 kHz, stereo, 64 kbit/s, with in-band FEC and
DTX requested. The CLI `--tone` path sends one packet per paced slot after
an Audio grant. Nothing here opens a microphone or a speaker. The jitter
buffer holds packets until 40 ms are queued and drops audio older than
120 ms.
