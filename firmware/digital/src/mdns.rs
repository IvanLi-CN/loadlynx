#![allow(dead_code)]

use core::fmt::Write as _;

use defmt::*;
use embassy_futures::select::{Either, select};
use embassy_net::{
    IpAddress, IpEndpoint, Ipv4Address, Stack,
    udp::{PacketMetadata, RecvError, SendError, UdpSocket},
};
use embassy_time::{Duration, Timer};
use heapless::String;

const MDNS_MULTICAST_V4: Ipv4Address = Ipv4Address::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;
const MDNS_RESPONSE_TTL_SECS: u32 = 120;
const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_DELAY: Duration = Duration::from_secs(2);
const HOSTNAME_PREFIX: &str = "loadlynx-";

/// Configuration for the mDNS task.
#[derive(Clone)]
pub struct MdnsConfig {
    /// Hostname without the `.local` suffix, e.g. `loadlynx-a1b2c3`.
    pub hostname: String<32>,
    /// Fully qualified `.local` hostname, e.g. `loadlynx-a1b2c3.local`.
    pub hostname_fqdn: String<48>,
    /// HTTP service port (currently 80).
    pub port: u16,
}

/// Derive a 6‑character lowercase hex short ID from the last 3 bytes of the MAC.
pub fn short_id_from_mac(mac: [u8; 6]) -> String<6> {
    let mut out: String<6> = String::new();
    for byte in mac.iter().skip(3) {
        // Safe because capacity is exactly 6 and we always write 2 chars per byte.
        let _ = core::write!(out, "{:02x}", byte);
    }
    out
}

/// Build the hostname (`loadlynx-<short_id>`) from the provided short ID.
pub fn hostname_from_short_id(short_id: &str) -> String<32> {
    let mut out: String<32> = String::new();
    let _ = out.push_str(HOSTNAME_PREFIX);
    for ch in short_id.chars() {
        if ch.is_ascii_alphanumeric() {
            let _ = out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

/// Append `.local` to the hostname.
pub fn fqdn_from_hostname(hostname: &str) -> String<48> {
    let mut out: String<48> = String::new();
    let _ = out.push_str(hostname);
    let _ = out.push_str(".local");
    out
}

#[embassy_executor::task]
pub async fn mdns_task(stack: Stack<'static>, cfg: MdnsConfig) {
    run_mdns(stack, cfg).await;
}

pub async fn run_mdns(stack: Stack<'static>, cfg: MdnsConfig) -> ! {
    loop {
        stack.wait_config_up().await;

        let ip = match stack.config_v4() {
            Some(v4) => v4.address.address(),
            None => {
                Timer::after(RETRY_DELAY).await;
                continue;
            }
        };

        if let Err(err) = stack.join_multicast_group(IpAddress::Ipv4(MDNS_MULTICAST_V4)) {
            warn!(
                "mdns: failed to join multicast group (hostname={}): {:?}",
                cfg.hostname_fqdn.as_str(),
                err
            );
            Timer::after(RETRY_DELAY).await;
            continue;
        }

        let mut rx_meta = [PacketMetadata::EMPTY; 4];
        let mut tx_meta = [PacketMetadata::EMPTY; 4];
        let mut rx_storage = [0u8; 512];
        let mut tx_storage = [0u8; 512];
        let mut recv_buf = [0u8; 512];
        let mut resp_buf = [0u8; 512];

        let mut socket = UdpSocket::new(
            stack,
            &mut rx_meta,
            &mut rx_storage,
            &mut tx_meta,
            &mut tx_storage,
        );
        socket.set_hop_limit(Some(255));
        // Binding to the current IPv4 address (instead of 0.0.0.0) avoids emitting responses
        // with a source address of 0.0.0.0, which some resolvers will drop.
        if let Err(err) = socket.bind((IpAddress::Ipv4(ip), MDNS_PORT)) {
            warn!(
                "mdns: bind 5353 failed (hostname={}): {:?}",
                cfg.hostname_fqdn.as_str(),
                err
            );
            Timer::after(RETRY_DELAY).await;
            continue;
        }

        info!(
            "mdns: announcing HTTP service via .local hostname (hostname={}, ip={}, port={})",
            cfg.hostname_fqdn.as_str(),
            ip,
            cfg.port
        );

        // Send an initial unsolicited announcement.
        send_a_response(
            &mut socket,
            &mut resp_buf,
            cfg.hostname.as_str(),
            ip,
            IpEndpoint::new(IpAddress::Ipv4(MDNS_MULTICAST_V4), MDNS_PORT),
            false,
        )
        .await;

        let mut announce_timer = Timer::after(ANNOUNCE_INTERVAL);

        loop {
            let recv_fut = socket.recv_from(&mut recv_buf);
            match select(recv_fut, announce_timer).await {
                Either::First(res) => {
                    announce_timer = Timer::after(ANNOUNCE_INTERVAL);
                    match res {
                        Ok((len, meta)) => {
                            if let Some(query) = parse_query(&recv_buf[..len]) {
                                info!(
                                    "mdns: query name={} qtype={} unicast={}",
                                    query.name.as_str(),
                                    query.qtype,
                                    query.unicast_response
                                );
                                if name_matches(&query.name, cfg.hostname_fqdn.as_str()) {
                                    let dest = if query.unicast_response {
                                        meta.endpoint
                                    } else {
                                        IpEndpoint::new(
                                            IpAddress::Ipv4(MDNS_MULTICAST_V4),
                                            MDNS_PORT,
                                        )
                                    };
                                    send_a_response(
                                        &mut socket,
                                        &mut resp_buf,
                                        cfg.hostname.as_str(),
                                        ip,
                                        dest,
                                        true,
                                    )
                                    .await;
                                }
                            }
                        }
                        Err(err) => match err {
                            RecvError::Truncated => {
                                warn!("mdns: truncated datagram");
                            }
                        },
                    }
                }
                Either::Second(_) => {
                    // Periodic unsolicited announcement.
                    send_a_response(
                        &mut socket,
                        &mut resp_buf,
                        cfg.hostname.as_str(),
                        ip,
                        IpEndpoint::new(IpAddress::Ipv4(MDNS_MULTICAST_V4), MDNS_PORT),
                        false,
                    )
                    .await;
                    announce_timer = Timer::after(ANNOUNCE_INTERVAL);
                }
            }

            if !stack.is_config_up() {
                break;
            }
        }

        Timer::after(RETRY_DELAY).await;
    }
}

async fn send_a_response(
    socket: &mut UdpSocket<'_>,
    buf: &mut [u8],
    hostname: &str,
    ip: Ipv4Address,
    dest: IpEndpoint,
    include_question: bool,
) {
    let len = if include_question {
        build_a_response(buf, hostname, ip)
    } else {
        build_unsolicited_response(buf, hostname, ip)
    }
    .unwrap_or_else(|| {
        warn!("mdns: failed to encode response (buffer too small)");
        0
    });
    if len == 0 {
        return;
    }

    if let Err(err) = socket.send_to(&buf[..len], dest).await {
        match err {
            SendError::NoRoute => warn!("mdns: send_to no route"),
            SendError::SocketNotBound => warn!("mdns: socket not bound"),
            SendError::PacketTooLarge => warn!("mdns: packet too large"),
        }
    }
}

fn build_a_response(buf: &mut [u8], hostname: &str, ip: Ipv4Address) -> Option<usize> {
    build_response_inner(buf, hostname, ip, true)
}

fn build_unsolicited_response(buf: &mut [u8], hostname: &str, ip: Ipv4Address) -> Option<usize> {
    build_response_inner(buf, hostname, ip, false)
}

fn build_response_inner(
    buf: &mut [u8],
    hostname: &str,
    ip: Ipv4Address,
    include_question: bool,
) -> Option<usize> {
    if buf.len() < 12 {
        return None;
    }

    // Header
    buf[0] = 0;
    buf[1] = 0; // ID = 0 for mDNS
    buf[2] = 0x84; // QR=1, AA=1
    buf[3] = 0x00;

    // Set QDCOUNT/ANCOUNT according to mode.
    let qdcount = if include_question { 1u16 } else { 0u16 };
    buf[4] = (qdcount >> 8) as u8;
    buf[5] = qdcount as u8;
    buf[6] = 0;
    buf[7] = 1; // ANCOUNT = 1
    buf[8] = 0;
    buf[9] = 0; // NSCOUNT
    buf[10] = 0;
    buf[11] = 0; // ARCOUNT

    let mut offset = 12;

    // Question (optional)
    if include_question {
        offset = encode_name(buf, offset, hostname)?;
        if offset + 4 > buf.len() {
            return None;
        }
        // QTYPE A
        buf[offset] = 0;
        buf[offset + 1] = 1;
        // QCLASS IN
        buf[offset + 2] = 0;
        buf[offset + 3] = 1;
        offset += 4;
    }

    // Answer name: if we included question, we can pointer-compress.
    if include_question {
        if offset + 2 > buf.len() {
            return None;
        }
        // pointer to the question name at 0x000c
        buf[offset] = 0xC0;
        buf[offset + 1] = 0x0C;
        offset += 2;
    } else {
        offset = encode_name(buf, offset, hostname)?;
    }

    if offset + 10 > buf.len() {
        return None;
    }

    // Type A
    buf[offset] = 0;
    buf[offset + 1] = 1;
    // Class IN with cache‑flush bit set
    buf[offset + 2] = 0x80;
    buf[offset + 3] = 0x01;
    // TTL
    buf[offset + 4] = (MDNS_RESPONSE_TTL_SECS >> 24) as u8;
    buf[offset + 5] = (MDNS_RESPONSE_TTL_SECS >> 16) as u8;
    buf[offset + 6] = (MDNS_RESPONSE_TTL_SECS >> 8) as u8;
    buf[offset + 7] = MDNS_RESPONSE_TTL_SECS as u8;
    // RDLENGTH
    buf[offset + 8] = 0;
    buf[offset + 9] = 4;
    // RDATA
    let octets = ip.octets();
    let end = offset + 14;
    if end > buf.len() {
        return None;
    }
    buf[offset + 10..end].copy_from_slice(&octets);
    Some(end)
}

fn encode_name(buf: &mut [u8], mut offset: usize, hostname: &str) -> Option<usize> {
    // Encode "<hostname>.local"
    for label in [hostname, "local"] {
        let len = label.as_bytes().len();
        if len == 0 || len > 63 || offset + 1 + len > buf.len() {
            return None;
        }
        buf[offset] = len as u8;
        offset += 1;
        buf[offset..offset + len].copy_from_slice(label.as_bytes());
        offset += len;
    }

    if offset >= buf.len() {
        return None;
    }
    buf[offset] = 0;
    Some(offset + 1)
}

#[derive(Debug)]
struct Query<'a> {
    name: String<64>,
    qtype: u16,
    unicast_response: bool,
    _marker: core::marker::PhantomData<&'a ()>,
}

fn parse_query(packet: &[u8]) -> Option<Query<'_>> {
    if packet.len() < 12 {
        return None;
    }

    let flags = u16::from_be_bytes([packet[2], packet[3]]);
    if flags & 0x8000 != 0 {
        // Not a query.
        return None;
    }

    let qdcount = u16::from_be_bytes([packet[4], packet[5]]);
    if qdcount == 0 {
        return None;
    }

    let mut offset = 12usize;
    let mut name = String::<64>::new();
    if let Some(next) = decode_name(packet, offset, &mut name) {
        offset = next;
    } else {
        return None;
    }

    if offset + 4 > packet.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([packet[offset], packet[offset + 1]]);
    let qclass_raw = u16::from_be_bytes([packet[offset + 2], packet[offset + 3]]);
    let unicast = (qclass_raw & 0x8000) != 0;
    let qclass = qclass_raw & 0x7FFF;

    if !(qtype == 1 || qtype == 255) {
        return None;
    }
    if qclass != 1 {
        return None;
    }

    Some(Query {
        name,
        qtype,
        unicast_response: unicast,
        _marker: core::marker::PhantomData,
    })
}

fn decode_name<'a>(packet: &'a [u8], mut offset: usize, out: &mut String<64>) -> Option<usize> {
    let mut jumped = false;
    let mut jump_offset = 0usize;

    loop {
        if offset >= packet.len() {
            return None;
        }
        let len = packet[offset];
        if len & 0xC0 == 0xC0 {
            if offset + 1 >= packet.len() {
                return None;
            }
            let ptr = (((len & 0x3F) as usize) << 8) | packet[offset + 1] as usize;
            if !jumped {
                jump_offset = offset + 2;
                jumped = true;
            }
            offset = ptr;
            continue;
        } else if len == 0 {
            offset += 1;
            break;
        } else {
            offset += 1;
            if offset + len as usize > packet.len() {
                return None;
            }
            if !out.is_empty() {
                let _ = out.push('.');
            }
            for &b in &packet[offset..offset + len as usize] {
                let _ = out.push((b as char).to_ascii_lowercase());
            }
            offset += len as usize;
        }
    }

    Some(if jumped { jump_offset } else { offset })
}

fn name_matches(candidate: &str, target: &str) -> bool {
    if candidate.eq_ignore_ascii_case(target) {
        return true;
    }
    if let Some(stripped) = candidate.strip_suffix('.') {
        return stripped.eq_ignore_ascii_case(target);
    }
    false
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    #[test]
    fn short_id_from_mac_basic_cases() {
        assert_eq!(
            short_id_from_mac([0x00, 0x11, 0x22, 0xAA, 0xBB, 0xCC]).as_str(),
            "aabbcc"
        );
        assert_eq!(short_id_from_mac([0, 0, 0, 0, 0, 0]).as_str(), "000000");
        assert_eq!(short_id_from_mac([0xFF; 6]).as_str(), "ffffff");
        assert_ne!(
            short_id_from_mac([0, 0, 0, 0x12, 0x34, 0x56]).as_str(),
            short_id_from_mac([0, 0, 0, 0x65, 0x43, 0x21]).as_str()
        );
    }

    #[test]
    fn hostname_and_fqdn_are_built_correctly() {
        let h = hostname_from_short_id("aabbcc");
        assert_eq!(h.as_str(), "loadlynx-aabbcc");
        let fqdn = fqdn_from_hostname(h.as_str());
        assert_eq!(fqdn.as_str(), "loadlynx-aabbcc.local");
    }

    #[test]
    fn encode_decode_roundtrip() {
        let ip = Ipv4Address::new(192, 168, 1, 42);
        let mut buf = [0u8; 128];
        let len = build_a_response(&mut buf, "loadlynx-aabbcc", ip).unwrap();
        // Basic header checks
        assert_eq!(&buf[0..2], &[0, 0]);
        assert_eq!(buf[2], 0x84);
        assert_eq!(buf[7], 1); // ANCOUNT

        // Ensure name is present and decodes back.
        let mut name = String::<64>::new();
        let next = decode_name(&buf, 12, &mut name).unwrap();
        assert_eq!(name.as_str(), "loadlynx-aabbcc.local");
        assert!(next < len);
    }
}
