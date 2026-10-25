// SPDX-License-Identifier: AGPL-3.0-or-later

//! Permanent password via the published `opaque-ke` 4.0.1 crate.
//!
//! The host stores a [`HostSetup`] and a [`PasswordRecord`]. Registration and
//! login functions that run on the host do not take the password. The
//! controller passes the password only into the client functions. Messages
//! that cross the gap are the OPAQUE byte strings. This module does not define
//! a cipher. The key-stretching function is Argon2id from the `argon2` crate's
//! defaults, selected by `opaque-ke`'s `Argon2` implementation of `Ksf`.
//!
//! A local caller that types the password still has it in their own memory.
//! This API cannot wipe that caller's buffer. It copies into a [`Zeroizing`]
//! buffer for the protocol and drops that copy when the call returns.
//!
//! The composition has not had an external review.

use opaque_ke::ciphersuite::CipherSuite;
use opaque_ke::errors::ProtocolError;
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters, CredentialFinalization, CredentialRequest,
    CredentialResponse, RegistrationRequest, RegistrationResponse, RegistrationUpload, ServerLogin,
    ServerLoginParameters, ServerRegistration, ServerSetup,
};
use rand::rngs::OsRng;
use sha2::Sha512;
use zeroize::Zeroizing;

use crate::CryptoError;

const IDENTIFIER: &[u8] = b"cbdesk-host-password-v1";

/// Minimum length in Unicode scalar values.
pub const MIN_PASSWORD_CHARS: usize = 10;

/// Maximum length in bytes. Longer inputs are refused before Argon2.
pub const MAX_PASSWORD_BYTES: usize = 128;

/// Minimum zxcvbn score. The scale is 0 through 4.
pub const MIN_ZXCVBN_SCORE: u8 = 3;

struct DeskSuite;

impl CipherSuite for DeskSuite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<opaque_ke::Ristretto255, Sha512>;
    type Ksf = opaque_ke::argon2::Argon2<'static>;
}

/// OPRF seed and host key pair. This is a secret. It is not the password.
pub struct HostSetup {
    raw: Zeroizing<Vec<u8>>,
}

/// Registration record stored on the host. It is not the password.
///
/// The upload that created it is about as sensitive as a password hash.
pub struct PasswordRecord {
    raw: Vec<u8>,
}

/// Controller state between the two registration messages.
pub struct PendingRegistration {
    state: ClientRegistration<DeskSuite>,
}

/// Controller state between the two login messages.
pub struct PendingLogin {
    state: ClientLogin<DeskSuite>,
}

/// Host state between the two login messages.
pub struct PendingHostLogin {
    state: ServerLogin<DeskSuite>,
}

impl std::fmt::Debug for HostSetup {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("HostSetup")
    }
}

impl std::fmt::Debug for PasswordRecord {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PasswordRecord")
    }
}

impl std::fmt::Debug for PendingRegistration {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PendingRegistration")
    }
}

impl std::fmt::Debug for PendingLogin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PendingLogin")
    }
}

impl std::fmt::Debug for PendingHostLogin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("PendingHostLogin")
    }
}

impl HostSetup {
    /// Create a new setup from the operating-system random source.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut rng = OsRng;
        let setup = ServerSetup::<DeskSuite>::new(&mut rng);
        Ok(Self {
            raw: Zeroizing::new(setup.serialize().as_slice().to_vec()),
        })
    }

    /// Restore a setup that [`Self::to_bytes`] produced.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let _ = ServerSetup::<DeskSuite>::deserialize(bytes)
            .map_err(|_| CryptoError::PasswordProtocol)?;
        Ok(Self {
            raw: Zeroizing::new(bytes.to_vec()),
        })
    }

    /// Bytes to store. The caller treats them as a secret.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.to_vec()
    }

    fn parsed(&self) -> Result<ServerSetup<DeskSuite>, CryptoError> {
        ServerSetup::<DeskSuite>::deserialize(&self.raw).map_err(|_| CryptoError::PasswordProtocol)
    }
}

impl PasswordRecord {
    /// Restore a record that [`Self::to_bytes`] produced.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        let _ = ServerRegistration::<DeskSuite>::deserialize(bytes)
            .map_err(|_| CryptoError::PasswordProtocol)?;
        Ok(Self {
            raw: bytes.to_vec(),
        })
    }

    /// Bytes to store on the host.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.raw.clone()
    }

    fn parsed(&self) -> Result<ServerRegistration<DeskSuite>, CryptoError> {
        ServerRegistration::<DeskSuite>::deserialize(&self.raw)
            .map_err(|_| CryptoError::PasswordProtocol)
    }
}

/// Accept `password` when it is long enough and zxcvbn scores at least 3.
///
/// The returned buffer is wiped on drop. `cbdesk` is passed to zxcvbn as a
/// user input so a password that is only the product name scores lower.
pub fn accept_password(password: &str) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    if password.chars().count() < MIN_PASSWORD_CHARS || password.len() > MAX_PASSWORD_BYTES {
        return Err(CryptoError::PasswordPolicy);
    }
    let estimate = zxcvbn::zxcvbn(password, &["cbdesk"]);
    if u8::from(estimate.score()) < MIN_ZXCVBN_SCORE {
        return Err(CryptoError::PasswordPolicy);
    }
    Ok(Zeroizing::new(password.as_bytes().to_vec()))
}

/// Argon2id parameters the KSF uses, from the `argon2` crate defaults.
pub fn argon2_parameters() -> (u32, u32, u32) {
    let params = opaque_ke::argon2::Params::default();
    (params.m_cost(), params.t_cost(), params.p_cost())
}

/// First controller message. The password is not copied into the returned bytes.
pub fn registration_request(
    password: &[u8],
) -> Result<(PendingRegistration, Vec<u8>), CryptoError> {
    let mut rng = OsRng;
    let started = ClientRegistration::<DeskSuite>::start(&mut rng, password)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let bytes = started.message.serialize().as_slice().to_vec();
    Ok((
        PendingRegistration {
            state: started.state,
        },
        bytes,
    ))
}

/// Host answer to [`registration_request`]. This function does not take a password.
pub fn registration_response(setup: &HostSetup, request: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let message = RegistrationRequest::<DeskSuite>::deserialize(request)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let started = ServerRegistration::<DeskSuite>::start(&setup.parsed()?, message, IDENTIFIER)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    Ok(started.message.serialize().as_slice().to_vec())
}

/// Second controller message. The upload is what the host stores, not the password.
pub fn registration_upload(
    pending: PendingRegistration,
    password: &[u8],
    response: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let mut rng = OsRng;
    let message = RegistrationResponse::<DeskSuite>::deserialize(response)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let finished = pending
        .state
        .finish(
            &mut rng,
            password,
            message,
            ClientRegistrationFinishParameters::default(),
        )
        .map_err(|_| CryptoError::PasswordProtocol)?;
    Ok(finished.message.serialize().as_slice().to_vec())
}

/// Store the upload. This function does not take a password.
pub fn registration_record(upload: &[u8]) -> Result<PasswordRecord, CryptoError> {
    let message = RegistrationUpload::<DeskSuite>::deserialize(upload)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let record = ServerRegistration::<DeskSuite>::finish(message);
    Ok(PasswordRecord {
        raw: record.serialize().as_slice().to_vec(),
    })
}

/// First controller login message.
pub fn login_request(password: &[u8]) -> Result<(PendingLogin, Vec<u8>), CryptoError> {
    let mut rng = OsRng;
    let started = ClientLogin::<DeskSuite>::start(&mut rng, password)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let bytes = started.message.serialize().as_slice().to_vec();
    Ok((
        PendingLogin {
            state: started.state,
        },
        bytes,
    ))
}

/// Host answer. `record` is `None` when this host has no password file.
///
/// The response is produced either way so the byte string does not reveal
/// whether a record exists. This function does not take a password.
pub fn login_response(
    setup: &HostSetup,
    record: Option<&PasswordRecord>,
    request: &[u8],
) -> Result<(PendingHostLogin, Vec<u8>), CryptoError> {
    let message = CredentialRequest::<DeskSuite>::deserialize(request)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let file = match record {
        Some(record) => Some(record.parsed()?),
        None => None,
    };
    let mut rng = OsRng;
    let started = ServerLogin::start(
        &mut rng,
        &setup.parsed()?,
        file,
        message,
        IDENTIFIER,
        ServerLoginParameters::default(),
    )
    .map_err(|_| CryptoError::PasswordProtocol)?;
    let bytes = started.message.serialize().as_slice().to_vec();
    Ok((
        PendingHostLogin {
            state: started.state,
        },
        bytes,
    ))
}

/// Finish the controller side. A wrong password is [`CryptoError::PasswordRejected`].
pub fn login_finish(
    pending: PendingLogin,
    password: &[u8],
    response: &[u8],
) -> Result<(Zeroizing<Vec<u8>>, Vec<u8>), CryptoError> {
    let mut rng = OsRng;
    let message = CredentialResponse::<DeskSuite>::deserialize(response)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let finished = pending
        .state
        .finish(
            &mut rng,
            password,
            message,
            ClientLoginFinishParameters::default(),
        )
        .map_err(login_error)?;
    let key = Zeroizing::new(finished.session_key.as_slice().to_vec());
    let finalization = finished.message.serialize().as_slice().to_vec();
    Ok((key, finalization))
}

/// Finish the host side. This function does not take a password.
pub fn login_host_finish(
    pending: PendingHostLogin,
    finalization: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let message = CredentialFinalization::<DeskSuite>::deserialize(finalization)
        .map_err(|_| CryptoError::PasswordProtocol)?;
    let finished = pending
        .state
        .finish(message, ServerLoginParameters::default())
        .map_err(login_error)?;
    Ok(Zeroizing::new(finished.session_key.as_slice().to_vec()))
}

/// Compare two session keys without stopping at the first differing byte.
pub fn keys_match(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0_u8;
    for (one, other) in left.iter().zip(right.iter()) {
        diff |= one ^ other;
    }
    diff == 0
}

fn login_error(err: ProtocolError) -> CryptoError {
    match err {
        ProtocolError::InvalidLoginError => CryptoError::PasswordRejected,
        _ => CryptoError::PasswordProtocol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strong() -> Zeroizing<Vec<u8>> {
        accept_password("correct-horse-battery-staple").expect("strength")
    }

    #[test]
    fn short_and_weak_passwords_are_refused_before_the_protocol() {
        assert!(matches!(
            accept_password("short"),
            Err(CryptoError::PasswordPolicy)
        ));
        assert!(matches!(
            accept_password("password1234"),
            Err(CryptoError::PasswordPolicy)
        ));
        assert_eq!(argon2_parameters(), (19 * 1024, 2, 1));
    }

    #[test]
    fn a_correct_password_matches_and_the_password_is_absent_from_the_messages() {
        let password = strong();
        let setup = HostSetup::generate().expect("setup");
        let (pending, request) = registration_request(&password).expect("request");
        let response = registration_response(&setup, &request).expect("response");
        let upload = registration_upload(pending, &password, &response).expect("upload");
        let record = registration_record(&upload).expect("record");
        for message in [&request, &response, &upload, &record.to_bytes()] {
            assert!(
                !message
                    .windows(password.len())
                    .any(|window| window == password.as_slice()),
                "password bytes appeared in an OPAQUE message"
            );
        }

        let (pending, request) = login_request(&password).expect("login request");
        let (host_pending, response) =
            login_response(&setup, Some(&record), &request).expect("login response");
        let (client_key, finalization) =
            login_finish(pending, &password, &response).expect("client finish");
        let host_key = login_host_finish(host_pending, &finalization).expect("host finish");
        assert!(keys_match(&client_key, &host_key));
        assert!(!request
            .windows(password.len())
            .any(|window| window == password.as_slice()));
        assert!(!response
            .windows(password.len())
            .any(|window| window == password.as_slice()));
        assert!(!finalization
            .windows(password.len())
            .any(|window| window == password.as_slice()));

        let wrong = accept_password("another-correct-horse-battery").expect("other");
        let (pending, request) = login_request(&wrong).expect("wrong request");
        let (host_pending, response) =
            login_response(&setup, Some(&record), &request).expect("wrong response");
        let rejected = login_finish(pending, &wrong, &response);
        assert!(matches!(rejected, Err(CryptoError::PasswordRejected)));
        drop(host_pending);

        let restored = HostSetup::from_bytes(&setup.to_bytes()).expect("restore setup");
        let restored_record = PasswordRecord::from_bytes(&record.to_bytes()).expect("restore");
        let (pending, request) = login_request(&password).expect("again");
        let (host_pending, response) =
            login_response(&restored, Some(&restored_record), &request).expect("again response");
        let (client_key, finalization) =
            login_finish(pending, &password, &response).expect("again finish");
        let host_key = login_host_finish(host_pending, &finalization).expect("again host");
        assert!(keys_match(&client_key, &host_key));
    }
}
