// SPDX-License-Identifier: AGPL-3.0-or-later

//! A local hash-chained access log.
//!
//! Each entry's hash is BLAKE3 over the previous hash, the sequence number,
//! and the action text. This is not a signed transparency log and it is not
//! stored on disk by this module. Changing an earlier action makes
//! [`verify_log`] fail.

use crate::SessionError;

/// One recorded action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// Zero-based position.
    pub sequence: u64,
    /// Device id the action names, when there is one.
    pub peer: Option<[u8; 32]>,
    /// Short action name. This is not free-form session content.
    pub action: String,
    /// BLAKE3 of the previous entry and this action.
    pub hash: [u8; 32],
}

/// Entries in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessLog {
    entries: Vec<LogEntry>,
    previous: [u8; 32],
}

impl AccessLog {
    /// An empty log. The chain starts at 32 zero bytes.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append `action` for `peer`.
    pub fn append(&mut self, peer: Option<[u8; 32]>, action: &str) -> Result<(), SessionError> {
        if action.is_empty() || action.len() > 64 {
            return Err(SessionError::BadConfig);
        }
        let sequence = self.entries.len() as u64;
        let hash = chain_hash(&self.previous, sequence, peer, action);
        self.previous = hash;
        self.entries.push(LogEntry {
            sequence,
            peer,
            action: action.to_string(),
            hash,
        });
        Ok(())
    }

    /// Entries in append order.
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }
}

/// Recompute the chain. A changed action or hash fails.
pub fn verify_log(log: &AccessLog) -> Result<(), SessionError> {
    let mut previous = [0_u8; 32];
    for (index, entry) in log.entries.iter().enumerate() {
        if entry.sequence != index as u64 {
            return Err(SessionError::BadConfig);
        }
        let expect = chain_hash(&previous, entry.sequence, entry.peer, &entry.action);
        if expect != entry.hash {
            return Err(SessionError::BadConfig);
        }
        previous = entry.hash;
    }
    if previous != log.previous {
        return Err(SessionError::BadConfig);
    }
    Ok(())
}

fn chain_hash(
    previous: &[u8; 32],
    sequence: u64,
    peer: Option<[u8; 32]>,
    action: &str,
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(previous);
    hasher.update(&sequence.to_be_bytes());
    match peer {
        Some(id) => {
            hasher.update(&id);
        }
        None => {
            hasher.update(&[0_u8; 32]);
        }
    }
    hasher.update(action.as_bytes());
    *hasher.finalize().as_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_changed_action_breaks_the_chain() {
        let mut log = AccessLog::new();
        log.append(None, "grant").expect("append");
        log.append(Some([7_u8; 32]), "close").expect("append");
        verify_log(&log).expect("intact");
        log.entries[0].action = "hide".to_string();
        assert!(verify_log(&log).is_err());
    }
}
