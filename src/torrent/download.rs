use anyhow::Result;
use sha1::Digest;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{error, info};

use super::{metainfo::TorrentMetainfo, peer::Peer};

const MAX_PEER_RETRIES: usize = 3;
const PEER_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_PENDING_REQUESTS: usize = 5;

pub struct DownloadManager {
    torrent: Arc<TorrentMetainfo>,
    peers: Vec<String>,
    info_hash: [u8; 20],
    pieces_queue: Arc<Mutex<Vec<usize>>>,
}

impl DownloadManager {
    pub fn new(torrent: TorrentMetainfo, peers: Vec<String>) -> Result<Self> {
        let info_hash = torrent.info_hash()?;
        let total_pieces = torrent.info.total_pieces();
        let pieces_queue: Vec<usize> = (0..total_pieces).collect();

        Ok(Self {
            torrent: Arc::new(torrent),
            peers,
            info_hash,
            pieces_queue: Arc::new(Mutex::new(pieces_queue)),
        })
    }

    pub async fn download(&self) -> Result<Vec<u8>> {
        let (tx, mut rx) = mpsc::channel::<Result<(usize, Vec<u8>)>>(32);
        let mut workers = vec![];

        // Spawn worker tasks for each peer
        for peer_addr in &self.peers {
            let tx = tx.clone();
            let pieces_queue = self.pieces_queue.clone();
            let torrent = self.torrent.clone();
            let info_hash = self.info_hash;
            let peer_addr = peer_addr.clone();

            let mut peer = Peer::new(peer_addr.parse()?, info_hash);

            let worker = tokio::spawn(async move {
                for _ in 0..MAX_PEER_RETRIES {
                    if let Err(e) = timeout(PEER_TIMEOUT, async {
                        if let Err(e) = peer.connect().await {
                            error!("Failed to connect to peer {}: {}", peer_addr, e);
                            return Err(e);
                        }
                        Ok(())
                    })
                    .await
                    {
                        error!("Connection timeout for peer {}: {}", peer_addr, e);
                        continue;
                    }

                    loop {
                        let mut pending_requests = 0;
                        let mut piece_indices = Vec::new();

                        // Get multiple pieces to pipeline requests
                        while pending_requests < MAX_PENDING_REQUESTS {
                            let piece_index = {
                                let mut queue = pieces_queue.lock().await;
                                queue.pop()
                            };

                            match piece_index {
                                Some(index) => {
                                    piece_indices.push(index);
                                    pending_requests += 1;
                                }
                                None => break,
                            }
                        }

                        if piece_indices.is_empty() {
                            break;
                        }

                        // Download pieces in parallel
                        for index in piece_indices {
                            let piece_length = torrent.info.piece_size(index);
                            match timeout(PEER_TIMEOUT, peer.download_piece(index, piece_length))
                                .await
                            {
                                Ok(Ok(data)) => {
                                    if tx.send(Ok((index, data))).await.is_err() {
                                        return;
                                    }
                                }
                                Ok(Err(e)) => {
                                    error!("Failed to download piece {}: {}", index, e);
                                    let mut queue = pieces_queue.lock().await;
                                    queue.push(index);
                                }
                                Err(_) => {
                                    error!("Timeout downloading piece {}", index);
                                    let mut queue = pieces_queue.lock().await;
                                    queue.push(index);
                                }
                            }
                        }
                    }
                }
            });
            workers.push(worker);
        }

        // Process completed pieces
        let mut file_data = vec![0u8; self.torrent.info.length];
        let mut completed = 0;
        let total_pieces = self.torrent.info.total_pieces();

        while completed < total_pieces {
            match rx.recv().await {
                Some(Ok((index, piece_data))) => {
                    // Verify piece hash
                    let mut hasher = sha1::Sha1::new();
                    hasher.update(&piece_data);
                    let hash = hasher.finalize();
                    let expected_hash = &self.torrent.info.pieces[index * 20..(index + 1) * 20];

                    if hash.as_slice() != expected_hash {
                        let mut queue = self.pieces_queue.lock().await;
                        queue.push(index);
                        continue;
                    }

                    // Calculate piece offset in file
                    let offset = index * self.torrent.info.piece_length;
                    let end = std::cmp::min(offset + piece_data.len(), self.torrent.info.length);
                    file_data[offset..end].copy_from_slice(&piece_data);
                    completed += 1;

                    info!(
                        "Downloaded piece {}/{} ({}%)",
                        completed,
                        total_pieces,
                        (completed * 100) / total_pieces
                    );
                }
                Some(Err(e)) => error!("Worker error: {}", e),
                None => break,
            }
        }

        // Wait for all workers to complete
        for worker in workers {
            worker.await?;
        }

        Ok(file_data)
    }
}
