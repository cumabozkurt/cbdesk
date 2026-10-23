// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hand a dialed QUIC connection to [`HostSession`].
//!
//! An invite ticket is not an input. The controller already used it as a
//! locator. This module reads `Hello`, and the host grants the session only
//! after [`HostSession::approve`] or closes it. An echo probe is not a hello:
//! it is rejected with `PROTOCOL_VIOLATION` and the session stays idle.
//! The production wait is [`wait_for_approval`], which is 30 seconds.
//! A unidirectional stream opened before the grant is `PROTOCOL_VIOLATION`.

use std::future::Future;
use std::net::IpAddr;

use cbdesk_crypto::DeviceId;
use cbdesk_proto::{
    session_supports, AuthMethod, Close, Control, ControlBody, ErrorCode, Hello, HelloAck,
    Permission, PROTO_MAJOR, PROTO_MINOR,
};
use cbdesk_session::{
    CloseReason, HostSession, PeerFacts, RateLimiter, SessionError, APPROVAL_TIMEOUT_MS,
};
use thiserror::Error;

use crate::control::ControlChannel;
use crate::error::NetError;
use crate::quic::QuicConn;

/// What the person at the host decided.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConsentAnswer {
    /// Approve `perms` at `now_ms`. The first-contact delay still applies.
    Approve {
        /// Host clock, in milliseconds.
        now_ms: u64,
        /// Permissions the person checked.
        perms: Vec<Permission>,
        /// The person opted into a high-risk permission on first contact.
        explicit_high_risk: bool,
    },
    /// Deny the prompt.
    Reject {
        /// Host clock, in milliseconds. Used for the prompt rate limit.
        now_ms: u64,
    },
}

/// What the controller received after `Hello`.
#[derive(Clone, Debug, PartialEq)]
pub enum ControllerOutcome {
    /// The host granted a session. This is not screen capture.
    Granted(cbdesk_proto::Granted),
    /// The host closed instead of granting.
    Closed(Close),
}

/// Why the host did not return a live session.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AdmitError {
    /// The QUIC stream failed or the frame was not a control message.
    #[error(transparent)]
    Net(#[from] NetError),
    /// The session state machine refused the transition.
    #[error(transparent)]
    Session(#[from] SessionError),
    /// The person denied the prompt.
    #[error("the person at the host rejected the session")]
    Rejected,
    /// The approval window closed with no answer.
    #[error("the approval window closed without an answer")]
    TimedOut,
    /// The host sent `Close` with this code before a grant.
    #[error("host closed the handshake with {0:?}")]
    Closed(ErrorCode),
}

/// A host session that owns its QUIC connection.
///
/// The connection is not a grant by itself. [`Self::authorize`] follows the
/// session phase. [`Self::send_view_frame`] opens a stream only after View.
#[derive(Debug)]
pub struct HostLink {
    session: HostSession,
    limits: RateLimiter,
    conn: QuicConn,
    remote: DeviceId,
    control: ControlChannel,
}

impl HostLink {
    /// The host policy state.
    pub fn session(&self) -> &HostSession {
        &self.session
    }

    /// The host policy state, for a later local decision such as closing.
    pub fn session_mut(&mut self) -> &mut HostSession {
        &mut self.session
    }

    /// Controller device id proven by the QUIC handshake.
    pub fn remote(&self) -> DeviceId {
        self.remote
    }

    /// Prompt and authentication counters after this handshake.
    pub fn limits(&self) -> &RateLimiter {
        &self.limits
    }

    /// Whether `perm` is in the grant. Fails before the person approves.
    pub fn authorize(&self, perm: Permission) -> Result<(), SessionError> {
        self.session.authorize(perm)
    }

    /// Send one encoded frame. The stream is not opened when View is absent.
    pub async fn send_view_frame(
        &mut self,
        header: &cbdesk_proto::VideoFrameHeader,
        payload: &[u8],
    ) -> Result<(), AdmitError> {
        self.authorize(Permission::View)?;
        crate::media::validate_view_frame(header, payload).map_err(AdmitError::Net)?;
        let mut send = self.conn.open_uni_stream().await.map_err(AdmitError::Net)?;
        crate::media::write_view_frame(&mut send, header, payload)
            .await
            .map_err(AdmitError::Net)?;
        self.session.open_media().map_err(AdmitError::Session)?;
        Ok(())
    }

    /// Read the next control message. A finished controller stream is an error.
    ///
    /// The host waits here after sending a frame so the connection stays up
    /// until [`ControllerGrant::release`].
    pub async fn read_control(&mut self) -> Result<cbdesk_proto::Control, NetError> {
        self.control.read().await
    }

    /// Read one input event. Without Input, the event is dropped and the
    /// session closes with `PROTOCOL_VIOLATION`. Nothing is injected here.
    pub async fn recv_input_event(&mut self) -> Result<cbdesk_proto::InputEvent, AdmitError> {
        let mut recv = self
            .conn
            .accept_uni_stream()
            .await
            .map_err(AdmitError::Net)?;
        if self.authorize(Permission::Input).is_err() {
            return Err(self.violate().await);
        }
        match crate::media::read_input(&mut recv).await {
            Ok(event) => Ok(event),
            Err(error) => {
                let _ = self.violate().await;
                Err(AdmitError::Net(error))
            }
        }
    }

    async fn violate(&mut self) -> AdmitError {
        let _ = self.session.close_with(CloseReason::LocalClose);
        send_close(&mut self.control, ErrorCode::ProtocolViolation).await;
        AdmitError::Closed(ErrorCode::ProtocolViolation)
    }
}

impl Drop for HostLink {
    fn drop(&mut self) {
        // The endpoint and the control stream are the session's connection.
        // Finish the stream on drop so the peer observes a clean end.
        let _ = self.control.finish_send();
        let _ = self.conn.remote_key();
    }
}

/// A handshake that ended without a grant. The connection is kept so a
/// `Close` frame can finish leaving the machine.
#[derive(Debug)]
pub struct FailedAdmit {
    /// Session state after the failure. Idle means authentication never started.
    pub session: HostSession,
    /// Counters after the failure.
    pub limits: RateLimiter,
    /// Why it failed.
    pub error: AdmitError,
    conn: QuicConn,
}

impl FailedAdmit {
    /// Peer key of the connection that failed admission.
    pub fn remote_key(&self) -> [u8; 32] {
        self.conn.remote_key()
    }
}

/// The person's answer, paired with the deadline wait.
pub(crate) struct ConsentWait<A, W> {
    /// Resolves when the person approves or rejects.
    pub answer: A,
    /// Resolves when the approval window should close.
    pub timeout: W,
}

/// Sleep for [`APPROVAL_TIMEOUT_MS`]. This is the production approval wait.
pub async fn wait_for_approval() {
    tokio::time::sleep(std::time::Duration::from_millis(APPROVAL_TIMEOUT_MS)).await;
}

/// Accept one control stream and run consent.
///
/// `answer` is the person's decision. The approval window is
/// [`wait_for_approval`]. A ticket is not consulted.
pub async fn admit_invited<A>(
    conn: QuicConn,
    session: HostSession,
    facts: PeerFacts,
    limits: RateLimiter,
    source: IpAddr,
    now_ms: u64,
    answer: A,
) -> Result<HostLink, FailedAdmit>
where
    A: Future<Output = ConsentAnswer> + Send,
{
    admit_until(
        conn,
        session,
        facts,
        limits,
        source,
        now_ms,
        ConsentWait {
            answer,
            timeout: wait_for_approval(),
        },
    )
    .await
}

pub(crate) async fn admit_until<A, W>(
    conn: QuicConn,
    mut session: HostSession,
    facts: PeerFacts,
    mut limits: RateLimiter,
    source: IpAddr,
    now_ms: u64,
    waits: ConsentWait<A, W>,
) -> Result<HostLink, FailedAdmit>
where
    A: Future<Output = ConsentAnswer> + Send,
    W: Future<Output = ()> + Send,
{
    let mut channel = match ControlChannel::accept(&conn).await {
        Ok(channel) => channel,
        Err(error) => return Err(failed(session, limits, conn, AdmitError::Net(error))),
    };
    let remote = match device_of(&conn) {
        Ok(remote) => remote,
        Err(error) => {
            send_close(&mut channel, ErrorCode::ProtocolViolation).await;
            return Err(failed(session, limits, conn, error));
        }
    };
    if let Err(error) = limits.begin_handshake() {
        send_close(&mut channel, ErrorCode::HostBusy).await;
        return Err(failed(session, limits, conn, AdmitError::Session(error)));
    }
    let result = {
        let mut ctx = Ctx {
            conn: &conn,
            channel: &mut channel,
            session: &mut session,
            limits: &mut limits,
            remote,
            facts,
            source,
            now_ms,
        };
        handshake(&mut ctx, waits).await
    };
    limits.end_handshake();
    match result {
        Ok(()) => Ok(HostLink {
            session,
            limits,
            conn,
            remote,
            control: channel,
        }),
        Err(error) => Err(failed(session, limits, conn, error)),
    }
}

/// Send `hello` and read either `Granted` or `Close`.
///
/// `HelloAck` must ask for interactive approval. Anything else is refused,
/// so a probe or a surprise grant cannot skip consent. The control stream is
/// dropped with the outcome. Use [`open_controller`] to keep it open for a
/// later frame.
pub async fn request_grant(conn: &QuicConn, hello: &Hello) -> Result<ControllerOutcome, NetError> {
    match open_controller(conn, hello).await? {
        ControllerSession::Granted(grant) => {
            let granted = grant.granted.clone();
            Ok(ControllerOutcome::Granted(granted))
        }
        ControllerSession::Closed(close) => Ok(ControllerOutcome::Closed(close)),
    }
}

/// Controller half of a handshake that keeps the control stream.
///
/// A granted session still has to `authorize` each action on the host. This
/// value does not capture the screen.
#[derive(Debug)]
pub struct ControllerGrant {
    /// The grant the host sent.
    pub granted: cbdesk_proto::Granted,
    control: ControlChannel,
}

impl ControllerGrant {
    /// Read the next control message, usually a `Close`.
    pub async fn read(&mut self) -> Result<Control, NetError> {
        self.control.read().await
    }

    /// Finish the control stream after the frame has been read.
    ///
    /// The host keeps the connection until this finish arrives. Dropping the
    /// connection earlier discards a frame the viewer has not read yet.
    pub async fn release(mut self) -> Result<(), NetError> {
        self.control.finish_send()?;
        self.control.until_stopped().await;
        Ok(())
    }
}

impl Drop for ControllerGrant {
    fn drop(&mut self) {
        let _ = self.control.finish_send();
    }
}

/// What [`open_controller`] received after `Hello`.
#[derive(Debug)]
pub enum ControllerSession {
    /// The host granted a session.
    Granted(ControllerGrant),
    /// The host closed instead of granting.
    Closed(Close),
}

/// Send `hello` and keep the control stream when the host grants.
pub async fn open_controller(
    conn: &QuicConn,
    hello: &Hello,
) -> Result<ControllerSession, NetError> {
    let mut channel = ControlChannel::open(conn).await?;
    channel
        .write(&wrap(ControlBody::Hello(hello.clone())))
        .await?;
    match channel.read().await?.body {
        ControlBody::Close(close) => Ok(ControllerSession::Closed(close)),
        ControlBody::HelloAck(ack) if interactive_ack(&ack) => {
            match read_decision(&mut channel).await? {
                ControllerOutcome::Granted(granted) => {
                    Ok(ControllerSession::Granted(ControllerGrant {
                        granted,
                        control: channel,
                    }))
                }
                ControllerOutcome::Closed(close) => Ok(ControllerSession::Closed(close)),
            }
        }
        _ => Err(NetError::BadControl),
    }
}

struct Ctx<'a> {
    conn: &'a QuicConn,
    channel: &'a mut ControlChannel,
    session: &'a mut HostSession,
    limits: &'a mut RateLimiter,
    remote: DeviceId,
    facts: PeerFacts,
    source: IpAddr,
    now_ms: u64,
}

async fn handshake<A, W>(ctx: &mut Ctx<'_>, waits: ConsentWait<A, W>) -> Result<(), AdmitError>
where
    A: Future<Output = ConsentAnswer> + Send,
    W: Future<Output = ()> + Send,
{
    prepare(ctx).await?;
    wait_for_decision(ctx, waits).await
}

async fn wait_for_decision<A, W>(
    ctx: &mut Ctx<'_>,
    waits: ConsentWait<A, W>,
) -> Result<(), AdmitError>
where
    A: Future<Output = ConsentAnswer> + Send,
    W: Future<Output = ()> + Send,
{
    let conn = ctx.conn;
    let decision = tokio::select! {
        biased;
        result = conn.accept_uni_stream() => {
            drop(result);
            Decision::Side
        }
        result = conn.accept_bi_streams() => {
            drop(result);
            Decision::Side
        }
        answer = waits.answer => Decision::Answer(answer),
        () = waits.timeout => Decision::Timeout,
    };
    match decision {
        Decision::Side => Err(refuse_side(ctx).await),
        Decision::Answer(answer) => apply_answer(ctx, answer).await,
        Decision::Timeout => apply_timeout(ctx).await,
    }
}

enum Decision<T> {
    Side,
    Answer(T),
    Timeout,
}

async fn prepare(ctx: &mut Ctx<'_>) -> Result<(), AdmitError> {
    let hello = match read_hello_or_side(ctx).await {
        Ok(hello) => hello,
        Err(error) => {
            ctx.limits
                .record_failure(ctx.remote, ctx.source, ctx.now_ms);
            send_close(ctx.channel, ErrorCode::ProtocolViolation).await;
            return Err(error);
        }
    };
    if let Some(code) = hello_refusal(&hello) {
        send_close(ctx.channel, code).await;
        return Err(AdmitError::Closed(code));
    }
    if ctx.limits.is_limited(&ctx.remote, ctx.source, ctx.now_ms) {
        send_close(ctx.channel, ErrorCode::AuthRateLimited).await;
        return Err(AdmitError::Session(SessionError::RateLimited));
    }
    start_interactive(ctx).await?;
    if ctx.session.approval_deadline_ms().is_none() {
        return refuse_without_prompt(ctx).await;
    }
    if let Err(error) = ctx.limits.begin_prompt(ctx.remote, ctx.now_ms) {
        let _ = ctx.session.close_with(CloseReason::Policy);
        send_close(ctx.channel, ErrorCode::PolicyDenied).await;
        return Err(AdmitError::Session(error));
    }
    let ack = HelloAck {
        proto_major: PROTO_MAJOR,
        proto_minor: PROTO_MINOR,
        auth_required: vec![AuthMethod::Interactive],
        ..HelloAck::default()
    };
    ctx.channel
        .write(&wrap(ControlBody::HelloAck(ack)))
        .await
        .map_err(AdmitError::Net)
}

async fn start_interactive(ctx: &mut Ctx<'_>) -> Result<(), AdmitError> {
    ctx.session
        .start_authentication(AuthMethod::Interactive)
        .map_err(AdmitError::Session)?;
    match ctx.session.authentication_succeeded(&ctx.facts, ctx.now_ms) {
        Ok(()) => Ok(()),
        Err(error) => {
            send_close(ctx.channel, ErrorCode::PolicyDenied).await;
            Err(AdmitError::Session(error))
        }
    }
}

async fn refuse_without_prompt(ctx: &mut Ctx<'_>) -> Result<(), AdmitError> {
    if !matches!(ctx.session.phase(), cbdesk_session::Phase::Closed { .. }) {
        let _ = ctx.session.close_with(CloseReason::Policy);
    }
    send_close(ctx.channel, ErrorCode::PolicyDenied).await;
    Err(AdmitError::Closed(ErrorCode::PolicyDenied))
}

async fn apply_answer(ctx: &mut Ctx<'_>, answer: ConsentAnswer) -> Result<(), AdmitError> {
    match answer {
        ConsentAnswer::Reject { now_ms } => {
            ctx.session.reject().map_err(AdmitError::Session)?;
            ctx.limits.record_reject(ctx.remote, now_ms);
            send_close(ctx.channel, ErrorCode::RejectedByUser).await;
            Err(AdmitError::Rejected)
        }
        ConsentAnswer::Approve {
            now_ms,
            perms,
            explicit_high_risk,
        } => match ctx.session.approve(now_ms, &perms, explicit_high_risk) {
            Ok(()) => write_grant(ctx).await,
            Err(error) => {
                let _ = ctx.session.close_with(CloseReason::Policy);
                send_close(ctx.channel, ErrorCode::PolicyDenied).await;
                Err(AdmitError::Session(error))
            }
        },
    }
}

async fn apply_timeout(ctx: &mut Ctx<'_>) -> Result<(), AdmitError> {
    let deadline = ctx
        .session
        .approval_deadline_ms()
        .ok_or(AdmitError::Session(SessionError::NotAwaitingConsent))?;
    ctx.session
        .expire_unanswered(deadline)
        .map_err(AdmitError::Session)?;
    ctx.limits.record_reject(ctx.remote, deadline);
    send_close(ctx.channel, ErrorCode::TimeoutUserResponse).await;
    Err(AdmitError::TimedOut)
}

async fn write_grant(ctx: &mut Ctx<'_>) -> Result<(), AdmitError> {
    let granted = ctx.session.granted_message().map_err(AdmitError::Session)?;
    ctx.limits.record_success(&ctx.remote);
    ctx.channel
        .write(&wrap(ControlBody::Granted(granted)))
        .await
        .map_err(AdmitError::Net)
}

async fn read_decision(channel: &mut ControlChannel) -> Result<ControllerOutcome, NetError> {
    match channel.read().await?.body {
        ControlBody::Granted(granted) => Ok(ControllerOutcome::Granted(granted)),
        ControlBody::Close(close) => Ok(ControllerOutcome::Closed(close)),
        _ => Err(NetError::BadControl),
    }
}

async fn read_hello_or_side(ctx: &mut Ctx<'_>) -> Result<Hello, AdmitError> {
    let conn = ctx.conn;
    let incoming = tokio::select! {
        biased;
        result = conn.accept_uni_stream() => {
            drop(result);
            Incoming::Side
        }
        result = conn.accept_bi_streams() => {
            drop(result);
            Incoming::Side
        }
        hello = read_hello(ctx.channel) => Incoming::Hello(hello),
    };
    match incoming {
        Incoming::Side => Err(refuse_side(ctx).await),
        Incoming::Hello(hello) => hello,
    }
}

enum Incoming {
    Side,
    Hello(Result<Hello, AdmitError>),
}

async fn refuse_side(ctx: &mut Ctx<'_>) -> AdmitError {
    let _ = ctx.session.close_with(CloseReason::LocalClose);
    ctx.limits
        .record_failure(ctx.remote, ctx.source, ctx.now_ms);
    send_close(ctx.channel, ErrorCode::ProtocolViolation).await;
    AdmitError::Closed(ErrorCode::ProtocolViolation)
}

async fn read_hello(channel: &mut ControlChannel) -> Result<Hello, AdmitError> {
    match channel.read().await.map_err(AdmitError::Net)?.body {
        ControlBody::Hello(hello) => Ok(hello),
        _ => Err(AdmitError::Net(NetError::BadControl)),
    }
}

fn hello_refusal(hello: &Hello) -> Option<ErrorCode> {
    if !session_supports(hello.proto_major) {
        return Some(ErrorCode::ProtoUnsupported);
    }
    if !hello.auth_offer.contains(&AuthMethod::Interactive) {
        return Some(ErrorCode::PolicyDenied);
    }
    None
}

fn interactive_ack(ack: &HelloAck) -> bool {
    ack.proto_major == PROTO_MAJOR && ack.auth_required.as_slice() == [AuthMethod::Interactive]
}

fn device_of(conn: &QuicConn) -> Result<DeviceId, AdmitError> {
    DeviceId::from_bytes(conn.remote_key()).map_err(|_| AdmitError::Net(NetError::BadControl))
}

fn wrap(body: ControlBody) -> Control {
    Control {
        body,
        unknown: cbdesk_proto::UnknownFields::new(),
    }
}

async fn send_close(channel: &mut ControlChannel, code: ErrorCode) {
    let message = wrap(ControlBody::Close(Close {
        code,
        ..Close::default()
    }));
    if channel.write(&message).await.is_ok() {
        let _ = channel.finish_send();
    }
}

fn failed(
    session: HostSession,
    limits: RateLimiter,
    conn: QuicConn,
    error: AdmitError,
) -> FailedAdmit {
    FailedAdmit {
        session,
        limits,
        error,
        conn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_session::APPROVAL_TIMEOUT_MS;

    #[tokio::test(start_paused = true)]
    async fn approval_wait_lasts_thirty_seconds() {
        let start = tokio::time::Instant::now();
        wait_for_approval().await;
        assert_eq!(start.elapsed().as_millis(), u128::from(APPROVAL_TIMEOUT_MS));
    }
}
