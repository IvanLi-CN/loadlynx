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
const LOADLYNX_SERVICE: &str = "_loadlynx._tcp.local";
const HTTP_SERVICE: &str = "_http._tcp.local";
const DNS_TYPE_A: u16 = 1;
const DNS_TYPE_PTR: u16 = 12;
const DNS_TYPE_TXT: u16 = 16;
const DNS_TYPE_SRV: u16 = 33;
const DNS_TYPE_ANY: u16 = 255;
const DNS_CLASS_IN: u16 = 1;
const DNS_CLASS_CACHE_FLUSH_IN: u16 = 0x8001;

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
            "mdns: announcing LoadLynx DNS-SD service (hostname={}, ip={}, port={})",
            cfg.hostname_fqdn.as_str(),
            ip,
            cfg.port
        );

        // Send an initial unsolicited announcement.
        send_service_response(
            &mut socket,
            &mut resp_buf,
            &cfg,
            ip,
            IpEndpoint::new(IpAddress::Ipv4(MDNS_MULTICAST_V4), MDNS_PORT),
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
                                } else if service_query_matches(&query.name, &cfg) {
                                    let dest = if query.unicast_response {
                                        meta.endpoint
                                    } else {
                                        IpEndpoint::new(
                                            IpAddress::Ipv4(MDNS_MULTICAST_V4),
                                            MDNS_PORT,
                                        )
                                    };
                                    send_service_response(
                                        &mut socket,
                                        &mut resp_buf,
                                        &cfg,
                                        ip,
                                        dest,
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
                    send_service_response(
                        &mut socket,
                        &mut resp_buf,
                        &cfg,
                        ip,
                        IpEndpoint::new(IpAddress::Ipv4(MDNS_MULTICAST_V4), MDNS_PORT),
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

async fn send_service_response(
    socket: &mut UdpSocket<'_>,
    buf: &mut [u8],
    cfg: &MdnsConfig,
    ip: Ipv4Address,
    dest: IpEndpoint,
) {
    let len = build_service_response(buf, cfg, ip).unwrap_or_else(|| {
        warn!("mdns: failed to encode DNS-SD response (buffer too small)");
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
    buf[offset + 1] = DNS_TYPE_A as u8;
    // Class IN with cache-flush bit set
    buf[offset + 2] = (DNS_CLASS_CACHE_FLUSH_IN >> 8) as u8;
    buf[offset + 3] = DNS_CLASS_CACHE_FLUSH_IN as u8;
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

fn build_service_response(buf: &mut [u8], cfg: &MdnsConfig, ip: Ipv4Address) -> Option<usize> {
    if buf.len() < 12 {
        return None;
    }

    buf[0] = 0;
    buf[1] = 0;
    buf[2] = 0x84;
    buf[3] = 0x00;
    buf[4] = 0;
    buf[5] = 0; // QDCOUNT
    buf[6] = 0;
    buf[7] = 7; // ANCOUNT: PTR + PTR + two SRV/TXT pairs + A
    buf[8] = 0;
    buf[9] = 0;
    buf[10] = 0;
    buf[11] = 0;

    let mut offset = 12;
    let loadlynx_instance = service_instance_name(cfg.hostname.as_str(), LOADLYNX_SERVICE);
    let http_instance = service_instance_name(cfg.hostname.as_str(), HTTP_SERVICE);

    offset = write_name_rr(
        buf,
        offset,
        LOADLYNX_SERVICE,
        DNS_TYPE_PTR,
        DNS_CLASS_IN,
        loadlynx_instance.as_str(),
    )?;
    offset = write_name_rr(
        buf,
        offset,
        HTTP_SERVICE,
        DNS_TYPE_PTR,
        DNS_CLASS_IN,
        http_instance.as_str(),
    )?;
    offset = write_srv_rr(
        buf,
        offset,
        loadlynx_instance.as_str(),
        cfg.port,
        cfg.hostname_fqdn.as_str(),
    )?;
    offset = write_txt_rr(
        buf,
        offset,
        loadlynx_instance.as_str(),
        cfg.hostname.as_str(),
    )?;
    offset = write_srv_rr(
        buf,
        offset,
        http_instance.as_str(),
        cfg.port,
        cfg.hostname_fqdn.as_str(),
    )?;
    offset = write_txt_rr(buf, offset, http_instance.as_str(), cfg.hostname.as_str())?;
    offset = write_a_rr(buf, offset, cfg.hostname_fqdn.as_str(), ip)?;
    Some(offset)
}

fn service_instance_name(hostname: &str, service_name: &str) -> String<96> {
    let mut out = String::<96>::new();
    let _ = out.push_str("LoadLynx ");
    let _ = out.push_str(hostname);
    let _ = out.push('.');
    let _ = out.push_str(service_name);
    out
}

fn write_name_rr(
    buf: &mut [u8],
    offset: usize,
    owner: &str,
    rr_type: u16,
    rr_class: u16,
    target: &str,
) -> Option<usize> {
    let (mut offset, rdlen_pos, rdata_start) = begin_rr(buf, offset, owner, rr_type, rr_class)?;
    offset = encode_dns_name(buf, offset, target)?;
    finish_rr(buf, rdlen_pos, rdata_start, offset)
}

fn write_srv_rr(
    buf: &mut [u8],
    offset: usize,
    owner: &str,
    port: u16,
    target: &str,
) -> Option<usize> {
    let (mut offset, rdlen_pos, rdata_start) =
        begin_rr(buf, offset, owner, DNS_TYPE_SRV, DNS_CLASS_CACHE_FLUSH_IN)?;
    if offset + 6 > buf.len() {
        return None;
    }
    buf[offset] = 0;
    buf[offset + 1] = 0; // priority
    buf[offset + 2] = 0;
    buf[offset + 3] = 0; // weight
    buf[offset + 4] = (port >> 8) as u8;
    buf[offset + 5] = port as u8;
    offset += 6;
    offset = encode_dns_name(buf, offset, target)?;
    finish_rr(buf, rdlen_pos, rdata_start, offset)
}

fn write_txt_rr(buf: &mut [u8], offset: usize, owner: &str, hostname: &str) -> Option<usize> {
    let (mut offset, rdlen_pos, rdata_start) =
        begin_rr(buf, offset, owner, DNS_TYPE_TXT, DNS_CLASS_CACHE_FLUSH_IN)?;
    for item in [
        "product=loadlynx",
        "api=v1",
        "cap=net_http,usb_cdc_jsonl",
        hostname,
    ] {
        let txt = if item == hostname { "device_id=" } else { item };
        let suffix = if item == hostname { hostname } else { "" };
        let len = txt.len() + suffix.len();
        if len > 255 || offset + 1 + len > buf.len() {
            return None;
        }
        buf[offset] = len as u8;
        offset += 1;
        buf[offset..offset + txt.len()].copy_from_slice(txt.as_bytes());
        offset += txt.len();
        if !suffix.is_empty() {
            buf[offset..offset + suffix.len()].copy_from_slice(suffix.as_bytes());
            offset += suffix.len();
        }
    }
    finish_rr(buf, rdlen_pos, rdata_start, offset)
}

fn write_a_rr(buf: &mut [u8], offset: usize, owner: &str, ip: Ipv4Address) -> Option<usize> {
    let (mut offset, rdlen_pos, rdata_start) =
        begin_rr(buf, offset, owner, DNS_TYPE_A, DNS_CLASS_CACHE_FLUSH_IN)?;
    let octets = ip.octets();
    if offset + 4 > buf.len() {
        return None;
    }
    buf[offset..offset + 4].copy_from_slice(&octets);
    offset += 4;
    finish_rr(buf, rdlen_pos, rdata_start, offset)
}

fn begin_rr(
    buf: &mut [u8],
    mut offset: usize,
    owner: &str,
    rr_type: u16,
    rr_class: u16,
) -> Option<(usize, usize, usize)> {
    offset = encode_dns_name(buf, offset, owner)?;
    if offset + 10 > buf.len() {
        return None;
    }
    buf[offset] = (rr_type >> 8) as u8;
    buf[offset + 1] = rr_type as u8;
    buf[offset + 2] = (rr_class >> 8) as u8;
    buf[offset + 3] = rr_class as u8;
    buf[offset + 4] = (MDNS_RESPONSE_TTL_SECS >> 24) as u8;
    buf[offset + 5] = (MDNS_RESPONSE_TTL_SECS >> 16) as u8;
    buf[offset + 6] = (MDNS_RESPONSE_TTL_SECS >> 8) as u8;
    buf[offset + 7] = MDNS_RESPONSE_TTL_SECS as u8;
    let rdlen_pos = offset + 8;
    offset += 10;
    Some((offset, rdlen_pos, offset))
}

fn finish_rr(buf: &mut [u8], rdlen_pos: usize, rdata_start: usize, offset: usize) -> Option<usize> {
    let rdlen = offset.checked_sub(rdata_start)?;
    if rdlen > u16::MAX as usize || rdlen_pos + 1 >= buf.len() {
        return None;
    }
    buf[rdlen_pos] = (rdlen >> 8) as u8;
    buf[rdlen_pos + 1] = rdlen as u8;
    Some(offset)
}

fn encode_dns_name(buf: &mut [u8], mut offset: usize, name: &str) -> Option<usize> {
    for label in name.trim_end_matches('.').split('.') {
        let len = label.len();
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

fn encode_name(buf: &mut [u8], mut offset: usize, hostname: &str) -> Option<usize> {
    // Encode "<hostname>.local"
    for label in [hostname, "local"] {
        let len = label.len();
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

    if !matches!(
        qtype,
        DNS_TYPE_A | DNS_TYPE_PTR | DNS_TYPE_TXT | DNS_TYPE_SRV | DNS_TYPE_ANY
    ) {
        return None;
    }
    if qclass != DNS_CLASS_IN {
        return None;
    }

    Some(Query {
        name,
        qtype,
        unicast_response: unicast,
        _marker: core::marker::PhantomData,
    })
}

fn decode_name(packet: &[u8], mut offset: usize, out: &mut String<64>) -> Option<usize> {
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

fn service_name_matches(candidate: &str) -> bool {
    name_matches(candidate, LOADLYNX_SERVICE) || name_matches(candidate, HTTP_SERVICE)
}

fn service_query_matches(candidate: &str, cfg: &MdnsConfig) -> bool {
    if service_name_matches(candidate) {
        return true;
    }
    let loadlynx_instance = service_instance_name(cfg.hostname.as_str(), LOADLYNX_SERVICE);
    let http_instance = service_instance_name(cfg.hostname.as_str(), HTTP_SERVICE);
    name_matches(candidate, loadlynx_instance.as_str())
        || name_matches(candidate, http_instance.as_str())
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

    #[test]
    fn dns_sd_response_advertises_loadlynx_service() {
        let ip = Ipv4Address::new(192, 168, 1, 42);
        let cfg = MdnsConfig {
            hostname: hostname_from_short_id("aabbcc"),
            hostname_fqdn: fqdn_from_hostname("loadlynx-aabbcc"),
            port: 80,
        };
        let mut buf = [0u8; 512];
        let len = build_service_response(&mut buf, &cfg, ip).unwrap();

        assert_eq!(buf[7], 7);
        let packet = core::str::from_utf8(&buf[..len]).unwrap_or("");
        assert!(packet.contains("_loadlynx"));
        assert!(packet.contains("_http"));
        assert!(packet.contains("product=loadlynx"));
        assert!(packet.contains("device_id=loadlynx-aabbcc"));
    }

    #[test]
    fn dns_sd_query_match_accepts_service_instances() {
        let cfg = MdnsConfig {
            hostname: hostname_from_short_id("aabbcc"),
            hostname_fqdn: fqdn_from_hostname("loadlynx-aabbcc"),
            port: 80,
        };

        assert!(service_query_matches("_http._tcp.local", &cfg));
        assert!(service_query_matches(
            "LoadLynx loadlynx-aabbcc._http._tcp.local",
            &cfg
        ));
        assert!(service_query_matches(
            "LoadLynx loadlynx-aabbcc._loadlynx._tcp.local.",
            &cfg
        ));
        assert!(!service_query_matches("_ssh._tcp.local", &cfg));
    }
}
