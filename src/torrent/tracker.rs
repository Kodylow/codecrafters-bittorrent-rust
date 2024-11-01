use anyhow::Result;
use reqwest::Url;
use serde::Serialize;
use std::net::Ipv4Addr;

use crate::bencode::Bencode;

const PEER_ID: &str = "00112233445566778899";
const PORT: u16 = 6881;

#[derive(Debug, Serialize)]
struct TrackerRequest<'a> {
    info_hash: &'a [u8],
    peer_id: &'a str,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    compact: u8,
}

pub struct Peer {
    pub ip: Ipv4Addr,
    pub port: u16,
}

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

pub fn get_peers(announce_url: &str, info_hash: [u8; 20], file_length: u64) -> Result<Vec<Peer>> {
    let client = reqwest::blocking::Client::new();

    let request = TrackerRequest {
        info_hash: &info_hash,
        peer_id: PEER_ID,
        port: PORT,
        uploaded: 0,
        downloaded: 0,
        left: file_length,
        compact: 1,
    };

    let url: Url = format!(
        "{}?{}",
        announce_url,
        serde_urlencoded::to_string(&request)?
    )
    .parse()?;

    let response = client.get(url).send()?;
    let response_bytes = response.bytes()?;

    let bvalue = Bencode::decode_bytes(&response_bytes)?;
    let peers = bvalue
        .get_dict()?
        .get("peers")
        .ok_or(anyhow::anyhow!("Peers not found"))?;
    let peers_bytes = peers.get_bytes()?;

    let mut peers = Vec::new();
    for chunk in peers_bytes.chunks_exact(6) {
        let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
        peers.push(Peer { ip, port });
    }

    Ok(peers)
}
