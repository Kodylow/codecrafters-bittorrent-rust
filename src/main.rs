use anyhow::Result;
use bencode::Bencode;
use torrent::metainfo::TorrentMetainfo;
use tracing::info;

pub mod bencode;
pub mod cli;
pub mod torrent;

// Usage: your_bittorrent.sh decode "<encoded_value>"
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
            let torrent = TorrentMetainfo::from_bytes(&bytes)?;
            println!("Tracker URL: {}", torrent.announce);
            println!("Length: {}", torrent.info.length);
            println!("Info Hash: {}", hex::encode(torrent.info_hash()?));
            println!("Piece Length: {}", torrent.info.piece_length);
            let hashes = torrent.info.piece_hashes();
            println!("Pieces Hashes:");
            for hash in hashes {
                println!("{}", hex::encode(hash));
            }
        }
    }
    Ok(())
}
