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
const MAX_PIECE_RETRIES: usize = 3;

#[derive(Debug)]
struct PieceWork {
    index: usize,
    length: usize,
    retries: usize,
}

pub struct DownloadManager {
    torrent: Arc<TorrentMetainfo>,
    peers: Vec<String>,
    info_hash: [u8; 20],
    pieces_queue: Arc<Mutex<Vec<PieceWork>>>,
    completed_pieces: Arc<Mutex<Vec<Option<Vec<u8>>>>>,
}

impl DownloadManager {
    pub fn new(torrent: TorrentMetainfo, peers: Vec<String>) -> Result<Self> {
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
        })
    }

    async fn worker_task(
        peer_addr: String,
        info_hash: [u8; 20],
        pieces_queue: Arc<Mutex<Vec<PieceWork>>>,
        completed_pieces: Arc<Mutex<Vec<Option<Vec<u8>>>>>,
        torrent: Arc<TorrentMetainfo>,
        tx: mpsc::Sender<Result<()>>,
    ) {
        let mut peer = Peer::new(peer_addr.parse().unwrap(), info_hash);

        'retry: for _ in 0..MAX_PEER_RETRIES {
            match timeout(PEER_TIMEOUT, peer.connect()).await {
                Ok(Ok(())) => {
                    loop {
                        // Get multiple pieces to work on
                        let work_batch = {
                            let mut queue = pieces_queue.lock().await;
                            if queue.is_empty() {
                                break 'retry;
                            }

                            // Take up to MAX_PENDING_REQUESTS pieces with lowest retry counts
                            let mut batch = Vec::new();
                            while batch.len() < MAX_PENDING_REQUESTS && !queue.is_empty() {
                                if let Some(pos) = queue
                                    .iter()
                                    .enumerate()
                                    .min_by_key(|(_, work)| work.retries)
                                    .map(|(i, _)| i)
                                {
                                    batch.push(queue.remove(pos));
                                }
                            }
                            batch
                        };

                        if work_batch.is_empty() {
                            break 'retry;
                        }

                        // Download pieces sequentially instead of parallel
                        for piece_work in work_batch {
                            match timeout(
                                PEER_TIMEOUT,
                                peer.download_piece(piece_work.index, piece_work.length),
                            )
                            .await
                            {
                                Ok(Ok(data)) => {
                                    // Verify piece hash
                                    let mut hasher = sha1::Sha1::new();
                                    hasher.update(&data);
                                    let hash = hasher.finalize();
                                    let expected_hash = &torrent.info.pieces
                                        [piece_work.index * 20..(piece_work.index + 1) * 20];

                                    if hash.as_slice() == expected_hash {
                                        let mut completed = completed_pieces.lock().await;
                                        completed[piece_work.index] = Some(data);
                                        info!(
                                            "Downloaded piece {}/{}",
                                            piece_work.index + 1,
                                            torrent.info.total_pieces()
                                        );
                                        continue;
                                    }
                                }
                                _ => {}
                            }

                            // Handle failed piece download
                            let mut failed_piece = piece_work;
                            failed_piece.retries += 1;
                            if failed_piece.retries < MAX_PIECE_RETRIES {
                                let mut queue = pieces_queue.lock().await;
                                queue.push(failed_piece);
                            }
                        }
                    }
                }
                _ => continue,
            }
        }

        tx.send(Ok(())).await.ok();
    }

    pub async fn download(&self) -> Result<Vec<u8>> {
        let (tx, mut rx) = mpsc::channel(32);
        let mut workers = vec![];

        // Spawn worker for each peer
        for peer_addr in &self.peers {
            let tx = tx.clone();
            let pieces_queue = self.pieces_queue.clone();
            let completed_pieces = self.completed_pieces.clone();
            let torrent = self.torrent.clone();
            let info_hash = self.info_hash;
            let peer_addr = peer_addr.clone();

            let worker = tokio::spawn(Self::worker_task(
                peer_addr,
                info_hash,
                pieces_queue,
                completed_pieces,
                torrent,
                tx,
            ));
            workers.push(worker);
        }

        // Wait for all pieces to complete
        let mut completed = 0;
        let total_pieces = self.torrent.info.total_pieces();

        while completed < total_pieces {
            match rx.recv().await {
                Some(Ok(())) => {
                    let pieces = self.completed_pieces.lock().await;
                    completed = pieces.iter().filter(|p| p.is_some()).count();
                }
                Some(Err(e)) => error!("Worker error: {}", e),
                None => break,
            }
        }

        // Combine all pieces
        let mut file_data = vec![0u8; self.torrent.info.length];
        let pieces = self.completed_pieces.lock().await;

        let mut offset = 0;
        for (i, piece) in pieces.iter().enumerate() {
            if let Some(data) = piece {
                let piece_size = self.torrent.info.piece_size(i);
                file_data[offset..offset + piece_size].copy_from_slice(&data[..piece_size]);
                offset += piece_size;
            }
        }

        // Wait for workers to complete
        for worker in workers {
            worker.await?;
        }

        Ok(file_data)
    }
}
