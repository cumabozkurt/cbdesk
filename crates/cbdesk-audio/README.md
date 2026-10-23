Opus for a consented session, plus the default microphone and speaker.

Encoding uses rusty-opus at 48 kHz, stereo, 64 kbit/s, with in-band FEC and
DTX requested. The CLI `--tone` path sends one packet per paced slot after
an Audio grant and does not open a device. `--mic` and `--speaker` open the
default devices through cpal 0.15.3. A missing device is an error. The
jitter buffer holds packets until 40 ms are queued and drops audio older
than 120 ms. This crate does not record that a person heard the stream.
