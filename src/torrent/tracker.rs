//! Tracker communication and peer discovery functionality.
//!
//! Handles communication with BitTorrent trackers to discover peers
//! and obtain information about the swarm.

use anyhow::Result;
use serde::Serialize;
use std::net::Ipv4Addr;
use tracing::info;

use super::peer::PeerId;
use crate::{bencode::Bencode, utils::serialize_peer_id, PEER_ID};

/// Configuration options for tracker requests.
#[derive(Debug)]
pub struct TrackerConfig {
    /// The peer ID to identify ourselves to the tracker
    pub peer_id: PeerId,
    /// The port we're listening on for incoming connections
    pub port: u16,
    /// Whether to request compact peer lists
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

/// Request parameters sent to the tracker.
#[derive(Debug, Serialize)]
struct TrackerRequest<'a> {
    peer_id: &'a str,
    port: u16,
    uploaded: u64,
    downloaded: u64,
    left: u64,
    compact: u8,
}

/// Represents a peer in the swarm.
pub struct Peer {
    /// IPv4 address of the peer
    pub ip: Ipv4Addr,
    /// Port the peer is listening on
    pub port: u16,
}

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

/// URL encodes a byte slice for use in tracker requests.
fn urlencode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| format!("%{:02x}", b)).collect()
}

/// Contacts a tracker to get a list of peers for a torrent.
///
/// # Arguments
///
/// * `announce_url` - The tracker's announce URL
/// * `info_hash` - The 20-byte SHA1 hash of the torrent's info dictionary
/// * `file_length` - Optional total length of the torrent data in bytes
/// * `config` - Optional tracker configuration settings
///
/// # Returns
///
/// Returns a vector of peers on success, or an error if the tracker request fails.
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
        peer_id: &serialize_peer_id(&config.peer_id),
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
