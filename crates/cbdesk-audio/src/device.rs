// SPDX-License-Identifier: AGPL-3.0-or-later

//! Default microphone and speaker.
//!
//! Samples are converted to 48 kHz stereo before Opus sees them. A machine
//! with no device returns [`AudioError::NoInput`] or [`AudioError::NoOutput`].
//! Nothing here invents a tone when the device is missing.

use std::sync::mpsc::{sync_channel, Receiver, RecvTimeoutError, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use crate::{AudioError, PacketCodec, CHANNELS, FRAME_SAMPLES, SAMPLE_RATE};

/// Names reported by the host audio API. Empty means that direction is absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceNames {
    /// Default input, when the operating system has one.
    pub input: Option<String>,
    /// Default output, when the operating system has one.
    pub output: Option<String>,
}

/// Ask the operating system. This does not start a stream.
pub fn probe_defaults() -> DeviceNames {
    let host = cpal::default_host();
    DeviceNames {
        input: host
            .default_input_device()
            .and_then(|device| device.name().ok()),
        output: host
            .default_output_device()
            .and_then(|device| device.name().ok()),
    }
}

/// One open input device. Packets are 20 ms Opus frames.
pub struct Microphone {
    _stream: Stream,
    rx: Receiver<Vec<f32>>,
    pending: Vec<f32>,
    position: f64,
    channels: usize,
    rate: u32,
    codec: PacketCodec,
}

impl Microphone {
    /// Open the default input and start its callback.
    pub fn open_default() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or(AudioError::NoInput)?;
        let supported = device
            .default_input_config()
            .map_err(|_| AudioError::NoInput)?;
        let rate = supported.sample_rate().0;
        let channels = usize::from(supported.channels().max(1));
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();
        if rate == 0 {
            return Err(AudioError::Unsupported);
        }
        let (tx, rx) = sync_channel(32);
        let stream = build_input(&device, &config, format, tx)?;
        stream.play().map_err(|_| AudioError::NoInput)?;
        Ok(Self {
            _stream: stream,
            rx,
            pending: Vec::new(),
            position: 0.0,
            channels,
            rate,
            codec: PacketCodec::new()?,
        })
    }

    /// Block until one 20 ms frame is available, or 1 second has passed.
    pub fn encode_packet(&mut self) -> Result<Vec<u8>, AudioError> {
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            if let Some(pcm) = pull_stereo(
                &mut self.pending,
                &mut self.position,
                self.channels,
                self.rate,
            ) {
                return self.codec.encode_pcm(&pcm);
            }
            let remain = deadline.saturating_duration_since(std::time::Instant::now());
            if remain.is_zero() {
                return Err(AudioError::Interrupted);
            }
            match self.rx.recv_timeout(remain.min(Duration::from_millis(200))) {
                Ok(chunk) => self.pending.extend(chunk),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return Err(AudioError::Interrupted),
            }
        }
    }
}

/// Background capture. [`CapturePump::pop`] never waits.
///
/// The device stream stays on the thread that opened it. cpal's stream is
/// not `Send`. The worker is detached. The receiver is in a mutex so a
/// session task can hold this value across an await (`Sync`). Dropping the
/// pump closes the channel and the worker leaves. It is not joined, because
/// encoding can block for a second.
pub struct CapturePump {
    rx: Mutex<Receiver<Vec<u8>>>,
}

impl CapturePump {
    /// Open the microphone on a worker thread, then encode there.
    pub fn start() -> Result<Self, AudioError> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (tx, rx) = sync_channel(8);
        std::thread::spawn(move || {
            let mut mic = match Microphone::open_default() {
                Ok(mic) => mic,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };
            if ready_tx.send(Ok(())).is_err() {
                return;
            }
            while let Ok(packet) = mic.encode_packet() {
                if tx.send(packet).is_err() {
                    break;
                }
            }
        });
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { rx: Mutex::new(rx) }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(AudioError::NoInput),
        }
    }

    /// The next packet, if the microphone has produced one.
    pub fn pop(&self) -> Option<Vec<u8>> {
        let Ok(rx) = self.rx.lock() else {
            return None;
        };
        rx.try_recv().ok()
    }
}

/// Default output. `play` queues 48 kHz stereo frames.
///
/// The stream stays on a private thread because it is not `Send`.
pub struct Speaker {
    tx: SyncSender<Vec<f32>>,
}

impl Speaker {
    /// Open the default output on a private thread.
    pub fn open_default() -> Result<Self, AudioError> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (tx, rx) = sync_channel(16);
        std::thread::spawn(move || {
            let host = cpal::default_host();
            let opened = (|| {
                let device = host.default_output_device().ok_or(AudioError::NoOutput)?;
                let supported = device
                    .default_output_config()
                    .map_err(|_| AudioError::NoOutput)?;
                let rate = supported.sample_rate().0;
                let channels = usize::from(supported.channels().max(1));
                let format = supported.sample_format();
                let config: StreamConfig = supported.into();
                if rate == 0 {
                    return Err(AudioError::Unsupported);
                }
                let pending = Arc::new(Mutex::new(Vec::<f32>::new()));
                let stream = build_output(
                    &device,
                    &config,
                    format,
                    rate,
                    channels,
                    Arc::clone(&pending),
                )?;
                stream.play().map_err(|_| AudioError::NoOutput)?;
                Ok((stream, pending))
            })();
            let (stream, pending) = match opened {
                Ok(pair) => pair,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };
            if ready_tx.send(Ok(())).is_err() {
                drop(stream);
                return;
            }
            while let Ok(frame) = rx.recv() {
                if let Ok(mut slot) = pending.lock() {
                    slot.extend(frame);
                }
            }
            drop(stream);
        });
        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self { tx }),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(AudioError::NoOutput),
        }
    }

    /// Queue one 20 ms stereo frame. A full buffer drops the frame.
    pub fn play(&self, stereo_48k: &[f32]) -> Result<(), AudioError> {
        if stereo_48k.len() != FRAME_SAMPLES * CHANNELS {
            return Err(AudioError::Unsupported);
        }
        match self.tx.try_send(stereo_48k.to_vec()) {
            Ok(()) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(()),
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => Err(AudioError::NoOutput),
        }
    }
}

fn build_input(
    device: &cpal::Device,
    config: &StreamConfig,
    format: SampleFormat,
    tx: SyncSender<Vec<f32>>,
) -> Result<Stream, AudioError> {
    let err = |_| {};
    match format {
        SampleFormat::F32 => device
            .build_input_stream(
                config,
                move |data: &[f32], _| {
                    let _ = tx.try_send(data.to_vec());
                },
                err,
                None,
            )
            .map_err(|_| AudioError::NoInput),
        SampleFormat::I16 => device
            .build_input_stream(
                config,
                move |data: &[i16], _| {
                    let pcm = data
                        .iter()
                        .map(|sample| f32::from(*sample) / 32768.0)
                        .collect();
                    let _ = tx.try_send(pcm);
                },
                err,
                None,
            )
            .map_err(|_| AudioError::NoInput),
        _ => Err(AudioError::Unsupported),
    }
}

fn build_output(
    device: &cpal::Device,
    config: &StreamConfig,
    format: SampleFormat,
    rate: u32,
    channels: usize,
    pending: Arc<Mutex<Vec<f32>>>,
) -> Result<Stream, AudioError> {
    let err = |_| {};
    match format {
        SampleFormat::F32 => device
            .build_output_stream(
                config,
                move |data: &mut [f32], _| fill_f32(data, &pending, rate, channels),
                err,
                None,
            )
            .map_err(|_| AudioError::NoOutput),
        SampleFormat::I16 => device
            .build_output_stream(
                config,
                move |data: &mut [i16], _| fill_i16(data, &pending, rate, channels),
                err,
                None,
            )
            .map_err(|_| AudioError::NoOutput),
        _ => Err(AudioError::Unsupported),
    }
}

fn fill_f32(data: &mut [f32], pending: &Mutex<Vec<f32>>, rate: u32, channels: usize) {
    for sample in data.iter_mut() {
        *sample = 0.0;
    }
    let Ok(mut queued) = pending.lock() else {
        return;
    };
    let mut position = 0.0_f64;
    if let Some(pcm) = take_device_frames(
        &mut queued,
        &mut position,
        channels,
        rate,
        data.len() / channels.max(1),
    ) {
        let count = data.len().min(pcm.len());
        data[..count].copy_from_slice(&pcm[..count]);
    }
}

fn fill_i16(data: &mut [i16], pending: &Mutex<Vec<f32>>, rate: u32, channels: usize) {
    for sample in data.iter_mut() {
        *sample = 0;
    }
    let Ok(mut queued) = pending.lock() else {
        return;
    };
    let mut position = 0.0_f64;
    let frames = data.len() / channels.max(1);
    if let Some(pcm) = take_device_frames(&mut queued, &mut position, channels, rate, frames) {
        for (slot, sample) in data.iter_mut().zip(pcm) {
            let scaled = (sample.clamp(-1.0, 1.0) * 32767.0).round();
            *slot = i16::try_from(scaled as i32).unwrap_or(0);
        }
    }
}

fn take_device_frames(
    queued: &mut Vec<f32>,
    position: &mut f64,
    channels: usize,
    rate: u32,
    frames: usize,
) -> Option<Vec<f32>> {
    // `queued` is 48 kHz stereo. Produce `frames` of the device format.
    let _ = position;
    if channels == 0 || frames == 0 {
        return None;
    }
    let needed = frames
        .saturating_mul(CHANNELS)
        .saturating_mul(rate as usize)
        / SAMPLE_RATE as usize;
    if queued.len() < needed.max(CHANNELS) && rate != SAMPLE_RATE as u32 {
        return None;
    }
    if rate == SAMPLE_RATE as u32 && channels == CHANNELS {
        let take = frames.saturating_mul(CHANNELS).min(queued.len());
        if take < CHANNELS {
            return None;
        }
        let pcm: Vec<f32> = queued.drain(..take).collect();
        return Some(pcm);
    }
    pull_device(queued, channels, rate, frames)
}

fn pull_device(
    queued: &mut Vec<f32>,
    channels: usize,
    rate: u32,
    frames: usize,
) -> Option<Vec<f32>> {
    let mut out = Vec::with_capacity(frames.saturating_mul(channels));
    let step = f64::from(SAMPLE_RATE) / f64::from(rate.max(1));
    let mut pos = 0.0_f64;
    let available = queued.len() / CHANNELS;
    for _ in 0..frames {
        let index = pos.floor() as usize;
        if index + 1 >= available {
            break;
        }
        let frac = (pos - index as f64) as f32;
        for channel in 0..channels {
            let src = channel.min(CHANNELS - 1);
            let a = queued[index * CHANNELS + src];
            let b = queued[(index + 1) * CHANNELS + src];
            out.push(a + (b - a) * frac);
        }
        pos += step;
    }
    let drop_frames = (pos.floor() as usize).min(available);
    let drop = drop_frames.saturating_mul(CHANNELS).min(queued.len());
    queued.drain(..drop);
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Convert captured interleaved samples into one 48 kHz stereo Opus frame.
pub fn pull_stereo(
    pending: &mut Vec<f32>,
    position: &mut f64,
    channels: usize,
    rate: u32,
) -> Option<Vec<f32>> {
    if channels == 0 || rate == 0 {
        return None;
    }
    let step = f64::from(rate) / f64::from(SAMPLE_RATE as u32);
    let end = *position + step * FRAME_SAMPLES as f64;
    let need = (end.ceil() as usize).saturating_add(1);
    let available = pending.len() / channels;
    if available < need {
        return None;
    }
    let mut out = Vec::with_capacity(FRAME_SAMPLES * CHANNELS);
    let mut pos = *position;
    for _ in 0..FRAME_SAMPLES {
        let index = pos.floor() as usize;
        let frac = (pos - index as f64) as f32;
        for channel in 0..CHANNELS {
            let src = if channels == 1 {
                0
            } else {
                channel.min(channels - 1)
            };
            let a = pending[index * channels + src];
            let b = pending[(index + 1) * channels + src];
            out.push(a + (b - a) * frac);
        }
        pos += step;
    }
    let drop_frames = pos.floor() as usize;
    let drop = drop_frames.saturating_mul(channels);
    pending.drain(..drop.min(pending.len()));
    *position = pos - drop_frames as f64;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forty_eight_kilo_passthrough_keeps_a_constant() {
        let mut pending = vec![0.25_f32; FRAME_SAMPLES * CHANNELS + CHANNELS];
        let mut position = 0.0;
        let frame =
            pull_stereo(&mut pending, &mut position, CHANNELS, SAMPLE_RATE as u32).expect("frame");
        assert_eq!(frame.len(), FRAME_SAMPLES * CHANNELS);
        assert!(frame.iter().all(|sample| (*sample - 0.25).abs() < 0.001));
    }

    #[test]
    fn mono_is_copied_to_both_ears() {
        let mut pending = vec![0.5_f32; FRAME_SAMPLES + 2];
        let mut position = 0.0;
        let frame = pull_stereo(&mut pending, &mut position, 1, SAMPLE_RATE as u32).expect("frame");
        assert!((frame[0] - 0.5).abs() < 0.001);
        assert!((frame[1] - 0.5).abs() < 0.001);
    }

    #[test]
    fn probing_reports_a_device_or_its_absence() {
        let names = probe_defaults();
        if let Some(name) = names.input {
            assert!(!name.is_empty());
        }
        if let Some(name) = names.output {
            assert!(!name.is_empty());
        }
    }

    #[test]
    fn opening_the_microphone_uses_a_real_device_or_reports_absence() {
        match Microphone::open_default() {
            Ok(mut mic) => match mic.encode_packet() {
                Ok(packet) => assert!(!packet.is_empty()),
                Err(AudioError::Interrupted) => {}
                Err(err) => panic!("{err}"),
            },
            Err(AudioError::NoInput | AudioError::Unsupported) => {}
            Err(err) => panic!("{err}"),
        }
    }

    #[test]
    fn opening_the_speaker_uses_a_real_device_or_reports_absence() {
        match Speaker::open_default() {
            Ok(speaker) => {
                let silence = vec![0_f32; FRAME_SAMPLES * CHANNELS];
                speaker.play(&silence).expect("play");
            }
            Err(AudioError::NoOutput | AudioError::Unsupported) => {}
            Err(err) => panic!("{err}"),
        }
    }
}
