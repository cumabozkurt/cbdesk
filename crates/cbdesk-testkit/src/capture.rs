// SPDX-License-Identifier: AGPL-3.0-or-later

//! Re-export of the barcode capturer. The implementation lives in
//! `cbdesk-capture` so the CLI and the codec can use it too.

pub use cbdesk_capture::{read_barcode, Frame, SyntheticCapturer, BARCODE_WIDTH};
