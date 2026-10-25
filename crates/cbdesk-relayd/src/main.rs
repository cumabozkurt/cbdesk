// SPDX-License-Identifier: AGPL-3.0-or-later

//! Print a loopback relay URL and serve until interrupted.
//!
//! The URL is the only line on stdout. Limits are on stderr.

use std::process::ExitCode;

use cbdesk_relayd::{parse_bind, spawn, RelayError};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bind = match parse_bind(&args) {
        Ok(bind) => bind,
        Err(RelayError::Help) => {
            eprintln!("{}", RelayError::Help);
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            eprintln!("cbdesk-relayd: {err}");
            return ExitCode::from(2);
        }
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("cbdesk-relayd: {err}");
            return ExitCode::from(1);
        }
    };
    let result = runtime.block_on(async move {
        let relay = spawn(bind).await?;
        println!("{}", relay.https_url());
        eprintln!("cbdesk-relayd: loopback only, self-signed certificate, not a NAT measurement");
        let _ = tokio::signal::ctrl_c().await;
        relay.shutdown().await
    });
    if let Err(err) = result {
        eprintln!("cbdesk-relayd: {err}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
