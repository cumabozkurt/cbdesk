// SPDX-License-Identifier: AGPL-3.0-or-later

//! Capability intersection for the codecs this build can encode.
//!
//! Software priority in the plan is VP9, then VP8, then AV1. This build
//! has no VP9 encoder, and it does not select H.264 or H.265: those stay
//! on an operating-system encoder that is not linked. VP8 therefore wins
//! when both peers offer it.

use crate::{Capability, Codec};

/// Capabilities written into `Hello` and `HelloAck` by this build.
pub fn local_capabilities() -> [Capability; 4] {
    [
        Capability::VideoVp8,
        Capability::VideoAv1,
        Capability::Input,
        Capability::Clipboard,
    ]
}

/// Pick the software video codec both peers offered.
///
/// T-MED-001 is the case where both lists contain only VP8.
pub fn select_video_codec(local: &[Capability], remote: &[Capability]) -> Option<Codec> {
    const CHOICES: [(Capability, Codec); 2] = [
        (Capability::VideoVp8, Codec::Vp8),
        (Capability::VideoAv1, Codec::Av1),
    ];
    CHOICES.into_iter().find_map(|(capability, codec)| {
        (local.contains(&capability) && remote.contains(&capability)).then_some(codec)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_sides_only_vp8_select_vp8() {
        let local = [Capability::VideoVp8];
        let remote = [Capability::VideoVp8, Capability::Input];
        assert_eq!(select_video_codec(&local, &remote), Some(Codec::Vp8));
    }

    #[test]
    fn vp8_outranks_av1_when_both_are_offered() {
        let local = local_capabilities();
        assert_eq!(select_video_codec(&local, &local), Some(Codec::Vp8));
    }

    #[test]
    fn av1_is_used_when_vp8_is_missing() {
        let local = [Capability::VideoAv1];
        let remote = [Capability::VideoAv1, Capability::VideoVp9];
        assert_eq!(select_video_codec(&local, &remote), Some(Codec::Av1));
        assert_eq!(select_video_codec(&local, &[Capability::VideoVp9]), None);
    }
}
