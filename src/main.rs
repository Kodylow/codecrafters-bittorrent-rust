use anyhow::Result;
use bencode::Bencode;
use once_cell::sync::Lazy;
use torrent::{
    download::Downloader,
    metainfo::TorrentMetainfo,
    peer::{PeerConfig, PeerId},
};
use tracing::info;
use utils::{peer_id_to_string, serialize_peer_id};

pub mod bencode;
pub mod cli;
pub mod torrent;
pub mod utils;

pub const PROTOCOL: &str = "BitTorrent protocol";
pub static PEER_ID: Lazy<PeerId> = Lazy::new(utils::generate_peer_id);

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
            let announce = torrent
                .announce
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("No tracker URL"))?;
            info!("Tracker URL: {}", announce);
            let info_hash = torrent.info_hash()?;
            info!("Info Hash: {}", hex::encode(info_hash));

            let peers = torrent::tracker::get_peers(
                announce,
                info_hash,
                torrent.info.as_ref().map(|i| i.length as u64),
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
            let peer_id = peer
                .peer_id
                .ok_or_else(|| anyhow::anyhow!("No peer ID received"))?;
            println!("Peer ID: {}", serialize_peer_id(&peer_id));
        }
        cli::Command::DownloadPiece {
            output,
            path,
            piece_index,
        } => handle_download_piece(output, path, piece_index).await?,
        cli::Command::Download { output, path } => handle_download(output, path).await?,
        cli::Command::MagnetParse { magnet_link } => handle_magnet_parse(magnet_link).await?,
        cli::Command::MagnetHandshake { magnet_link } => {
            handle_magnet_handshake(magnet_link).await?
        }
    }

    Ok(())
}

async fn handle_download_piece(output: String, path: String, piece_index: usize) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let torrent = TorrentMetainfo::from_bytes(&bytes)?;

    let downloader = Downloader::new(torrent).await?;
    let piece_data = downloader.download_piece(piece_index).await?;

    tokio::fs::write(output, piece_data).await?;
    info!("Successfully downloaded and verified piece {}", piece_index);
    Ok(())
}

async fn handle_download(output: String, path: String) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let torrent = TorrentMetainfo::from_bytes(&bytes)?;

    let downloader = Downloader::new(torrent).await?;
    downloader.download_all(&output).await?;

    Ok(())
}

async fn handle_magnet_parse(magnet_link: String) -> Result<()> {
    let magnet = torrent::magnet_link::MagnetLink::parse(&magnet_link)?;
    println!("{}", magnet);
    Ok(())
}

async fn handle_magnet_handshake(magnet_link: String) -> Result<()> {
    let magnet = torrent::magnet_link::MagnetLink::parse(&magnet_link)?;
    let peer_id = magnet.perform_handshake().await?;
    println!("Peer ID: {}", peer_id_to_string(&peer_id));
    Ok(())
}
