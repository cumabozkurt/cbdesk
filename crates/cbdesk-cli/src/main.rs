// SPDX-License-Identifier: AGPL-3.0-or-later

//! `cbdesk` host and view commands.

use cbdesk_cli::{parse, CliError, Command};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(err) = dispatch(&args).await {
        eprintln!("cbdesk: {err}");
        std::process::exit(1);
    }
}

async fn dispatch(args: &[String]) -> Result<(), CliError> {
    match parse(args)? {
        Command::Host(host) => cbdesk_cli::serve(&host).await,
        Command::View(view) => cbdesk_cli::view_remote(&view).await,
    }
}
