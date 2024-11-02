use anyhow::Result;
use bencode::Bencode;
use sha1::Digest;
use torrent::metainfo::TorrentMetainfo;
use tracing::info;

pub mod bencode;
pub mod cli;
pub mod torrent;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = cli::Args::parse();

    match args.command {
        cli::Command::Decode { input } => {
            info!("Decoding input: {}", input);
            let decoded_value = Bencode::decode(&input)?;
            println!("{}", decoded_value);
        }
        cli::Command::Encode { input } => {
            info!("Encoding input: {}", input);
            let encoded_value = Bencode::encode(&serde_json::Value::from(input))?;
            println!("{}", encoded_value);
        }
        cli::Command::Info { path } => {
            info!("Getting info about torrent file: {}", path);
            let bytes = std::fs::read(path)?;
            let torrent_info = TorrentMetainfo::from_bytes(&bytes)?;
            println!("{}", torrent_info);
        }
        cli::Command::Peers { path } => {
            info!("Getting peers for torrent file: {}", path);
            let bytes = std::fs::read(path)?;
            let torrent = TorrentMetainfo::from_bytes(&bytes)?;
            info!("Tracker URL: {}", torrent.announce);
            let info_hash = torrent.info_hash()?;
            info!("Info Hash: {}", hex::encode(info_hash));

            let peers = torrent::tracker::get_peers(
                &torrent.announce,
                info_hash,
                torrent.info.length as u64,
            )?;

            for peer in peers {
                println!("{}", peer);
            }
        }
        cli::Command::Handshake { path, peer } => {
            info!("Performing handshake with peer: {}", peer);
            let bytes = std::fs::read(path)?;
            let torrent = TorrentMetainfo::from_bytes(&bytes)?;
            let info_hash = torrent.info_hash()?;

            let mut peer = torrent::peer::Peer::new(peer.parse()?, info_hash);
            peer.connect()?;
            println!("Peer ID: {}", hex::encode(peer.peer_id.unwrap()));
        }
        cli::Command::DownloadPiece {
            output,
            path,
            piece_index,
        } => {
            info!(
                "Downloading piece {} from torrent file: {}",
                piece_index, path
            );
            let bytes = std::fs::read(path)?;
            let torrent = TorrentMetainfo::from_bytes(&bytes)?;
            let info_hash = torrent.info_hash()?;

            // Get peers
            let peers = torrent::tracker::get_peers(
                &torrent.announce,
                info_hash,
                torrent.info.length as u64,
            )?;

            // Connect to first peer
            let mut peer = torrent::peer::Peer::new(peers[0].to_string().parse()?, info_hash);
            peer.connect()?;

            // Download piece
            let piece_length = if piece_index == torrent.info.total_pieces() - 1 {
                torrent.info.piece_size(piece_index)
            } else {
                torrent.info.piece_length
            };

            let piece_data = peer.download_piece(piece_index, piece_length)?;

            // Verify piece hash
            let mut hasher = sha1::Sha1::new();
            hasher.update(&piece_data);
            let hash = hasher.finalize();
            let expected_hash = &torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];

            if hash.as_slice() != expected_hash {
                return Err(anyhow::anyhow!("Piece hash verification failed"));
            }

            // Save piece to file
            std::fs::write(output, piece_data)?;
            info!("Successfully downloaded and verified piece {}", piece_index);
        }
        cli::Command::Download { output, path } => {
            info!("Downloading torrent file: {} to {}", path, output);
            let bytes = std::fs::read(path)?;
            let torrent = TorrentMetainfo::from_bytes(&bytes)?;
            let info_hash = torrent.info_hash()?;

            // Get peers
            let peers = torrent::tracker::get_peers(
                &torrent.announce,
                info_hash,
                torrent.info.length as u64,
            )?;

            if peers.is_empty() {
                return Err(anyhow::anyhow!("No peers available"));
            }

            // Connect to first peer
            let mut peer = torrent::peer::Peer::new(peers[0].to_string().parse()?, info_hash);
            peer.connect()?;

            // Download all pieces
            let mut file_data = Vec::with_capacity(torrent.info.length);
            let total_pieces = torrent.info.total_pieces();

            for piece_index in 0..total_pieces {
                let piece_length = torrent.info.piece_size(piece_index);
                info!(
                    "Downloading piece {}/{} (size: {})",
                    piece_index + 1,
                    total_pieces,
                    piece_length
                );

                let piece_data = peer.download_piece(piece_index, piece_length)?;

                // Verify piece hash
                let mut hasher = sha1::Sha1::new();
                hasher.update(&piece_data);
                let hash = hasher.finalize();
                let expected_hash = &torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];

                if hash.as_slice() != expected_hash {
                    return Err(anyhow::anyhow!(
                        "Piece {} hash verification failed",
                        piece_index
                    ));
                }

                file_data.extend_from_slice(&piece_data);
            }

            // Save complete file
            std::fs::write(output, file_data)?;
            info!("Successfully downloaded file");
        }
    }
    Ok(())
}
