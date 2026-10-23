// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(test), deny(clippy::unwrap_used))]
#![cfg_attr(not(test), deny(clippy::expect_used))]

//! Opus audio after an Audio grant.
//!
//! Frames are 20 ms, 48 kHz, stereo. In-band FEC and DTX are on. A jitter
//! buffer holds 40 to 120 ms. [`Microphone`] and [`Speaker`] open the
//! operating system's default devices. A missing device is an error.

mod device;

use rusty_opus::{Application, OpusDecoder, OpusEncoder};
use thiserror::Error;

pub use device::{probe_defaults, CapturePump, DeviceNames, Microphone, Speaker};

/// Samples per second.
pub const SAMPLE_RATE: i32 = 48_000;

/// Stereo.
pub const CHANNELS: usize = 2;

/// Samples per channel in one 20 ms frame.
pub const FRAME_SAMPLES: usize = 960;

/// How a tone or a packet failed.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AudioError {
    /// The codec rejected the frame.
    #[error("opus failed: {0}")]
    Codec(&'static str),
    /// This machine has no default input device.
    #[error("microphone unavailable")]
    NoInput,
    /// This machine has no default output device.
    #[error("speaker unavailable")]
    NoOutput,
    /// The device format cannot be converted to 48 kHz stereo.
    #[error("audio device format is not supported")]
    Unsupported,
    /// The device did not deliver a full frame in time.
    #[error("microphone produced no samples")]
    Interrupted,
}

/// Encode one synthetic stereo tone and decode it again.
pub struct ToneCodec {
    encoder: OpusEncoder,
    decoder: OpusDecoder,
    frame: u32,
}

impl ToneCodec {
    /// An encoder with FEC and DTX enabled, and a matching decoder.
    pub fn new() -> Result<Self, AudioError> {
        Ok(Self {
            encoder: new_encoder()?,
            decoder: new_decoder()?,
            frame: 0,
        })
    }

    /// Encode the next 20 ms tone. The packet is what a host would send.
    pub fn encode_frame(&mut self) -> Result<Vec<u8>, AudioError> {
        let pcm = tone(self.frame);
        self.frame = self.frame.saturating_add(1);
        let mut packet = [0_u8; 1500];
        let len = self
            .encoder
            .encode(&pcm, FRAME_SAMPLES, &mut packet)
            .map_err(|_| AudioError::Codec("encode"))?;
        Ok(packet[..len].to_vec())
    }

    /// Decode one packet, or conceal a loss when `packet` is empty.
    pub fn decode_frame(&mut self, packet: &[u8]) -> Result<Vec<f32>, AudioError> {
        let mut pcm = vec![0_f32; FRAME_SAMPLES * CHANNELS];
        let samples = self
            .decoder
            .decode(packet, FRAME_SAMPLES, &mut pcm)
            .map_err(|_| AudioError::Codec("decode"))?;
        if samples != FRAME_SAMPLES {
            return Err(AudioError::Codec("length"));
        }
        Ok(pcm)
    }
}

/// Encode and decode one Opus packet of captured or received PCM.
pub struct PacketCodec {
    encoder: OpusEncoder,
    decoder: OpusDecoder,
}

impl PacketCodec {
    /// FEC and DTX match [`ToneCodec`].
    pub fn new() -> Result<Self, AudioError> {
        Ok(Self {
            encoder: new_encoder()?,
            decoder: new_decoder()?,
        })
    }

    /// Encode 20 ms of interleaved stereo `pcm` at 48 kHz.
    pub fn encode_pcm(&mut self, pcm: &[f32]) -> Result<Vec<u8>, AudioError> {
        if pcm.len() != FRAME_SAMPLES * CHANNELS {
            return Err(AudioError::Codec("length"));
        }
        let mut packet = [0_u8; 1500];
        let len = self
            .encoder
            .encode(pcm, FRAME_SAMPLES, &mut packet)
            .map_err(|_| AudioError::Codec("encode"))?;
        Ok(packet[..len].to_vec())
    }

    /// Decode one packet, or conceal a loss when `packet` is empty.
    pub fn decode_packet(&mut self, packet: &[u8]) -> Result<Vec<f32>, AudioError> {
        let mut pcm = vec![0_f32; FRAME_SAMPLES * CHANNELS];
        let samples = self
            .decoder
            .decode(packet, FRAME_SAMPLES, &mut pcm)
            .map_err(|_| AudioError::Codec("decode"))?;
        if samples != FRAME_SAMPLES {
            return Err(AudioError::Codec("length"));
        }
        Ok(pcm)
    }
}

fn new_encoder() -> Result<OpusEncoder, AudioError> {
    let mut encoder = OpusEncoder::new(SAMPLE_RATE, CHANNELS, Application::Audio)
        .map_err(|_| AudioError::Codec("encoder"))?;
    encoder.bitrate_bps = 64_000;
    encoder.complexity = 5;
    encoder.use_inband_fec = true;
    encoder.use_dtx = true;
    encoder.packet_loss_perc = 10;
    Ok(encoder)
}

fn new_decoder() -> Result<OpusDecoder, AudioError> {
    OpusDecoder::new(SAMPLE_RATE, CHANNELS).map_err(|_| AudioError::Codec("decoder"))
}

/// 40–120 ms of packets. Nothing is released until 40 ms has arrived.
#[derive(Debug)]
pub struct JitterBuffer {
    queued: std::collections::VecDeque<Vec<u8>>,
    primed: bool,
}

impl JitterBuffer {
    /// Two frames before the first pop, six frames at most.
    pub fn new() -> Self {
        Self {
            queued: std::collections::VecDeque::new(),
            primed: false,
        }
    }

    /// Store `packet`, dropping the oldest past 120 ms.
    pub fn push(&mut self, packet: Vec<u8>) {
        if self.queued.len() >= 6 {
            self.queued.pop_front();
        }
        self.queued.push_back(packet);
        if self.queued.len() >= 2 {
            self.primed = true;
        }
    }

    /// The next packet after the buffer has reached 40 ms.
    pub fn pop(&mut self) -> Option<Vec<u8>> {
        if !self.primed {
            None
        } else {
            self.queued.pop_front()
        }
    }
}

impl Default for JitterBuffer {
    fn default() -> Self {
        Self::new()
    }
}

fn tone(frame: u32) -> Vec<f32> {
    let mut pcm = vec![0_f32; FRAME_SAMPLES * CHANNELS];
    for sample in 0..FRAME_SAMPLES {
        let phase = (frame as f32) * 0.4 + (sample as f32) * 0.05;
        let value = phase.sin() * 0.2;
        pcm[sample * 2] = value;
        pcm[sample * 2 + 1] = value;
    }
    pcm
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(pcm: &[f32]) -> f32 {
        pcm.iter().map(|sample| sample * sample).sum()
    }

    #[test]
    fn a_tone_round_trips_and_a_loss_is_concealed() {
        let mut codec = ToneCodec::new().expect("codec");
        let packet = codec.encode_frame().expect("encode");
        assert!(!packet.is_empty());
        let decoded = codec.decode_frame(&packet).expect("decode");
        assert_eq!(decoded.len(), FRAME_SAMPLES * CHANNELS);
        assert!(energy(&decoded) > 0.01);
        let concealed = codec.decode_frame(&[]).expect("plc");
        assert_eq!(concealed.len(), FRAME_SAMPLES * CHANNELS);
    }

    #[test]
    fn captured_pcm_round_trips_without_opening_a_device() {
        let mut codec = PacketCodec::new().expect("codec");
        let mut pcm = vec![0_f32; FRAME_SAMPLES * CHANNELS];
        for sample in 0..FRAME_SAMPLES {
            let value = (sample as f32 * 0.02).sin() * 0.2;
            pcm[sample * 2] = value;
            pcm[sample * 2 + 1] = value;
        }
        let packet = codec.encode_pcm(&pcm).expect("encode");
        assert!(!packet.is_empty());
        let decoded = codec.decode_packet(&packet).expect("decode");
        assert!(energy(&decoded) > 0.01);
    }

    #[test]
    fn the_jitter_buffer_waits_for_forty_milliseconds() {
        let mut buffer = JitterBuffer::new();
        buffer.push(vec![1]);
        assert!(buffer.pop().is_none());
        buffer.push(vec![2]);
        assert_eq!(buffer.pop(), Some(vec![1]));
        for value in 3..10 {
            buffer.push(vec![value]);
        }
        let next = buffer.pop().expect("queued");
        assert!(next[0] >= 3);
    }
}
