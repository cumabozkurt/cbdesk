// SPDX-License-Identifier: AGPL-3.0-or-later

//! Keyboard and pointer events.
//!
//! Exactly one of `key` or `pointer` is set. These bytes do not authorize
//! themselves: the host still has to have granted [`crate::Permission::Input`].

use crate::error::ProtoError;
use crate::limits::CONTROL_MAX_BYTES;
use crate::wire::{self, Fields, UnknownFields};
use crate::ProtoMessage;

/// One key transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    /// Key identifier. Printable ASCII uses the code point. Named keys use
    /// the private-use range documented by `cbdesk-input`.
    pub keycode: u32,
    /// True for a press, false for a release.
    pub down: bool,
    /// Modifier mask. The input crate defines the bits.
    pub modifiers: u32,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for KeyEvent {
    fn default() -> Self {
        Self {
            keycode: 0,
            down: false,
            modifiers: 0,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for KeyEvent {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_u32(out, 1, self.keycode);
        wire::write_bool(out, 2, self.down);
        wire::write_u32(out, 3, self.modifiers);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            keycode: fields.take_u32(1)?,
            down: fields.take_bool(2)?,
            modifiers: fields.take_u32(3)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// One pointer sample.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointerEvent {
    /// Horizontal position in screen pixels.
    pub x: i32,
    /// Vertical position in screen pixels.
    pub y: i32,
    /// Buttons currently held. Bit 0 is left, bit 1 is right, bit 2 is middle.
    pub buttons: u32,
    /// Wheel ticks. Positive is away from the person.
    pub wheel: i32,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl Default for PointerEvent {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            buttons: 0,
            wheel: 0,
            unknown: UnknownFields::new(),
        }
    }
}

impl ProtoMessage for PointerEvent {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        wire::write_i32(out, 1, self.x);
        wire::write_i32(out, 2, self.y);
        wire::write_u32(out, 3, self.buttons);
        wire::write_i32(out, 4, self.wheel);
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        Ok(Self {
            x: fields.take_i32(1)?,
            y: fields.take_i32(2)?,
            buttons: fields.take_u32(3)?,
            wheel: fields.take_i32(4)?,
            unknown: fields.finish_unknown(),
        })
    }
}

/// Which input event is inside an [`InputEvent`] frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InputBody {
    /// A key press or release.
    Key(KeyEvent),
    /// A pointer sample.
    Pointer(PointerEvent),
}

/// One input event. The QUIC stream adds a big-endian length in front.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputEvent {
    /// The one body this frame carries.
    pub body: InputBody,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl ProtoMessage for InputEvent {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        match &self.body {
            InputBody::Key(message) => wire::write_message(out, 1, &message.to_bytes()?),
            InputBody::Pointer(message) => wire::write_message(out, 2, &message.to_bytes()?),
        }
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        let present: Vec<u32> = [1, 2]
            .into_iter()
            .filter(|number| fields.contains(*number))
            .collect();
        if present.len() != 1 {
            return Err(ProtoError::InputBody);
        }
        let body = match present[0] {
            1 => InputBody::Key(KeyEvent::from_bytes(&fields.take_bytes(1)?)?),
            2 => InputBody::Pointer(PointerEvent::from_bytes(&fields.take_bytes(2)?)?),
            _ => return Err(ProtoError::InputBody),
        };
        Ok(Self {
            body,
            unknown: fields.finish_unknown(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_and_pointer_roundtrip_and_two_bodies_fail() {
        let key = InputEvent {
            body: InputBody::Key(KeyEvent {
                keycode: u32::from(b'a'),
                down: true,
                modifiers: 2,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        };
        let bytes = key.to_bytes().expect("encode");
        assert_eq!(InputEvent::from_bytes(&bytes).expect("decode"), key);

        let pointer = InputEvent {
            body: InputBody::Pointer(PointerEvent {
                x: -3,
                y: 9,
                buttons: 1,
                wheel: -1,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        };
        let bytes = pointer.to_bytes().expect("encode");
        assert_eq!(InputEvent::from_bytes(&bytes).expect("decode"), pointer);
        assert_eq!(
            InputEvent::from_bytes(&[]).expect_err("empty"),
            ProtoError::InputBody
        );
    }
}
