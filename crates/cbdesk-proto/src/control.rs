// SPDX-License-Identifier: AGPL-3.0-or-later

//! One control-channel message.
//!
//! The QUIC stream carries a big-endian length and then these bytes. Exactly
//! one of `hello`, `hello_ack`, `granted`, or `close` is set. An invite ticket
//! is not a control body: it is a locator, not a session message.

use crate::error::ProtoError;
use crate::limits::CONTROL_MAX_BYTES;
use crate::messages::{Close, Granted, Hello, HelloAck, ProtoMessage};
use crate::wire::{self, Fields, UnknownFields};

/// Which handshake message is inside a [`Control`] frame.
#[derive(Clone, Debug, PartialEq)]
pub enum ControlBody {
    /// Controller hello.
    Hello(Hello),
    /// Host answer.
    HelloAck(HelloAck),
    /// Permissions now in force.
    Granted(Granted),
    /// The sender is closing.
    Close(Close),
}

/// A single control-channel message.
#[derive(Clone, Debug, PartialEq)]
pub struct Control {
    /// The one body this frame carries.
    pub body: ControlBody,
    /// Fields added by a newer peer.
    pub unknown: UnknownFields,
}

impl ProtoMessage for Control {
    const MAX_BYTES: usize = CONTROL_MAX_BYTES;

    fn encode(&self, out: &mut Vec<u8>) -> Result<(), ProtoError> {
        match &self.body {
            ControlBody::Hello(message) => wire::write_message(out, 1, &message.to_bytes()?),
            ControlBody::HelloAck(message) => wire::write_message(out, 2, &message.to_bytes()?),
            ControlBody::Granted(message) => wire::write_message(out, 3, &message.to_bytes()?),
            ControlBody::Close(message) => wire::write_message(out, 4, &message.to_bytes()?),
        }
        self.unknown.encode(out);
        Ok(())
    }

    fn decode_fields(fields: &mut Fields) -> Result<Self, ProtoError> {
        let present: Vec<u32> = [1, 2, 3, 4]
            .into_iter()
            .filter(|number| fields.contains(*number))
            .collect();
        if present.len() != 1 {
            return Err(ProtoError::ControlBody);
        }
        let body = match present[0] {
            1 => ControlBody::Hello(Hello::from_bytes(&fields.take_bytes(1)?)?),
            2 => ControlBody::HelloAck(HelloAck::from_bytes(&fields.take_bytes(2)?)?),
            3 => ControlBody::Granted(Granted::from_bytes(&fields.take_bytes(3)?)?),
            4 => ControlBody::Close(Close::from_bytes(&fields.take_bytes(4)?)?),
            _ => return Err(ProtoError::ControlBody),
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
    use crate::ProtoMessage;

    #[test]
    fn one_body_roundtrips_and_two_bodies_fail() {
        let message = Control {
            body: ControlBody::Hello(Hello {
                proto_major: 1,
                ..Hello::default()
            }),
            unknown: UnknownFields::new(),
        };
        let bytes = message.to_bytes().expect("encode");
        assert_eq!(Control::from_bytes(&bytes).expect("decode"), message);
        assert_eq!(
            Control::from_bytes(&[]).expect_err("empty"),
            ProtoError::ControlBody
        );

        let mut raw = Vec::new();
        wire::write_message(&mut raw, 1, &Hello::default().to_bytes().expect("hello"));
        wire::write_message(&mut raw, 4, &Close::default().to_bytes().expect("close"));
        assert_eq!(
            Control::from_bytes(&raw).expect_err("two"),
            ProtoError::ControlBody
        );
    }
}
