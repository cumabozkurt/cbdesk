// SPDX-License-Identifier: AGPL-3.0-or-later

//! `cbdesk net-check` and `cbdesk doctor`.
//!
//! The report describes this machine. It does not claim a NAT type unless a
//! mapping was observed, and the local report never observes one. `--loopback`
//! connects two endpoints on this computer and says that a direct path there
//! is not a hole punch.

use cbdesk_net::{local_report, measure_loopback_paths, NatClass, NetCheckReport};

use crate::error::CliError;

/// Options for the diagnostic command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NetCheckArgs {
    /// Print JSON instead of the text report.
    pub json: bool,
    /// Omit local source addresses.
    pub anonymous: bool,
    /// Also connect two loopback endpoints and record the path.
    pub loopback: bool,
}

/// Print the report to stdout.
pub async fn run_net_check(args: &NetCheckArgs) -> Result<(), CliError> {
    let report = local_report(0);
    let loopback = if args.loopback {
        Some(measure_loopback_paths().await.map_err(cli)?)
    } else {
        None
    };
    if args.json {
        println!(
            "{}",
            json_report(&report, args.anonymous, loopback.as_ref())
        );
    } else {
        println!(
            "{}",
            text_report(&report, args.anonymous, loopback.as_ref())
        );
    }
    Ok(())
}

fn text_report(
    report: &NetCheckReport,
    anonymous: bool,
    loopback: Option<&cbdesk_net::LoopbackPathReport>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("udp ipv4: {}", yes(report.udp_ipv4)));
    lines.push(format!("udp ipv6: {}", yes(report.udp_ipv6)));
    if anonymous {
        lines.push("ipv4 source: omitted".to_string());
        lines.push("ipv6 source: omitted".to_string());
    } else {
        lines.push(format!("ipv4 source: {}", addr(report.ipv4_source)));
        lines.push(format!("ipv6 source: {}", addr(report.ipv6_source)));
    }
    lines.push(format!("cgnat: {}", yes(report.cgnat)));
    lines.push(format!("address class: {}", class_name(report.nat_class)));
    lines.push("symmetric nat: unmeasured".to_string());
    lines.push(format!("upnp: {}", report.upnp));
    lines.push(format!("dht: {}", report.dht));
    lines.push(format!("hole punch: {}", report.hole_punch));
    lines.push(
        "mapping rule: two external ports for one local port is endpoint-dependent and usually needs a relay"
            .to_string(),
    );
    if let Some(loopback) = loopback {
        lines.push(format!("loopback direct paths: {}", loopback.direct_paths));
        lines.push(format!("loopback relay paths: {}", loopback.relay_paths));
        lines.push(format!(
            "loopback selected direct: {}",
            yes(loopback.selected_direct)
        ));
        lines.push(format!("loopback note: {}", loopback.note));
    }
    lines.join("\n")
}

fn json_report(
    report: &NetCheckReport,
    anonymous: bool,
    loopback: Option<&cbdesk_net::LoopbackPathReport>,
) -> String {
    let ipv4 = if anonymous {
        "null".to_string()
    } else {
        json_addr(report.ipv4_source)
    };
    let ipv6 = if anonymous {
        "null".to_string()
    } else {
        json_addr(report.ipv6_source)
    };
    let loopback_json = match loopback {
        Some(item) => format!(
            "{{\"direct_paths\":{},\"relay_paths\":{},\"selected_direct\":{},\"note\":{}}}",
            item.direct_paths,
            item.relay_paths,
            item.selected_direct,
            json_string(item.note)
        ),
        None => "null".to_string(),
    };
    format!(
        "{{\"udp_ipv4\":{},\"udp_ipv6\":{},\"ipv4_source\":{},\"ipv6_source\":{},\"cgnat\":{},\"address_class\":{},\"symmetric_nat\":\"unmeasured\",\"upnp\":{},\"dht\":{},\"hole_punch\":{},\"loopback\":{}}}",
        report.udp_ipv4,
        report.udp_ipv6,
        ipv4,
        ipv6,
        report.cgnat,
        json_string(class_name(report.nat_class)),
        json_string(report.upnp),
        json_string(report.dht),
        json_string(report.hole_punch),
        loopback_json
    )
}

fn class_name(class: NatClass) -> &'static str {
    match class {
        NatClass::UdpBlocked => "udp-blocked",
        NatClass::Cgnat => "cgnat",
        NatClass::OpenInternet => "open-internet",
        NatClass::PrivateUnmeasured => "private-unmeasured",
    }
}

fn yes(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn addr(value: Option<std::net::IpAddr>) -> String {
    value
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn json_addr(value: Option<std::net::IpAddr>) -> String {
    match value {
        Some(ip) => json_string(&ip.to_string()),
        None => "null".to_string(),
    }
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

fn cli(err: impl std::fmt::Display) -> CliError {
    CliError::new(err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_report_says_symmetric_nat_is_unmeasured() {
        let report = local_report(0);
        let text = text_report(&report, true, None);
        assert!(text.contains("symmetric nat: unmeasured"));
        assert!(text.contains("ipv4 source: omitted"));
        assert!(!text.contains("hole punch: measured"));
    }
}
