use anyhow::Result;
use sha1::Digest;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tracing::info;

use super::message::Message;
use super::peer::InfoHash;
use super::{
    metainfo::TorrentMetainfo,
    peer::{Peer, PeerConfig},
};

#[derive(Debug)]
struct PieceWork {
    index: usize,
    length: usize,
    retries: usize,
}

#[derive(Debug)]
struct PeerState {
    successful_pieces: usize,
    failed_pieces: usize,
    last_success: std::time::Instant,
}

#[derive(Debug, Clone)]
pub struct DownloadConfig {
    pub peer_retries: usize,
    pub peer_timeout: Duration,
    pub max_pending: usize,
    pub piece_retries: usize,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            peer_retries: 3,
            peer_timeout: Duration::from_secs(10),
            max_pending: 5,
            piece_retries: 3,
        }
    }
}

pub struct DownloadManager {
    torrent: Arc<TorrentMetainfo>,
    peers: Vec<String>,
    info_hash: InfoHash,
    pieces_queue: Arc<Mutex<Vec<PieceWork>>>,
    completed_pieces: Arc<Mutex<Vec<Option<Vec<u8>>>>>,
    config: DownloadConfig,
}

impl DownloadManager {
    pub fn new(
        torrent: TorrentMetainfo,
        peers: Vec<String>,
        config: Option<DownloadConfig>,
    ) -> Result<Self> {
        let info_hash = torrent.info_hash()?;
        let total_pieces = torrent.info.total_pieces();
        let pieces_queue = (0..total_pieces)
            .map(|i| PieceWork {
                index: i,
                length: torrent.info.piece_size(i),
                retries: 0,
            })
            .collect();

        let completed_pieces = vec![None; total_pieces];

        Ok(Self {
            torrent: Arc::new(torrent),
            peers,
            info_hash,
            pieces_queue: Arc::new(Mutex::new(pieces_queue)),
            completed_pieces: Arc::new(Mutex::new(completed_pieces)),
            config: config.unwrap_or_default(),
        })
    }

    pub async fn download_single_peer(&self) -> Result<Vec<u8>> {
        if self.peers.is_empty() {
            return Err(anyhow::anyhow!("No peers available"));
        }

        let peer_config = PeerConfig {
            info_hash: self.info_hash,
            ..Default::default()
        };

        let mut peer = Peer::new(self.peers[0].parse()?, peer_config);
        peer.connect().await?;

        // Wait for and verify bitfield
        let bitfield = match peer.receive_message().await? {
            Message::Bitfield(b) => b,
            _ => return Err(anyhow::anyhow!("Expected bitfield message")),
        };

        // Send interested message once
        peer.send_message(Message::Interested).await?;

        // Wait for unchoke once
        match peer.receive_message().await? {
            Message::Unchoke => (),
            _ => return Err(anyhow::anyhow!("Expected unchoke message")),
        };

        let mut file_data = Vec::with_capacity(self.torrent.info.length);

        for piece_index in 0..self.torrent.info.total_pieces() {
            info!(
                "Downloading piece {}/{}",
                piece_index + 1,
                self.torrent.info.total_pieces()
            );

            // Verify piece availability in bitfield
            let byte_index = piece_index / 8;
            let bit_index = 7 - (piece_index % 8);
            if byte_index >= bitfield.len() || (bitfield[byte_index] & (1 << bit_index)) == 0 {
                return Err(anyhow::anyhow!("Peer does not have piece {}", piece_index));
            }

            let piece_length = self.torrent.info.piece_size(piece_index);
            let piece_data = peer.download_piece(piece_index, piece_length).await?;

            // Verify piece hash
            let mut hasher = sha1::Sha1::new();
            hasher.update(&piece_data);
            let hash = hasher.finalize();
            let expected_hash = &self.torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];

            if hash.as_slice() != expected_hash {
                return Err(anyhow::anyhow!(
                    "Piece {} hash verification failed",
                    piece_index
                ));
            }

            file_data.extend_from_slice(&piece_data);
        }

        Ok(file_data)
    }
}
