// SPDX-License-Identifier: AGPL-3.0-or-later

//! Flutter bridge.
//!
//! [`api`] is hand-written and forbids unsafe. `frb_generated` is produced by
//! flutter_rust_bridge 2.12.0 and is the only module here that may use unsafe.

#![allow(missing_docs, unused, clippy::all)]

pub mod api;

#[allow(missing_docs, unused, clippy::all, unexpected_cfgs)]
mod frb_generated;
