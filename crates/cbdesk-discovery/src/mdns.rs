// SPDX-License-Identifier: AGPL-3.0-or-later

//! DNS-SD bytes for `_cbdesk._udp.local.`.
//!
//! The TXT body is [`MdnsTxt::character_strings`]. The instance label is the
//! device id in z-base-32, so a browse can recover the full key and then check
//! it against the 16-byte `pk` prefix. A discovered record is not a grant.

use cbdesk_crypto::DeviceId;

use crate::error::DiscoveryError;
use crate::lan::{MdnsTxt, MDNS_SERVICE_TYPE};

const PTR: u16 = 12;
const TXT: u16 = 16;
const SRV: u16 = 33;
const CLASS_IN: u16 = 1;
const TTL: u32 = 120;
const MAX_PACKET: usize = 2048;
const MAX_RECORDS: u32 = 32;

/// A response that advertises one device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MdnsAnnouncement {
    /// Full device id, taken from the instance label.
    pub device_id: DeviceId,
    /// TXT fields. `port` matches the SRV port.
    pub txt: MdnsTxt,
}

/// A CBDesk mDNS message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MdnsMessage {
    /// A browse for [`MDNS_SERVICE_TYPE`].
    Query,
    /// An unsolicited or solicited advertisement.
    Announcement(MdnsAnnouncement),
}

/// One question for the CBDesk service PTR.
pub fn encode_browse_query() -> Result<Vec<u8>, DiscoveryError> {
    let mut out = Vec::with_capacity(64);
    write_header(&mut out, 0, 1, 0, 0, 0);
    write_question(&mut out, MDNS_SERVICE_TYPE, PTR)?;
    Ok(out)
}

/// Unsolicited response built from the tested TXT fields.
pub fn encode_announcement(
    device_id: &DeviceId,
    quic_port: u16,
    name: Option<&str>,
) -> Result<Vec<u8>, DiscoveryError> {
    let name = normalize_name(name)?;
    let txt = MdnsTxt::from_device(device_id, quic_port, name);
    let instance = instance_name(device_id)?;
    let host = host_name(device_id)?;
    let mut ptr_data = Vec::new();
    write_name(&mut ptr_data, &instance)?;
    let mut srv_data = Vec::new();
    push_u16(&mut srv_data, 0);
    push_u16(&mut srv_data, 0);
    push_u16(&mut srv_data, quic_port);
    write_name(&mut srv_data, &host)?;
    let txt_data = write_txt(&txt.character_strings())?;

    let mut out = Vec::with_capacity(256);
    // QR | AA. No n0 names are written into this packet.
    write_header(&mut out, 0x8400, 0, 1, 0, 2);
    write_rr(&mut out, MDNS_SERVICE_TYPE, PTR, &ptr_data)?;
    write_rr(&mut out, &instance, SRV, &srv_data)?;
    write_rr(&mut out, &instance, TXT, &txt_data)?;
    Ok(out)
}

/// Decode a query or an announcement. Other DNS messages are rejected.
pub fn decode_mdns(packet: &[u8]) -> Result<MdnsMessage, DiscoveryError> {
    if packet.len() < 12 || packet.len() > MAX_PACKET {
        return Err(DiscoveryError::BadMdns);
    }
    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    if flags & 0x7800 != 0 || flags & 0x0200 != 0 {
        return Err(DiscoveryError::BadMdns);
    }
    let counts = [
        u16::from_be_bytes([packet[4], packet[5]]),
        u16::from_be_bytes([packet[6], packet[7]]),
        u16::from_be_bytes([packet[8], packet[9]]),
        u16::from_be_bytes([packet[10], packet[11]]),
    ];
    let total = counts.iter().map(|count| u32::from(*count)).sum::<u32>();
    if total > MAX_RECORDS {
        return Err(DiscoveryError::BadMdns);
    }
    let mut pos = 12;
    let mut saw_service_ptr = false;
    for _ in 0..counts[0] {
        let (name, next) = read_name(packet, pos, 0)?;
        pos = question_end(packet, next)?;
        if eq_name(&name, MDNS_SERVICE_TYPE) {
            saw_service_ptr = true;
        }
    }
    let mut records = Vec::new();
    let record_count = usize::from(counts[1]) + usize::from(counts[2]) + usize::from(counts[3]);
    for _ in 0..record_count {
        let (record, next) = read_rr(packet, pos)?;
        pos = next;
        records.push(record);
    }
    if pos != packet.len() {
        return Err(DiscoveryError::BadMdns);
    }
    if flags & 0x8000 == 0 {
        if saw_service_ptr {
            return Ok(MdnsMessage::Query);
        }
        return Err(DiscoveryError::BadMdns);
    }
    Ok(MdnsMessage::Announcement(assemble(packet, &records)?))
}

struct Record {
    name: Vec<String>,
    typ: u16,
    at: usize,
    len: usize,
}

fn assemble(packet: &[u8], records: &[Record]) -> Result<MdnsAnnouncement, DiscoveryError> {
    let mut device_id = None;
    for record in records {
        if record.typ != PTR || !eq_name(&record.name, MDNS_SERVICE_TYPE) {
            continue;
        }
        let (target, end) = read_name(packet, record.at, 0)?;
        if end != record.at + record.len {
            return Err(DiscoveryError::BadMdns);
        }
        device_id = Some(device_from_instance(&target)?);
        break;
    }
    let device_id = device_id.ok_or(DiscoveryError::BadMdns)?;
    let instance = instance_name(&device_id)?;
    let txt = txt_for_instance(packet, records, &instance)?;
    let srv_port = srv_port(packet, records, &instance)?;
    if txt.port != srv_port {
        return Err(DiscoveryError::BadMdns);
    }
    let expected = MdnsTxt::from_device(&device_id, srv_port, txt.name.clone());
    if txt.pk_hex16 != expected.pk_hex16 || txt.version != expected.version {
        return Err(DiscoveryError::BadMdns);
    }
    Ok(MdnsAnnouncement { device_id, txt })
}

fn txt_for_instance(
    packet: &[u8],
    records: &[Record],
    instance: &str,
) -> Result<MdnsTxt, DiscoveryError> {
    for record in records {
        if record.typ == TXT && eq_name(&record.name, instance) {
            let bytes = packet
                .get(record.at..record.at + record.len)
                .ok_or(DiscoveryError::BadMdns)?;
            return MdnsTxt::from_character_strings(parse_txt(bytes)?);
        }
    }
    Err(DiscoveryError::BadMdns)
}

fn srv_port(packet: &[u8], records: &[Record], instance: &str) -> Result<u16, DiscoveryError> {
    for record in records {
        if record.typ == SRV && eq_name(&record.name, instance) {
            return read_srv_port(packet, record);
        }
    }
    Err(DiscoveryError::BadMdns)
}

fn read_srv_port(packet: &[u8], record: &Record) -> Result<u16, DiscoveryError> {
    if record.len < 6 || record.at + record.len > packet.len() {
        return Err(DiscoveryError::BadMdns);
    }
    let port = u16::from_be_bytes([packet[record.at + 4], packet[record.at + 5]]);
    let (_, end) = read_name(packet, record.at + 6, 0)?;
    if end != record.at + record.len {
        return Err(DiscoveryError::BadMdns);
    }
    Ok(port)
}

fn device_from_instance(labels: &[String]) -> Result<DeviceId, DiscoveryError> {
    let Some((label, rest)) = labels.split_first() else {
        return Err(DiscoveryError::BadMdns);
    };
    if !eq_name(rest, MDNS_SERVICE_TYPE) {
        return Err(DiscoveryError::BadMdns);
    }
    DeviceId::from_z32(label).map_err(DiscoveryError::from)
}

fn parse_txt(rdata: &[u8]) -> Result<Vec<String>, DiscoveryError> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < rdata.len() {
        let len = usize::from(rdata[index]);
        index += 1;
        let end = index + len;
        if end > rdata.len() {
            return Err(DiscoveryError::BadMdns);
        }
        let text = std::str::from_utf8(&rdata[index..end]).map_err(|_| DiscoveryError::BadMdns)?;
        out.push(text.to_string());
        index = end;
    }
    Ok(out)
}

fn read_rr(packet: &[u8], pos: usize) -> Result<(Record, usize), DiscoveryError> {
    let (name, pos) = read_name(packet, pos, 0)?;
    if pos + 10 > packet.len() {
        return Err(DiscoveryError::BadMdns);
    }
    let typ = u16::from_be_bytes([packet[pos], packet[pos + 1]]);
    let class = u16::from_be_bytes([packet[pos + 2], packet[pos + 3]]);
    if class & 0x7fff != CLASS_IN {
        return Err(DiscoveryError::BadMdns);
    }
    let len = usize::from(u16::from_be_bytes([packet[pos + 8], packet[pos + 9]]));
    let at = pos + 10;
    let next = at + len;
    if next > packet.len() {
        return Err(DiscoveryError::BadMdns);
    }
    Ok((Record { name, typ, at, len }, next))
}

fn question_end(packet: &[u8], pos: usize) -> Result<usize, DiscoveryError> {
    if pos + 4 > packet.len() {
        return Err(DiscoveryError::BadMdns);
    }
    let typ = u16::from_be_bytes([packet[pos], packet[pos + 1]]);
    let class = u16::from_be_bytes([packet[pos + 2], packet[pos + 3]]);
    if typ != PTR || class != CLASS_IN {
        return Err(DiscoveryError::BadMdns);
    }
    Ok(pos + 4)
}

fn read_name(
    packet: &[u8],
    mut pos: usize,
    depth: u8,
) -> Result<(Vec<String>, usize), DiscoveryError> {
    if depth > 8 {
        return Err(DiscoveryError::BadMdns);
    }
    let mut labels = Vec::new();
    let mut end = None;
    loop {
        if pos >= packet.len() {
            return Err(DiscoveryError::BadMdns);
        }
        let len_byte = packet[pos];
        if len_byte == 0 {
            if end.is_none() {
                end = Some(pos + 1);
            }
            break;
        }
        if len_byte & 0xc0 == 0xc0 {
            if pos + 1 >= packet.len() {
                return Err(DiscoveryError::BadMdns);
            }
            let ptr = (usize::from(len_byte & 0x3f) << 8) | usize::from(packet[pos + 1]);
            if end.is_none() {
                end = Some(pos + 2);
            }
            let (rest, _) = read_name(packet, ptr, depth + 1)?;
            labels.extend(rest);
            break;
        }
        if len_byte & 0xc0 != 0 {
            return Err(DiscoveryError::BadMdns);
        }
        let len = usize::from(len_byte);
        let start = pos + 1;
        let label_end = start + len;
        if label_end > packet.len() || len == 0 {
            return Err(DiscoveryError::BadMdns);
        }
        let label =
            std::str::from_utf8(&packet[start..label_end]).map_err(|_| DiscoveryError::BadMdns)?;
        labels.push(label.to_string());
        pos = label_end;
    }
    Ok((labels, end.ok_or(DiscoveryError::BadMdns)?))
}

fn eq_name(labels: &[String], dotted: &str) -> bool {
    let mut got = labels.join(".");
    if !got.is_empty() {
        got.push('.');
    }
    got.eq_ignore_ascii_case(dotted)
}

fn write_header(out: &mut Vec<u8>, flags: u16, qd: u16, an: u16, ns: u16, ar: u16) {
    push_u16(out, 0);
    push_u16(out, flags);
    push_u16(out, qd);
    push_u16(out, an);
    push_u16(out, ns);
    push_u16(out, ar);
}

fn write_question(out: &mut Vec<u8>, name: &str, typ: u16) -> Result<(), DiscoveryError> {
    write_name(out, name)?;
    push_u16(out, typ);
    push_u16(out, CLASS_IN);
    Ok(())
}

fn write_rr(out: &mut Vec<u8>, name: &str, typ: u16, rdata: &[u8]) -> Result<(), DiscoveryError> {
    write_name(out, name)?;
    push_u16(out, typ);
    push_u16(out, CLASS_IN);
    push_u32(out, TTL);
    let len = u16::try_from(rdata.len()).map_err(|_| DiscoveryError::BadMdns)?;
    push_u16(out, len);
    out.extend_from_slice(rdata);
    Ok(())
}

fn write_txt(strings: &[String]) -> Result<Vec<u8>, DiscoveryError> {
    let mut out = Vec::new();
    for item in strings {
        let len = u8::try_from(item.len()).map_err(|_| DiscoveryError::BadMdns)?;
        out.push(len);
        out.extend_from_slice(item.as_bytes());
    }
    if out.is_empty() {
        out.push(0);
    }
    Ok(out)
}

fn write_name(out: &mut Vec<u8>, name: &str) -> Result<(), DiscoveryError> {
    let trimmed = name.trim_end_matches('.');
    if trimmed.is_empty() || trimmed.len() > 253 {
        return Err(DiscoveryError::BadMdns);
    }
    for label in trimmed.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(DiscoveryError::BadMdns);
        }
        let len = u8::try_from(label.len()).map_err(|_| DiscoveryError::BadMdns)?;
        out.push(len);
        out.extend_from_slice(label.as_bytes());
    }
    out.push(0);
    Ok(())
}

fn push_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn push_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}

fn instance_name(device_id: &DeviceId) -> Result<String, DiscoveryError> {
    let z32 = device_id.to_z32();
    if z32.is_empty() || z32.len() > 63 {
        return Err(DiscoveryError::BadMdns);
    }
    Ok(format!("{z32}.{MDNS_SERVICE_TYPE}"))
}

fn host_name(device_id: &DeviceId) -> Result<String, DiscoveryError> {
    Ok(format!("{}.local.", device_id.to_z32()))
}

fn normalize_name(name: Option<&str>) -> Result<Option<String>, DiscoveryError> {
    match name {
        None => Ok(None),
        Some(value) if value.is_empty() || value.len() > 63 => Err(DiscoveryError::BadMdns),
        Some(value) => Ok(Some(value.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cbdesk_crypto::Identity;

    fn device() -> DeviceId {
        Identity::from_seed(&[9u8; 32]).expect("seed").device_id()
    }

    #[test]
    fn announcement_roundtrip_uses_the_txt_fields() {
        let id = device();
        let bytes = encode_announcement(&id, 22116, None).expect("encode");
        let MdnsMessage::Announcement(got) = decode_mdns(&bytes).expect("decode") else {
            panic!("expected an announcement");
        };
        assert_eq!(got.device_id, id);
        assert_eq!(got.txt, MdnsTxt::from_device(&id, 22116, None));
        assert!(got
            .txt
            .character_strings()
            .iter()
            .all(|item| !item.starts_with("name=")));

        let named = encode_announcement(&id, 22116, Some("Ofis")).expect("encode");
        let MdnsMessage::Announcement(got) = decode_mdns(&named).expect("decode") else {
            panic!("expected an announcement");
        };
        assert_eq!(got.txt.name.as_deref(), Some("Ofis"));
        assert!(decode_mdns(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn browse_query_is_not_an_announcement() {
        let bytes = encode_browse_query().expect("query");
        assert!(matches!(
            decode_mdns(&bytes).expect("decode"),
            MdnsMessage::Query
        ));
    }
}
