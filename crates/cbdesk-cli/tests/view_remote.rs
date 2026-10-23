// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `cbdesk` binary views one synthetic frame after consent.

use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use cbdesk_capture::{read_barcode, Frame};

#[test]
fn cli_views_a_synthetic_frame_only_after_consent() {
    let started = Instant::now();
    let mut host = Command::new(env!("CARGO_BIN_EXE_cbdesk"))
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
    let view = Command::new(env!("CARGO_BIN_EXE_cbdesk"))
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
    let mut host = Command::new(env!("CARGO_BIN_EXE_cbdesk"))
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
    let view = Command::new(env!("CARGO_BIN_EXE_cbdesk"))
        .args([
            "view",
            "--ticket",
            &ticket,
            "--ppm",
            ppm.to_str().expect("ppm path"),
            "--pointer",
            "12,34",
        ])
        .output()
        .expect("view");
    let input = read_line(&mut host).expect("input line");
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
    let _ = std::fs::remove_file(&ppm);
}

fn read_line(child: &mut std::process::Child) -> std::io::Result<String> {
    let stdout = child.stdout.as_mut().expect("stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    Ok(line)
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
