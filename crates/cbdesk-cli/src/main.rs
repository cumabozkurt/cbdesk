// SPDX-License-Identifier: AGPL-3.0-or-later

//! `cbdesk` host and view commands.
//!
//! The viewer window runs on this thread. macOS will not open a window from
//! a background runtime, so the process is not `#[tokio::main]`.

use cbdesk_cli::{parse, CliError, Command};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(err) = dispatch(&args) {
        eprintln!("cbdesk: {err}");
        std::process::exit(1);
    }
}

fn dispatch(args: &[String]) -> Result<(), CliError> {
    match parse(args)? {
        Command::Host(host) => runtime()?.block_on(cbdesk_cli::serve(&host)),
        Command::View(view) if view.window => cbdesk_cli::view_window(&view),
        Command::View(view) => runtime()?.block_on(cbdesk_cli::view_remote(&view)),
    }
}

fn runtime() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| CliError::new(err.to_string()))
}
