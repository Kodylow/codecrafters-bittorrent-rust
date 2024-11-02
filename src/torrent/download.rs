use anyhow::Result;
use sha1::Digest;
use std::time::Duration;
use tracing::info;

use crate::torrent::{
    message::Message,
    metainfo::TorrentMetainfo,
    peer::{Peer, PeerConfig},
    tracker::{self, TrackerConfig},
};

pub struct Downloader {
    torrent: TorrentMetainfo,
    peers: Vec<String>,
    peer_config: PeerConfig,
}

impl Downloader {
    pub async fn new(torrent: TorrentMetainfo) -> Result<Self> {
        let info_hash = torrent.info_hash()?;
        let peer_config = PeerConfig {
            info_hash,
            ..Default::default()
        };

        let peers = tracker::get_peers(
            &torrent.announce,
            info_hash,
            torrent.info.length as u64,
            Some(TrackerConfig::default()),
        )
        .await?;

        if peers.is_empty() {
            return Err(anyhow::anyhow!("No peers available"));
        }

        Ok(Self {
            torrent,
            peers: peers.into_iter().map(|p| p.to_string()).collect(),
            peer_config,
        })
    }

    pub async fn download_piece(&self, piece_index: usize) -> Result<Vec<u8>> {
        let piece_length = self.torrent.info.piece_size(piece_index);

        for peer_addr in self.peers.iter().cycle().take(3 * self.peers.len()) {
            if peer_addr != &self.peers[0] {
                info!("Retrying piece {} with new peer", piece_index);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }

            let mut peer = Peer::new(peer_addr.parse()?, self.peer_config.clone());
            match self
                .download_piece_from_peer(&mut peer, piece_index, piece_length)
                .await
            {
                Ok(data) => return Ok(data),
                Err(e) => info!("Failed to download from peer {}: {}", peer_addr, e),
            }
        }

        Err(anyhow::anyhow!(
            "Failed to download piece {} after trying all peers",
            piece_index
        ))
    }

    pub async fn download_all(&self, output: &str) -> Result<()> {
        let mut file_data = Vec::with_capacity(self.torrent.info.length);

        for piece_index in 0..self.torrent.info.total_pieces() {
            info!(
                "Downloading piece {}/{}",
                piece_index + 1,
                self.torrent.info.total_pieces()
            );

            let piece_data = self.download_piece(piece_index).await?;
            file_data.extend_from_slice(&piece_data);
        }

        tokio::fs::write(output, file_data).await?;
        info!("Download completed successfully");
        Ok(())
    }

    async fn download_piece_from_peer(
        &self,
        peer: &mut Peer,
        piece_index: usize,
        piece_length: usize,
    ) -> Result<Vec<u8>> {
        peer.connect().await?;
        self.wait_for_bitfield(peer, piece_index).await?;
        self.wait_for_unchoke(peer).await?;

        let piece_data = peer.download_piece(piece_index, piece_length).await?;
        self.verify_piece(&piece_data, piece_index)?;

        Ok(piece_data)
    }

    async fn wait_for_bitfield(&self, peer: &mut Peer, piece_index: usize) -> Result<()> {
        loop {
            match peer.receive_message().await? {
                Message::Bitfield(b) => {
                    let byte_index = piece_index / 8;
                    let bit_index = 7 - (piece_index % 8);

                    if byte_index >= b.len() || (b[byte_index] & (1 << bit_index)) == 0 {
                        return Err(anyhow::anyhow!("Peer does not have piece {}", piece_index));
                    }
                    return Ok(());
                }
                Message::KeepAlive => continue,
                msg => {
                    return Err(anyhow::anyhow!(
                        "Unexpected message before bitfield: {:?}",
                        msg
                    ))
                }
            }
        }
    }

    async fn wait_for_unchoke(&self, peer: &mut Peer) -> Result<()> {
        peer.send_message(Message::Interested).await?;

        loop {
            match peer.receive_message().await? {
                Message::Unchoke => return Ok(()),
                Message::KeepAlive | Message::Choke | _ => continue,
            }
        }
    }

    fn verify_piece(&self, piece_data: &[u8], piece_index: usize) -> Result<()> {
        let mut hasher = sha1::Sha1::new();
        hasher.update(piece_data);
        let hash = hasher.finalize();

        let expected_hash = &self.torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];

        if hash.as_slice() != expected_hash {
            return Err(anyhow::anyhow!("Piece hash verification failed"));
        }

        Ok(())
    }
}
