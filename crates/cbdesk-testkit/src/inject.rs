// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plays back frames that a test recorded earlier.

use std::collections::VecDeque;

use crate::capture::Frame;

/// A queue of frames a test can feed to a decoder.
#[derive(Clone, Debug, Default)]
pub struct RecordingInjector {
    frames: VecDeque<Frame>,
}

impl RecordingInjector {
    /// An empty recording.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a frame.
    pub fn push(&mut self, frame: Frame) {
        self.frames.push_back(frame);
    }

    /// Take the next frame, if any remain.
    pub fn inject(&mut self) -> Option<Frame> {
        self.frames.pop_front()
    }

    /// Frames still waiting.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// True when [`RecordingInjector::inject`] would return `None`.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}
