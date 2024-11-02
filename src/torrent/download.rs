use anyhow::Result;
use sha1::Digest;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{timeout, Duration};
use tracing::{error, info};

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

    async fn worker_task(
        peer_addr: String,
        info_hash: [u8; 20],
        pieces_queue: Arc<Mutex<Vec<PieceWork>>>,
        completed_pieces: Arc<Mutex<Vec<Option<Vec<u8>>>>>,
        torrent: Arc<TorrentMetainfo>,
        tx: mpsc::Sender<Result<()>>,
        peer_states: Arc<Mutex<HashMap<String, PeerState>>>,
        config: DownloadConfig,
    ) -> Result<()> {
        let peer_config = PeerConfig {
            info_hash: info_hash.into(),
            ..Default::default()
        };
        let mut peer = Peer::new(peer_addr.parse()?, peer_config);

        'retry: for _ in 0..config.peer_retries {
            match timeout(config.peer_timeout, peer.connect()).await {
                Ok(Ok(())) => {
                    let mut consecutive_failures = 0;

                    loop {
                        if consecutive_failures >= 3 {
                            break; // Drop this peer connection and retry
                        }

                        let work_batch = {
                            let mut queue = pieces_queue.lock().await;
                            if queue.is_empty() {
                                break 'retry;
                            }

                            let mut batch = Vec::new();
                            while batch.len() < config.max_pending && !queue.is_empty() {
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

                        for piece_work in work_batch {
                            match timeout(
                                config.peer_timeout,
                                peer.download_piece(piece_work.index, piece_work.length),
                            )
                            .await
                            {
                                Ok(Ok(data)) => {
                                    let mut hasher = sha1::Sha1::new();
                                    hasher.update(&data);
                                    let hash = hasher.finalize();
                                    let expected_hash = &torrent.info.pieces
                                        [piece_work.index * 20..(piece_work.index + 1) * 20];

                                    if hash.as_slice() == expected_hash {
                                        // Update peer stats
                                        let mut states = peer_states.lock().await;
                                        if let Some(state) = states.get_mut(&peer_addr) {
                                            state.successful_pieces += 1;
                                            state.last_success = std::time::Instant::now();
                                        }
                                        consecutive_failures = 0;

                                        let mut completed = completed_pieces.lock().await;
                                        completed[piece_work.index] = Some(data);
                                        info!(
                                            "Downloaded piece {}/{} from {}",
                                            piece_work.index + 1,
                                            torrent.info.total_pieces(),
                                            peer_addr
                                        );
                                        continue;
                                    }
                                }
                                _ => {
                                    consecutive_failures += 1;
                                    let mut states = peer_states.lock().await;
                                    if let Some(state) = states.get_mut(&peer_addr) {
                                        state.failed_pieces += 1;
                                    }
                                }
                            }

                            let mut failed_piece = piece_work;
                            failed_piece.retries += 1;
                            if failed_piece.retries < config.piece_retries {
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
        Ok(())
    }

    pub async fn download(&self) -> Result<Vec<u8>> {
        let (tx, mut rx) = mpsc::channel(32);
        let mut workers = vec![];
        let peer_states = Arc::new(Mutex::new(HashMap::new()));

        // Initialize peer states
        {
            let mut states = peer_states.lock().await;
            for peer in &self.peers {
                states.insert(
                    peer.clone(),
                    PeerState {
                        successful_pieces: 0,
                        failed_pieces: 0,
                        last_success: std::time::Instant::now(),
                    },
                );
            }
        }

        // Spawn initial workers
        for peer_addr in &self.peers {
            let tx = tx.clone();
            let pieces_queue = self.pieces_queue.clone();
            let completed_pieces = self.completed_pieces.clone();
            let torrent = self.torrent.clone();
            let info_hash = self.info_hash;
            let peer_addr = peer_addr.clone();
            let peer_states = peer_states.clone();
            let config = self.config.clone();

            let worker = tokio::spawn(Self::worker_task(
                peer_addr,
                info_hash,
                pieces_queue,
                completed_pieces,
                torrent,
                tx,
                peer_states,
                config,
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
            worker.await??;
        }

        Ok(file_data)
    }
}
