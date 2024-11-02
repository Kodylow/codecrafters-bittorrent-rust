use anyhow::Result;
use serde::Serialize;
use std::net::Ipv4Addr;
use tracing::info;

use super::peer::PeerId;
use crate::{bencode::Bencode, PEER_ID};

#[derive(Debug)]
pub struct TrackerConfig {
    pub peer_id: PeerId,
    pub port: u16,
    pub compact: bool,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            peer_id: *PEER_ID,
            port: 6881,
            compact: true,
        }
    }
}

#[derive(Debug, Serialize)]
struct TrackerRequest<'a> {
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

fn urlencode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| format!("%{:02x}", b)).collect()
}

pub async fn get_peers(
    announce_url: &str,
    info_hash: [u8; 20],
    file_length: Option<u64>,
    config: Option<TrackerConfig>,
) -> Result<Vec<Peer>> {
    let config = config.unwrap_or_default();

    info!("Getting peers for tracker URL: {}", announce_url);
    let client = reqwest::Client::new();

    let request = TrackerRequest {
        peer_id: std::str::from_utf8(&config.peer_id)?,
        port: config.port,
        uploaded: 0,
        downloaded: 0,
        left: file_length.unwrap_or(0),
        compact: config.compact as u8,
    };

    let url_params = serde_urlencoded::to_string(&request)?;
    let url = format!(
        "{}?{}&info_hash={}",
        announce_url,
        url_params,
        urlencode(&info_hash)
    );

    info!("Tracker URL: {}", url);

    let response = client.get(url).send().await?;
    let response_bytes = response.bytes().await?;

    let bvalue = Bencode::decode_bytes(&response_bytes)?;
    info!("Response: {}", bvalue);
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
