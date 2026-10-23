// SPDX-License-Identifier: AGPL-3.0-or-later

//! A recorder used by tests. It does not touch the operating system.

use cbdesk_proto::InputEvent;

use crate::error::InputError;
use crate::Injector;

/// Events a test can inspect after a grant.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputLog {
    events: Vec<InputEvent>,
}

impl InputLog {
    /// An empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Events in arrival order.
    pub fn events(&self) -> &[InputEvent] {
        &self.events
    }
}

impl Injector for InputLog {
    fn inject(&mut self, event: &InputEvent) -> Result<(), InputError> {
        self.events.push(event.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use cbdesk_proto::{InputBody, PointerEvent, UnknownFields};

    use super::*;

    #[test]
    fn log_keeps_events_in_order() {
        let event = InputEvent {
            body: InputBody::Pointer(PointerEvent {
                x: 3,
                y: 4,
                buttons: 0,
                wheel: 0,
                unknown: UnknownFields::new(),
            }),
            unknown: UnknownFields::new(),
        };
        let mut log = InputLog::new();
        log.inject(&event).expect("inject");
        assert_eq!(log.events(), &[event]);
    }
}
