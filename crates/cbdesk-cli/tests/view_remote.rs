// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `cbdesk` binary views one synthetic frame after consent.

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cbdesk_capture::{read_barcode, Frame};

#[test]
fn cli_views_a_synthetic_frame_only_after_consent() {
    let started = Instant::now();
    let mut host = cbdesk()
        .args([
            "host",
            "--bind",
            "127.0.0.1:0",
            "--synthetic",
            "--approve",
            "view",
            "--frames",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("host");
    let ticket = read_line(&mut host).expect("ticket line");
    let ticket = ticket
        .trim()
        .strip_prefix("ticket ")
        .expect("ticket prefix")
        .to_string();
    let ppm = std::env::temp_dir().join(format!("cbdesk-view-{}.ppm", std::process::id()));
    let view = cbdesk()
        .args([
            "view",
            "--ticket",
            &ticket,
            "--ppm",
            ppm.to_str().expect("ppm path"),
        ])
        .output()
        .expect("view");
    let host_status = host.wait().expect("host wait");
    let host_err = stderr_of(&mut host);
    assert!(
        view.status.success(),
        "view failed: {}",
        String::from_utf8_lossy(&view.stderr)
    );
    assert!(host_status.success(), "host failed: {host_err}");
    assert!(
        started.elapsed() >= Duration::from_millis(3_000),
        "host granted before the first-contact delay"
    );
    let frame = ppm_luma(&std::fs::read(&ppm).expect("ppm"));
    assert_eq!(read_barcode(&frame).expect("barcode"), (1, 1_000));
    let _ = std::fs::remove_file(&ppm);
}

#[test]
fn cli_records_pointer_input_after_an_input_grant() {
    let mut host = cbdesk()
        .args([
            "host",
            "--bind",
            "127.0.0.1:0",
            "--synthetic",
            "--approve",
            "view,input",
            "--frames",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("host");
    let ticket = read_line(&mut host).expect("ticket line");
    let ticket = ticket
        .trim()
        .strip_prefix("ticket ")
        .expect("ticket prefix")
        .to_string();
    let ppm = std::env::temp_dir().join(format!("cbdesk-input-{}.ppm", std::process::id()));
    let view = cbdesk()
        .args([
            "view",
            "--ticket",
            &ticket,
            "--ppm",
            ppm.to_str().expect("ppm path"),
            "--pointer",
            "12,34",
            "--key",
            "enter",
        ])
        .output()
        .expect("view");
    let input = read_line(&mut host).expect("input line");
    let key = read_line(&mut host).expect("key line");
    let host_status = host.wait().expect("host wait");
    assert!(
        view.status.success(),
        "view failed: {}",
        String::from_utf8_lossy(&view.stderr)
    );
    assert!(
        host_status.success(),
        "host failed: {}",
        stderr_of(&mut host)
    );
    assert_eq!(input.trim(), "input pointer 12 34");
    assert_eq!(key.trim(), "input key 57344 1");
    let _ = std::fs::remove_file(&ppm);
}

#[test]
fn cli_stores_a_put_file_after_a_file_write_grant() {
    let root = std::env::temp_dir().join(format!("cbdesk-files-{}", std::process::id()));
    let src_dir = std::env::temp_dir().join(format!("cbdesk-put-{}", std::process::id()));
    std::fs::create_dir_all(&src_dir).expect("src");
    let src = src_dir.join("note.txt");
    std::fs::write(&src, b"hello").expect("src file");
    let mut host = cbdesk()
        .args([
            "host",
            "--bind",
            "127.0.0.1:0",
            "--synthetic",
            "--approve",
            "file_write",
            "--allow-high-risk",
            "--files-root",
            root.to_str().expect("root"),
            "--frames",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("host");
    let ticket = read_line(&mut host).expect("ticket line");
    let ticket = ticket
        .trim()
        .strip_prefix("ticket ")
        .expect("ticket prefix")
        .to_string();
    let view = cbdesk()
        .args([
            "view",
            "--ticket",
            &ticket,
            "--put",
            src.to_str().expect("put"),
        ])
        .output()
        .expect("view");
    let stored = read_line(&mut host).expect("file line");
    let host_status = host.wait().expect("host wait");
    assert!(
        view.status.success(),
        "view failed: {}",
        String::from_utf8_lossy(&view.stderr)
    );
    assert!(
        host_status.success(),
        "host failed: {}",
        stderr_of(&mut host)
    );
    assert_eq!(stored.trim(), "file complete note.txt");
    assert_eq!(
        std::fs::read(root.join("note.txt")).expect("stored"),
        b"hello"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&src_dir);
}

#[test]
fn cli_plays_a_tone_after_an_audio_grant() {
    let mut host = cbdesk()
        .args([
            "host",
            "--bind",
            "127.0.0.1:0",
            "--synthetic",
            "--approve",
            "audio",
            "--tone",
            "--frames",
            "1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("host");
    let ticket = read_line(&mut host).expect("ticket line");
    let ticket = ticket
        .trim()
        .strip_prefix("ticket ")
        .expect("ticket prefix")
        .to_string();
    let view = cbdesk()
        .args(["view", "--ticket", &ticket, "--audio"])
        .output()
        .expect("view");
    let host_status = host.wait().expect("host wait");
    assert!(
        view.status.success(),
        "view failed: {}",
        String::from_utf8_lossy(&view.stderr)
    );
    assert!(
        host_status.success(),
        "host failed: {}",
        stderr_of(&mut host)
    );
    let stdout = String::from_utf8_lossy(&view.stdout);
    assert!(
        stdout.lines().any(|line| line.starts_with("audio ")),
        "viewer stdout: {stdout}"
    );
}

#[test]
fn cli_browses_a_lan_host_and_reads_the_barcode() {
    let _lock = hold_discovery_lock();
    let started = Instant::now();
    let mut host = cbdesk()
        .args([
            "host",
            "--bind",
            "0.0.0.0:0",
            "--synthetic",
            "--approve",
            "view",
            "--frames",
            "1",
            "--lan",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("host");
    let ticket = read_line(&mut host).expect("ticket line");
    assert!(ticket.trim().starts_with("ticket "), "{ticket}");
    let ppm = std::env::temp_dir().join(format!("cbdesk-lan-{}.ppm", std::process::id()));
    let view = cbdesk()
        .args(["view", "--browse", "--ppm", ppm.to_str().expect("ppm path")])
        .output()
        .expect("view");
    let view_err = String::from_utf8_lossy(&view.stderr).to_string();
    if !view.status.success() {
        let _ = host.kill();
    }
    let host_status = host.wait().expect("host wait");
    let host_err = stderr_of(&mut host);
    if !view.status.success() && same_keychain_device(&host_err, &view_err) {
        eprintln!("LAN browse skipped: both processes loaded one keychain identity");
        return;
    }
    assert!(view.status.success(), "view failed: {view_err}\n{host_err}");
    assert!(host_status.success(), "host failed: {host_err}");
    assert!(
        started.elapsed() >= Duration::from_millis(3_000),
        "host granted before the first-contact delay"
    );
    let frame = ppm_luma(&std::fs::read(&ppm).expect("ppm"));
    assert_eq!(read_barcode(&frame).expect("barcode"), (1, 1_000));
    let _ = std::fs::remove_file(&ppm);
}

fn cbdesk() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cbdesk"));
    command.env("CBDESK_EPHEMERAL_IDENTITY", "1");
    command
}

fn hold_discovery_lock() -> std::net::TcpListener {
    let started = Instant::now();
    loop {
        if let Ok(listener) = std::net::TcpListener::bind("127.0.0.1:22953") {
            return listener;
        }
        if started.elapsed() > Duration::from_secs(30) {
            panic!("timed out waiting for the discovery test lock");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn same_keychain_device(host_err: &str, view_err: &str) -> bool {
    match (keychain_id(host_err), keychain_id(view_err)) {
        (Some(host_id), Some(view_id)) => host_id == view_id,
        _ => false,
    }
}

fn keychain_id(text: &str) -> Option<&str> {
    for line in text.lines() {
        let mut parts = line.split_whitespace();
        if parts.next() == Some("identity") {
            let id = parts.next()?;
            if parts.next() == Some("keychain") {
                return Some(id);
            }
        }
    }
    None
}

fn read_line(child: &mut std::process::Child) -> std::io::Result<String> {
    // One byte at a time. A `BufReader` would pull the next line into its
    // buffer and drop those bytes when this function returns.
    let stdout = child.stdout.as_mut().expect("stdout");
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        let read = std::io::Read::read(stdout, &mut byte)?;
        if read == 0 {
            break;
        }
        line.push(byte[0]);
        if byte[0] == b'\n' {
            break;
        }
    }
    Ok(String::from_utf8_lossy(&line).into_owned())
}

fn stderr_of(child: &mut std::process::Child) -> String {
    let Some(stderr) = child.stderr.take() else {
        return String::new();
    };
    let mut text = String::new();
    let _ = std::io::Read::read_to_string(&mut std::io::BufReader::new(stderr), &mut text);
    text
}

fn ppm_luma(bytes: &[u8]) -> Frame {
    let header = b"P6\n";
    assert!(bytes.starts_with(header), "ppm magic");
    let rest = &bytes[header.len()..];
    let newline = rest.iter().position(|byte| *byte == b'\n').expect("size");
    let size = std::str::from_utf8(&rest[..newline]).expect("size text");
    let (width, height) = size.split_once(' ').expect("pair");
    let width: u32 = width.parse().expect("width");
    let height: u32 = height.parse().expect("height");
    let after = &rest[newline + 1..];
    let max = after.iter().position(|byte| *byte == b'\n').expect("max");
    let pixels_rgb = &after[max + 1..];
    assert_eq!(pixels_rgb.len(), width as usize * height as usize * 3);
    let mut pixels = Vec::with_capacity(width as usize * height as usize);
    for chunk in pixels_rgb.chunks_exact(3) {
        assert_eq!(chunk[0], chunk[1]);
        assert_eq!(chunk[1], chunk[2]);
        pixels.push(chunk[0]);
    }
    Frame {
        width,
        height,
        pixels,
    }
}
