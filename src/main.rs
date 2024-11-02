use anyhow::Result;
use bencode::Bencode;
use sha1::Digest;
use torrent::{message::Message, metainfo::TorrentMetainfo, peer::PeerConfig};
use tracing::info;

pub mod bencode;
pub mod cli;
pub mod torrent;

pub const PROTOCOL: &str = "BitTorrent protocol";
pub const PEER_ID: torrent::peer::PeerId = *b"00112233445566778899";

#[tokio::main]
async fn main() -> Result<()> {
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
                Some(torrent::tracker::TrackerConfig::default()),
            )
            .await?;

            for peer in peers {
                println!("{}", peer);
            }
        }
        cli::Command::Handshake { path, peer } => {
            info!("Performing handshake with peer: {}", peer);
            let bytes = std::fs::read(path)?;
            let torrent = TorrentMetainfo::from_bytes(&bytes)?;
            let info_hash = torrent.info_hash()?;
            let peer_config = PeerConfig {
                info_hash,
                ..Default::default()
            };

            let mut peer = torrent::peer::Peer::new(peer.parse()?, peer_config);
            peer.connect().await?;
            println!("Peer ID: {}", hex::encode(peer.peer_id.unwrap()));
        }
        cli::Command::DownloadPiece {
            output,
            path,
            piece_index,
        } => handle_download_piece(output, path, piece_index).await?,
        cli::Command::Download { output, path } => handle_download(output, path).await?,
    }
    Ok(())
}

async fn handle_download_piece(output: String, path: String, piece_index: usize) -> Result<()> {
    info!(
        "Downloading piece {} from torrent file: {}",
        piece_index, path
    );
    let bytes = std::fs::read(path)?;
    let torrent = TorrentMetainfo::from_bytes(&bytes)?;
    let info_hash = torrent.info_hash()?;
    let peer_config = PeerConfig {
        info_hash,
        ..Default::default()
    };

    let peers = torrent::tracker::get_peers(
        &torrent.announce,
        info_hash,
        torrent.info.length as u64,
        Some(torrent::tracker::TrackerConfig::default()),
    )
    .await?;

    let mut peer = torrent::peer::Peer::new(peers[0].to_string().parse()?, peer_config);
    peer.connect().await?;

    let piece_length = if piece_index == torrent.info.total_pieces() - 1 {
        torrent.info.piece_size(piece_index)
    } else {
        torrent.info.piece_length
    };

    let piece_data = peer.download_piece(piece_index, piece_length).await?;

    let mut hasher = sha1::Sha1::new();
    hasher.update(&piece_data);
    let hash = hasher.finalize();
    let expected_hash = &torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];

    if hash.as_slice() != expected_hash {
        return Err(anyhow::anyhow!("Piece hash verification failed"));
    }

    tokio::fs::write(output, piece_data).await?;
    info!("Successfully downloaded and verified piece {}", piece_index);
    Ok(())
}

async fn handle_download(output: String, path: String) -> Result<()> {
    info!("Downloading torrent file: {} to {}", path, output);
    let bytes = std::fs::read(path)?;
    if let Ok(contents) = String::from_utf8(bytes.clone()) {
        println!("File contents: {}", contents);
    } else {
        println!("File contents (binary): {:?}", bytes);
    }
    let torrent = TorrentMetainfo::from_bytes(&bytes)?;
    let info_hash = torrent.info_hash()?;

    let peers = torrent::tracker::get_peers(
        &torrent.announce,
        info_hash,
        torrent.info.length as u64,
        Some(torrent::tracker::TrackerConfig::default()),
    )
    .await?;

    if peers.is_empty() {
        return Err(anyhow::anyhow!("No peers available"));
    }

    let peer_config = PeerConfig {
        info_hash,
        ..Default::default()
    };

    let mut file_data = Vec::with_capacity(torrent.info.length);
    let mut current_peer_index = 0;

    'piece_loop: for piece_index in 0..torrent.info.total_pieces() {
        info!(
            "Downloading piece {}/{}",
            piece_index + 1,
            torrent.info.total_pieces()
        );

        // Try up to 3 times per piece
        for attempt in 0..3 {
            if attempt > 0 {
                info!("Retrying piece {} (attempt {})", piece_index, attempt + 1);
            }

            // Try different peers if available
            while current_peer_index < peers.len() {
                let mut peer = torrent::peer::Peer::new(
                    peers[current_peer_index].to_string().parse()?,
                    peer_config.clone(),
                );

                match download_piece_from_peer(&mut peer, piece_index, &torrent).await {
                    Ok(piece_data) => {
                        file_data.extend_from_slice(&piece_data);
                        continue 'piece_loop;
                    }
                    Err(e) => {
                        info!("Failed to download from peer {}: {}", current_peer_index, e);
                        current_peer_index += 1;
                    }
                }
            }

            // Reset peer index if we've tried all peers
            current_peer_index = 0;
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        return Err(anyhow::anyhow!(
            "Failed to download piece {} after multiple attempts",
            piece_index
        ));
    }

    tokio::fs::write(output, file_data).await?;
    info!("Download completed successfully");
    Ok(())
}

async fn download_piece_from_peer(
    peer: &mut torrent::peer::Peer,
    piece_index: usize,
    torrent: &TorrentMetainfo,
) -> Result<Vec<u8>> {
    peer.connect().await?;

    // Wait for and verify bitfield
    let bitfield = match peer.receive_message().await? {
        Message::Bitfield(b) => b,
        _ => return Err(anyhow::anyhow!("Expected bitfield message")),
    };

    // Verify piece availability in bitfield
    let byte_index = piece_index / 8;
    let bit_index = 7 - (piece_index % 8);
    if byte_index >= bitfield.len() || (bitfield[byte_index] & (1 << bit_index)) == 0 {
        return Err(anyhow::anyhow!("Peer does not have piece {}", piece_index));
    }

    // Send interested message
    peer.send_message(Message::Interested).await?;

    // Wait for unchoke
    match peer.receive_message().await? {
        Message::Unchoke => (),
        _ => return Err(anyhow::anyhow!("Expected unchoke message")),
    };

    let piece_length = torrent.info.piece_size(piece_index);
    let piece_data = peer.download_piece(piece_index, piece_length).await?;

    // Verify piece hash
    let mut hasher = sha1::Sha1::new();
    hasher.update(&piece_data);
    let hash = hasher.finalize();
    let expected_hash = &torrent.info.pieces[piece_index * 20..(piece_index + 1) * 20];

    if hash.as_slice() != expected_hash {
        return Err(anyhow::anyhow!("Piece hash verification failed"));
    }

    Ok(piece_data)
}
