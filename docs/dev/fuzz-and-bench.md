# Fuzz smoke and benchmarks

`cbdesk-proto` feeds random bytes to the hello, ticket, grant, video
header, feedback, clipboard, file, input, and control decoders. The test
is `garbage_does_not_panic`. It only checks that a decode returns instead
of panicking. It is not `cargo-fuzz`, not libFuzzer, and not OSS-Fuzz.
Those were not run.

`measure_vp8` times a software VP8 sequence. `cargo test -p cbdesk-codec
--lib a_small_sequence` runs a 32×16 pair. Two ignored tests are compiled
and skipped by a normal `cargo test`:

- `vp8_1080p_two_frames` encodes 1920×1080. The last recorded numbers are
  in `docs/dev/media-measurement.md`. That run is not 30 frames per second.
- `time_parallel_strips_on_a_64_pixel_picture` encodes four bands of a
  64×64 picture. It was not used as a performance claim.

Run an ignored test with:

```sh
cargo test -p cbdesk-codec --lib time_parallel_strips -- --ignored --nocapture
```

Criterion is not a dependency. A 1080p number from this file is not a
desktop measurement unless `docs/dev/media-measurement.md` says the run
happened.
