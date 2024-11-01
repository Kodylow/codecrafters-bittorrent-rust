use anyhow::Result;
use reqwest::Url;
use serde::Serialize;
use std::net::Ipv4Addr;
use tracing::info;

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
    info!("Getting peers for tracker URL: {}", announce_url);
    let client = reqwest::blocking::Client::new();

    let encoded_hash = info_hash
        .iter()
        .map(|&b| format!("%{:02x}", b))
        .collect::<String>();

    let url = Url::parse_with_params(
        announce_url,
        &[
            ("info_hash", &encoded_hash),
            ("peer_id", &PEER_ID.to_string()),
            ("port", &PORT.to_string()),
            ("uploaded", &"0".to_string()),
            ("downloaded", &"0".to_string()),
            ("left", &file_length.to_string()),
            ("compact", &"1".to_string()),
        ],
    )?;

    info!("Tracker URL: {}", url);

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
