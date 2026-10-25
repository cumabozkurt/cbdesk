// SPDX-License-Identifier: AGPL-3.0-or-later

//! Protocol enumerations.
//!
//! Unknown numeric values are preserved. They are never treated as a known
//! permission or capability.

/// Operating system reported in `Hello`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    /// The peer did not say.
    Unspecified,
    /// Microsoft Windows.
    Windows,
    /// Apple macOS.
    Macos,
    /// Linux, without claiming a particular compositor.
    Linux,
    /// Android.
    Android,
    /// Apple iOS. Controllers only; this build cannot host a session there.
    Ios,
    /// A future browser client.
    Web,
    /// A numeric value this version does not know.
    Unknown(i32),
}

/// A feature the peer can use during capability negotiation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Capability {
    /// Unset.
    Unspecified,
    /// Software VP8 video.
    VideoVp8,
    /// Software VP9 video.
    VideoVp9,
    /// AV1 video.
    VideoAv1,
    /// Hardware or OS H.264. Not a software encoder shipped by CBDesk.
    VideoH264,
    /// Hardware or OS H.265. Not a software encoder shipped by CBDesk.
    VideoH265,
    /// Opus audio.
    AudioOpus,
    /// Keyboard and pointer input.
    Input,
    /// Clipboard synchronization.
    Clipboard,
    /// File transfer.
    File,
    /// Text chat.
    Chat,
    /// More than one display.
    MultiDisplay,
    /// IPv6 candidates.
    Ipv6,
    /// Relay fallback.
    Relay,
    /// A numeric value this version does not know.
    Unknown(i32),
}

/// An authorization method the host may require.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AuthMethod {
    /// Unset.
    Unspecified,
    /// The controller key is already in the host trust store.
    TrustedKey,
    /// Short-code PAKE.
    PakeCode,
    /// One-time password.
    OneTimePassword,
    /// OPAQUE permanent password. The protocol lives in `cbdesk-crypto`. The CLI does not enroll one.
    OpaquePassword,
    /// Organization certificate. Not implemented in this milestone.
    OrgCert,
    /// TOTP second factor.
    Totp,
    /// A person at the host must approve the session.
    Interactive,
    /// A numeric value this version does not know.
    Unknown(i32),
}

/// A capability the host can grant for one session.
///
/// There is no permission that hides the session. A hidden or stealth mode is
/// not representable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    /// Unset.
    Unspecified,
    /// See the screen.
    View,
    /// Send keyboard and pointer input.
    Input,
    /// Synchronize the clipboard.
    Clipboard,
    /// Read files.
    FileRead,
    /// Write or delete files.
    FileWrite,
    /// Hear the host's audio.
    Audio,
    /// Send microphone audio to the host.
    Mic,
    /// Restart the host.
    Restart,
    /// Power off, sleep, or sign out.
    Power,
    /// Open a terminal.
    Terminal,
    /// Forward a TCP port.
    Tunnel,
    /// Record the session. The host is told when this is granted.
    Record,
    /// Blank the host's physical display. A visible notice still stays up.
    PrivacyMode,
    /// Block local input on the host.
    BlockInput,
    /// Print on the controller's printer.
    Print,
    /// Read non-secret device inventory.
    Sysinfo,
    /// Run a command. Organization or explicitly authorized sessions only.
    Script,
    /// A numeric value this version does not know.
    Unknown(i32),
}

/// Why a session closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    /// Unset.
    Unspecified,
    /// Authentication failed.
    AuthFailed,
    /// Too many failed attempts.
    AuthRateLimited,
    /// The person at the host denied the request.
    RejectedByUser,
    /// The person at the host did not answer in time.
    TimeoutUserResponse,
    /// The controller is on the host's block list.
    PeerBlocked,
    /// Host policy refused the request.
    PolicyDenied,
    /// An organization certificate was revoked.
    CertRevoked,
    /// The peers do not share a protocol version.
    ProtoUnsupported,
    /// The host is already at its session limit.
    HostBusy,
    /// A permission was removed during the session.
    PermissionRevoked,
    /// The session passed its time limit.
    SessionExpired,
    /// The host is shutting down.
    HostShutdown,
    /// The host session is locked in a way that cannot be driven remotely.
    SessionLocked,
    /// The peer broke a protocol rule, such as opening a channel too early.
    ProtocolViolation,
    /// A numeric value this version does not know.
    Unknown(i32),
}

/// Video codec identifier carried beside a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Codec {
    /// Unset.
    Unspecified,
    /// VP8.
    Vp8,
    /// VP9.
    Vp9,
    /// AV1.
    Av1,
    /// H.264 via the operating system or GPU.
    H264,
    /// H.265 via the operating system or GPU.
    H265,
    /// A numeric value this version does not know.
    Unknown(i32),
}

/// What an invite ticket is for. A ticket never grants access by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InvitePurpose {
    /// Unset.
    Unspecified,
    /// Find a host for a remote-desktop session.
    Session,
    /// Pair a device into "my devices".
    PairDevice,
    /// Join an organization. The certificate check is separate.
    JoinOrg,
    /// A numeric value this version does not know.
    Unknown(i32),
}

macro_rules! wire_enum {
    ($ty:ty, $($variant:ident => $value:expr),+ $(,)?) => {
        impl $ty {
            /// Decode a protobuf enum value, preserving unknown numbers.
            pub fn from_i32(value: i32) -> Self {
                match value {
                    $($value => Self::$variant,)+
                    other => Self::Unknown(other),
                }
            }

            /// Encode this value, including unknown numbers.
            pub fn as_i32(self) -> i32 {
                match self {
                    $(Self::$variant => $value,)+
                    Self::Unknown(value) => value,
                }
            }
        }
    };
}

wire_enum!(
    Platform,
    Unspecified => 0,
    Windows => 1,
    Macos => 2,
    Linux => 3,
    Android => 4,
    Ios => 5,
    Web => 6,
);
wire_enum!(
    Capability,
    Unspecified => 0,
    VideoVp8 => 1,
    VideoVp9 => 2,
    VideoAv1 => 3,
    VideoH264 => 4,
    VideoH265 => 5,
    AudioOpus => 6,
    Input => 7,
    Clipboard => 8,
    File => 9,
    Chat => 10,
    MultiDisplay => 11,
    Ipv6 => 12,
    Relay => 13,
);
wire_enum!(
    AuthMethod,
    Unspecified => 0,
    TrustedKey => 1,
    PakeCode => 2,
    OneTimePassword => 3,
    OpaquePassword => 4,
    OrgCert => 5,
    Totp => 6,
    Interactive => 7,
);
wire_enum!(
    Permission,
    Unspecified => 0,
    View => 1,
    Input => 2,
    Clipboard => 3,
    FileRead => 4,
    FileWrite => 5,
    Audio => 6,
    Mic => 7,
    Restart => 8,
    Power => 9,
    Terminal => 10,
    Tunnel => 11,
    Record => 12,
    PrivacyMode => 13,
    BlockInput => 14,
    Print => 15,
    Sysinfo => 16,
    Script => 17,
);
wire_enum!(
    ErrorCode,
    Unspecified => 0,
    AuthFailed => 1,
    AuthRateLimited => 2,
    RejectedByUser => 3,
    TimeoutUserResponse => 4,
    PeerBlocked => 5,
    PolicyDenied => 6,
    CertRevoked => 7,
    ProtoUnsupported => 8,
    HostBusy => 9,
    PermissionRevoked => 10,
    SessionExpired => 11,
    HostShutdown => 12,
    SessionLocked => 13,
    ProtocolViolation => 14,
);
wire_enum!(
    Codec,
    Unspecified => 0,
    Vp8 => 1,
    Vp9 => 2,
    Av1 => 3,
    H264 => 4,
    H265 => 5,
);
wire_enum!(
    InvitePurpose,
    Unspecified => 0,
    Session => 1,
    PairDevice => 2,
    JoinOrg => 3,
);

impl Permission {
    /// Bit index used by the session permission set, if this value can be granted.
    pub fn bit(self) -> Option<u8> {
        let value = self.as_i32();
        if (1..=17).contains(&value) {
            u8::try_from(value).ok()
        } else {
            None
        }
    }

    /// Permissions that stay off on a first contact unless the host checks them.
    pub fn is_high_risk(self) -> bool {
        matches!(
            self,
            Self::FileWrite | Self::Terminal | Self::BlockInput | Self::PrivacyMode
        )
    }
}

impl ErrorCode {
    /// Every known code, in wire order. Unknown values are not included.
    pub fn known() -> &'static [Self] {
        &KNOWN_ERRORS
    }

    /// Stable message catalog key, shared by the Turkish and English catalogs.
    pub fn message_key(self) -> &'static str {
        match self {
            Self::Unspecified => "error.unspecified",
            Self::AuthFailed => "error.auth_failed",
            Self::AuthRateLimited => "error.auth_rate_limited",
            Self::RejectedByUser => "error.rejected_by_user",
            Self::TimeoutUserResponse => "error.timeout_user_response",
            Self::PeerBlocked => "error.peer_blocked",
            Self::PolicyDenied => "error.policy_denied",
            Self::CertRevoked => "error.cert_revoked",
            Self::ProtoUnsupported => "error.proto_unsupported",
            Self::HostBusy => "error.host_busy",
            Self::PermissionRevoked => "error.permission_revoked",
            Self::SessionExpired => "error.session_expired",
            Self::HostShutdown => "error.host_shutdown",
            Self::SessionLocked => "error.session_locked",
            Self::ProtocolViolation => "error.protocol_violation",
            Self::Unknown(_) => "error.unknown",
        }
    }

    /// Catalog key for the short "what to do" hint.
    pub fn hint_key(self) -> &'static str {
        match self {
            Self::Unspecified => "error.unspecified.hint",
            Self::AuthFailed => "error.auth_failed.hint",
            Self::AuthRateLimited => "error.auth_rate_limited.hint",
            Self::RejectedByUser => "error.rejected_by_user.hint",
            Self::TimeoutUserResponse => "error.timeout_user_response.hint",
            Self::PeerBlocked => "error.peer_blocked.hint",
            Self::PolicyDenied => "error.policy_denied.hint",
            Self::CertRevoked => "error.cert_revoked.hint",
            Self::ProtoUnsupported => "error.proto_unsupported.hint",
            Self::HostBusy => "error.host_busy.hint",
            Self::PermissionRevoked => "error.permission_revoked.hint",
            Self::SessionExpired => "error.session_expired.hint",
            Self::HostShutdown => "error.host_shutdown.hint",
            Self::SessionLocked => "error.session_locked.hint",
            Self::ProtocolViolation => "error.protocol_violation.hint",
            Self::Unknown(_) => "error.unknown.hint",
        }
    }
}

const KNOWN_ERRORS: [ErrorCode; 15] = [
    ErrorCode::Unspecified,
    ErrorCode::AuthFailed,
    ErrorCode::AuthRateLimited,
    ErrorCode::RejectedByUser,
    ErrorCode::TimeoutUserResponse,
    ErrorCode::PeerBlocked,
    ErrorCode::PolicyDenied,
    ErrorCode::CertRevoked,
    ErrorCode::ProtoUnsupported,
    ErrorCode::HostBusy,
    ErrorCode::PermissionRevoked,
    ErrorCode::SessionExpired,
    ErrorCode::HostShutdown,
    ErrorCode::SessionLocked,
    ErrorCode::ProtocolViolation,
];
