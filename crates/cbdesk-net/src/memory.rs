// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-process byte pipes.
//!
//! Dial enqueues a connection on an unbounded channel, so a test can dial,
//! send, and only then accept, all on one thread. Nothing here speaks QUIC.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cbdesk_crypto::DeviceId;
use tokio::sync::mpsc::{self, error::TryRecvError, UnboundedReceiver, UnboundedSender};

use crate::error::NetError;
use crate::policy::{check_hints, RelayHint};
use crate::Transport;

struct Incoming {
    from: DeviceId,
    to_dialer: UnboundedSender<Vec<u8>>,
    from_dialer: UnboundedReceiver<Vec<u8>>,
}

/// One direction of an in-memory connection.
pub struct Conn {
    peer: DeviceId,
    tx: UnboundedSender<Vec<u8>>,
    rx: UnboundedReceiver<Vec<u8>>,
}

impl Conn {
    /// Device id on the other side.
    pub fn peer(&self) -> DeviceId {
        self.peer
    }

    /// Send bytes. The peer reads them from [`Conn::recv`].
    pub async fn send(&self, bytes: Vec<u8>) -> Result<(), NetError> {
        self.tx.send(bytes).map_err(|_| NetError::Closed)
    }

    /// Wait for the next buffer.
    pub async fn recv(&mut self) -> Result<Vec<u8>, NetError> {
        self.rx.recv().await.ok_or(NetError::Closed)
    }
}

/// Shared map of device id to listener.
#[derive(Clone, Default)]
pub struct MemoryHub {
    listeners: Arc<Mutex<HashMap<DeviceId, UnboundedSender<Incoming>>>>,
}

impl MemoryHub {
    /// A hub with no listeners.
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind `id`. A second bind of the same id fails.
    pub fn bind(&self, id: DeviceId) -> Result<MemoryEndpoint, NetError> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut guard = lock(&self.listeners);
        if guard.contains_key(&id) {
            return Err(NetError::AlreadyBound);
        }
        guard.insert(id, tx);
        drop(guard);
        Ok(MemoryEndpoint {
            id,
            listeners: Arc::clone(&self.listeners),
            incoming: tokio::sync::Mutex::new(rx),
        })
    }
}

/// One device attached to a [`MemoryHub`].
pub struct MemoryEndpoint {
    id: DeviceId,
    listeners: Arc<Mutex<HashMap<DeviceId, UnboundedSender<Incoming>>>>,
    incoming: tokio::sync::Mutex<UnboundedReceiver<Incoming>>,
}

impl MemoryEndpoint {
    /// Local device id.
    pub fn device_id(&self) -> DeviceId {
        self.id
    }

    /// Connect to `peer`. Forbidden relays fail before a pipe is created.
    pub async fn dial(&self, peer: DeviceId, hints: &[RelayHint]) -> Result<Conn, NetError> {
        check_hints(hints)?;
        if peer == self.id {
            return Err(NetError::SelfDial);
        }
        let (to_dialer_tx, to_dialer_rx) = mpsc::unbounded_channel();
        let (to_acceptor_tx, to_acceptor_rx) = mpsc::unbounded_channel();
        let incoming = Incoming {
            from: self.id,
            to_dialer: to_dialer_tx,
            from_dialer: to_acceptor_rx,
        };
        let sender = {
            let guard = lock(&self.listeners);
            guard.get(&peer).cloned()
        };
        let sender = sender.ok_or(NetError::PeerUnavailable)?;
        sender.send(incoming).map_err(|_| NetError::Closed)?;
        Ok(Conn {
            peer,
            tx: to_acceptor_tx,
            rx: to_dialer_rx,
        })
    }

    /// Wait for a dialer.
    pub async fn accept(&self) -> Result<Conn, NetError> {
        let mut guard = self.incoming.lock().await;
        let incoming = guard.recv().await.ok_or(NetError::Closed)?;
        drop(guard);
        Ok(conn_from(incoming))
    }

    /// Accept if a dialer is already queued. Does not wait.
    pub fn poll_accept(&self) -> Result<Option<Conn>, NetError> {
        let mut guard = self.incoming.try_lock().map_err(|_| NetError::Busy)?;
        match guard.try_recv() {
            Ok(incoming) => Ok(Some(conn_from(incoming))),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(NetError::Closed),
        }
    }
}

impl Transport for MemoryEndpoint {
    async fn dial(&self, peer: DeviceId, hints: &[RelayHint]) -> Result<Conn, NetError> {
        MemoryEndpoint::dial(self, peer, hints).await
    }

    async fn accept(&self) -> Result<Conn, NetError> {
        MemoryEndpoint::accept(self).await
    }
}

impl Drop for MemoryEndpoint {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.listeners.lock() {
            guard.remove(&self.id);
        }
    }
}

fn conn_from(incoming: Incoming) -> Conn {
    Conn {
        peer: incoming.from,
        tx: incoming.to_dialer,
        rx: incoming.from_dialer,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_crypto::Identity;
    use cbdesk_proto::{Hello, ProtoMessage, PROTO_MAJOR};

    fn endpoint(hub: &MemoryHub, seed: u8) -> MemoryEndpoint {
        let identity = Identity::from_seed(&[seed; 32]).expect("identity");
        hub.bind(identity.device_id()).expect("bind")
    }

    #[tokio::test]
    async fn dial_then_send_then_accept_delivers_bytes() {
        let hub = MemoryHub::new();
        let dialer = endpoint(&hub, 1);
        let acceptor = endpoint(&hub, 2);
        let left = dialer.dial(acceptor.device_id(), &[]).await.expect("dial");
        let hello = Hello {
            proto_major: PROTO_MAJOR,
            app_version: "0.0.0".to_string(),
            ..Hello::default()
        };
        let bytes = hello.to_bytes().expect("encode");
        left.send(bytes.clone()).await.expect("send");
        let mut right = acceptor.accept().await.expect("accept");
        let got = right.recv().await.expect("recv");
        assert_eq!(got, bytes);
        assert_eq!(right.peer(), dialer.device_id());
    }

    #[tokio::test]
    async fn self_dial_and_forbidden_relays_offer_nothing() {
        let hub = MemoryHub::new();
        let dialer = endpoint(&hub, 3);
        let acceptor = endpoint(&hub, 4);
        let self_dial = dialer.dial(dialer.device_id(), &[]).await;
        assert_eq!(self_dial.err(), Some(NetError::SelfDial));
        let hints = [RelayHint {
            url: "relay://foo.iroh.link:443".to_string(),
        }];
        let refused = dialer.dial(acceptor.device_id(), &hints).await;
        assert!(matches!(refused, Err(NetError::ForbiddenRelay { .. })));
        assert!(acceptor.poll_accept().expect("poll").is_none());
    }
}
