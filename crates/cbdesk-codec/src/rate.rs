// SPDX-License-Identifier: AGPL-3.0-or-later

//! A small bitrate adapter for the live picture.
//!
//! The target interval is 33 ms. Quantizer index is the lever this encoder
//! actually has: a higher index is a coarser picture. Three rising delay
//! samples, or packet loss above 2 percent, coarsen it. A fast encode with a
//! quiet network refines it. This is not a claim that a full-resolution
//! software frame finishes inside 33 ms.

/// What the sender should do after measuring one encode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pace {
    /// Send the picture that was just encoded.
    Send,
    /// The encode missed the frame budget. Skip the following capture.
    SkipNext,
}

/// Quantizer choice for one live VP8 stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BitrateControl {
    qindex: u8,
    last_delay: Option<u32>,
    delay_rises: u8,
}

impl BitrateControl {
    /// Frame budget the pacer aims at, in milliseconds.
    pub const TARGET_MS: u32 = 33;

    /// A live picture starts at a mid quantizer, not at lossless.
    pub fn live() -> Self {
        Self {
            qindex: 32,
            last_delay: None,
            delay_rises: 0,
        }
    }

    /// Current VP8 `y_ac_qi`, from 0 through 127.
    pub fn qindex(&self) -> u8 {
        self.qindex
    }

    /// Account for one encode. Over budget coarsens and asks to skip a slot.
    pub fn observe_encode(&mut self, elapsed_ms: u32) -> Pace {
        if elapsed_ms > Self::TARGET_MS {
            self.raise(8);
            Pace::SkipNext
        } else if elapsed_ms < 16 {
            self.lower(2);
            Pace::Send
        } else {
            Pace::Send
        }
    }

    /// Account for one receiver report.
    ///
    /// `loss` is a fraction from 0 to 1. Three consecutive increases in
    /// `delay_ms` coarsen the quantizer. The step is a fixed index change,
    /// which is this encoder's stand-in for the plan's 15 percent rate cut.
    pub fn observe_feedback(&mut self, loss: f32, delay_ms: u32) {
        if loss > 0.02 {
            self.raise(4);
        }
        if let Some(previous) = self.last_delay {
            if delay_ms > previous {
                self.delay_rises = self.delay_rises.saturating_add(1);
            } else {
                self.delay_rises = 0;
            }
        }
        self.last_delay = Some(delay_ms);
        if self.delay_rises >= 3 {
            self.raise(8);
            self.delay_rises = 0;
        } else if loss == 0.0 && delay_ms <= 20 {
            self.lower(1);
        }
    }

    fn raise(&mut self, step: u8) {
        self.qindex = self.qindex.saturating_add(step).min(127);
    }

    fn lower(&mut self, step: u8) {
        self.qindex = self.qindex.saturating_sub(step);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slow_encode_skips_the_next_slot_and_coarsens() {
        let mut control = BitrateControl::live();
        assert_eq!(control.observe_encode(40), Pace::SkipNext);
        assert!(control.qindex() > 32);
    }

    #[test]
    fn a_fast_encode_refines_the_quantizer() {
        let mut control = BitrateControl::live();
        assert_eq!(control.observe_encode(10), Pace::Send);
        assert!(control.qindex() < 32);
    }

    #[test]
    fn three_delay_increases_coarsen_once() {
        let mut control = BitrateControl::live();
        let start = control.qindex();
        control.observe_feedback(0.0, 30);
        control.observe_feedback(0.0, 40);
        control.observe_feedback(0.0, 50);
        control.observe_feedback(0.0, 60);
        assert!(control.qindex() > start);
    }

    #[test]
    fn loss_above_two_percent_coarsens() {
        let mut control = BitrateControl::live();
        control.observe_feedback(0.05, 10);
        assert!(control.qindex() > 32);
    }
}
