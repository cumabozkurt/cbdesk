// SPDX-License-Identifier: AGPL-3.0-or-later

//! Where the device seed lives between processes.
//!
//! [`MemoryKeyStore`] is the test double. [`OsKeyStore`] asks the operating
//! system credential store (Secret Service, Keychain, or Credential Manager)
//! and returns [`CryptoError::KeyStoreUnavailable`] when that store cannot
//! be used. A missing entry is not an error: the caller may create one.
//! The seed is never written to a file by this module.

use std::sync::Mutex;
use std::time::Duration;

use zeroize::Zeroize;

use crate::error::CryptoError;
use crate::Identity;

/// Load and store the 32-byte Ed25519 seed for this device.
pub trait IdentityStore {
    /// The saved seed, or `None` when this name has no entry yet.
    fn load(&self) -> Result<Option<[u8; 32]>, CryptoError>;

    /// Replace the saved seed. Callers wipe their own copy.
    fn store(&self, seed: &[u8; 32]) -> Result<(), CryptoError>;
}

/// In-memory seed. Dropping the store drops the seed.
#[derive(Debug, Default)]
pub struct MemoryKeyStore {
    seed: Mutex<Option<[u8; 32]>>,
}

impl IdentityStore for MemoryKeyStore {
    fn load(&self) -> Result<Option<[u8; 32]>, CryptoError> {
        Ok(*lock_seed(&self.seed))
    }

    fn store(&self, seed: &[u8; 32]) -> Result<(), CryptoError> {
        *lock_seed(&self.seed) = Some(*seed);
        Ok(())
    }
}

/// The operating-system credential store, service `cbdesk`.
#[derive(Clone, Copy, Debug, Default)]
pub struct OsKeyStore;

impl IdentityStore for OsKeyStore {
    fn load(&self) -> Result<Option<[u8; 32]>, CryptoError> {
        with_entry(|entry| match entry.get_secret() {
            Ok(bytes) if bytes.len() == 32 => {
                let mut seed = [0_u8; 32];
                seed.copy_from_slice(&bytes);
                Ok(Some(seed))
            }
            Ok(_) | Err(keyring::Error::NoEntry) => Ok(None),
            Err(_) => Err(CryptoError::KeyStoreUnavailable),
        })
    }

    fn store(&self, seed: &[u8; 32]) -> Result<(), CryptoError> {
        let mut owned = *seed;
        let result = with_entry(|entry| {
            entry
                .set_secret(&owned)
                .map_err(|_| CryptoError::KeyStoreUnavailable)
        });
        owned.zeroize();
        result
    }
}

/// Load a seed or create one and store it.
///
/// The copy used for [`Identity::from_seed`] is wiped before this returns.
pub fn identity_from_store(store: &dyn IdentityStore) -> Result<Identity, CryptoError> {
    let identity = match store.load()? {
        Some(mut seed) => {
            let identity = Identity::from_seed(&seed)?;
            seed.zeroize();
            identity
        }
        None => {
            let identity = Identity::generate()?;
            let mut seed = identity.signing_seed();
            store.store(&seed)?;
            seed.zeroize();
            identity
        }
    };
    Ok(identity)
}

/// Load or create the OS identity, giving the daemon `timeout` to answer.
///
/// A credential daemon that does not answer is
/// [`CryptoError::KeyStoreUnavailable`]. The call does not wait forever.
pub fn identity_from_os(timeout: Duration) -> Result<Identity, CryptoError> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(identity_from_store(&OsKeyStore));
    });
    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => Err(CryptoError::KeyStoreUnavailable),
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            Err(CryptoError::KeyStoreUnavailable)
        }
    }
}

fn with_entry<T>(
    body: impl FnOnce(keyring::Entry) -> Result<T, CryptoError>,
) -> Result<T, CryptoError> {
    let entry =
        keyring::Entry::new(SERVICE, ACCOUNT).map_err(|_| CryptoError::KeyStoreUnavailable)?;
    body(entry)
}

fn lock_seed(seed: &Mutex<Option<[u8; 32]>>) -> std::sync::MutexGuard<'_, Option<[u8; 32]>> {
    seed.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

const SERVICE: &str = "cbdesk";
const ACCOUNT: &str = "signing-seed";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trips_the_same_device() {
        let store = MemoryKeyStore::default();
        let first = identity_from_store(&store).expect("create");
        let second = identity_from_store(&store).expect("load");
        assert_eq!(first.device_id(), second.device_id());
        assert!(store.load().expect("load").is_some());
    }

    #[test]
    fn os_store_answers_or_reports_unavailable() {
        let result = identity_from_os(Duration::from_millis(500));
        match result {
            Ok(identity) => {
                let _ = identity.device_id();
            }
            Err(CryptoError::KeyStoreUnavailable) => {}
            Err(err) => panic!("unexpected keystore error: {err}"),
        }
    }
}
