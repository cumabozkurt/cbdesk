// SPDX-License-Identifier: AGPL-3.0-or-later

//! Device identity for the CLI.
//!
//! The seed is loaded from the OS keychain when that store answers. Otherwise
//! the process generates an identity and says so on stderr. The seed is not
//! printed and is not written to a file. `CBDESK_EPHEMERAL_IDENTITY=1` skips
//! the keychain so two test processes are not the same device.

use std::time::Duration;

use cbdesk_crypto::{identity_from_os, CryptoError, Identity};

use crate::error::CliError;

/// The device identity, and whether the OS keychain stored it.
pub fn open_identity() -> Result<Identity, CliError> {
    if std::env::var_os("CBDESK_EPHEMERAL_IDENTITY").is_some_and(|value| value == "1") {
        return memory_identity();
    }
    match identity_from_os(Duration::from_millis(400)) {
        Ok(identity) => {
            eprintln!("identity {} keychain", identity.device_id());
            Ok(identity)
        }
        Err(CryptoError::KeyStoreUnavailable) => {
            eprintln!(
                "cbdesk: OS keychain is unavailable; this process keeps a new identity only in memory"
            );
            memory_identity()
        }
        Err(err) => Err(CliError::new(err.to_string())),
    }
}

fn memory_identity() -> Result<Identity, CliError> {
    let identity = Identity::generate().map_err(|err| CliError::new(err.to_string()))?;
    eprintln!("identity {} memory", identity.device_id());
    Ok(identity)
}
