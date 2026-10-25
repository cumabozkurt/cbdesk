// SPDX-License-Identifier: AGPL-3.0-or-later

//! Whiteboard strokes drawn on top of a session.
//!
//! A stroke is accepted only while the session indicator is on. This module
//! stores points. It does not draw them and it does not hide the picture.

use crate::SessionError;

/// One stroke in session coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stroke {
    /// Red, green, blue.
    pub color: [u8; 3],
    /// Points as `(x, y)` pairs. At least one, at most 256.
    pub points: Vec<(i32, i32)>,
}

/// Accept `points` when the indicator is visible.
pub fn accept_stroke(
    indicator_on: bool,
    color: [u8; 3],
    points: Vec<(i32, i32)>,
) -> Result<Stroke, SessionError> {
    if !indicator_on {
        return Err(SessionError::MarkupNeedsIndicator);
    }
    if points.is_empty() || points.len() > 256 {
        return Err(SessionError::BadConfig);
    }
    Ok(Stroke { color, points })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stroke_requires_the_indicator() {
        assert!(accept_stroke(false, [255, 0, 0], vec![(1, 1)]).is_err());
        let stroke = accept_stroke(true, [255, 0, 0], vec![(1, 1), (2, 2)]).expect("stroke");
        assert_eq!(stroke.points.len(), 2);
    }
}
