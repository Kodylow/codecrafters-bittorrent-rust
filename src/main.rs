use anyhow::Result;
use bencode::Bencode;
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

            let peer = torrent::peer::Peer::new(peer.parse()?, info_hash);
            println!("Peer ID: {}", hex::encode(peer.peer_id.unwrap()));
        }
    }
    Ok(())
}
