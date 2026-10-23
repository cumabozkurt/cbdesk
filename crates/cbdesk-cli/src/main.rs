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
        Command::Host(host) => run_async(async move { cbdesk_cli::serve(&host).await }),
        Command::View(view) if view.window => cbdesk_cli::view_window(&view),
        Command::View(view) => run_async(async move { cbdesk_cli::view_remote(&view).await }),
    }
}

fn run_async<F>(future: F) -> Result<(), CliError>
where
    F: std::future::Future<Output = Result<(), CliError>> + Send + 'static,
{
    // Windows gives the process main thread a 1 MiB stack. VP8 encode and the
    // QUIC runtime need more than that, so the runtime lives on an 8 MiB thread.
    std::thread::Builder::new()
        .name("cbdesk-runtime".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(move || runtime()?.block_on(future))
        .map_err(|err| CliError::new(err.to_string()))?
        .join()
        .unwrap_or_else(|_| Err(CliError::new("runtime thread panicked")))
}

fn runtime() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .map_err(|err| CliError::new(err.to_string()))
}
