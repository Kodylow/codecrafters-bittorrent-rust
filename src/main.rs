use anyhow::Result;
use bencode::Bencode;
use sha1::Digest;
use torrent::metainfo::TorrentMetainfo;
use tracing::info;

pub mod bencode;
pub mod cli;
pub mod torrent;

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

            let mut peer = torrent::peer::Peer::new(peer.parse()?, info_hash);
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

    let peers =
        torrent::tracker::get_peers(&torrent.announce, info_hash, torrent.info.length as u64)
            .await?;

    let mut peer = torrent::peer::Peer::new(peers[0].to_string().parse()?, info_hash);
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
    let torrent = TorrentMetainfo::from_bytes(&bytes)?;
    let info_hash = torrent.info_hash()?;

    let peers =
        torrent::tracker::get_peers(&torrent.announce, info_hash, torrent.info.length as u64)
            .await?;

    if peers.is_empty() {
        return Err(anyhow::anyhow!("No peers available"));
    }

    let peer_addrs: Vec<String> = peers.iter().map(|p| p.to_string()).collect();

    let manager = torrent::download::DownloadManager::new(torrent, peer_addrs)?;
    let file_data = manager.download().await?;

    tokio::fs::write(output, file_data).await?;
    info!("Successfully downloaded file");
    Ok(())
}
