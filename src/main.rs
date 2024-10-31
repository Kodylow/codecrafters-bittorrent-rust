use anyhow::Result;
use decode::BencodeDecoder;
use tracing::info;

mod cli;
mod decode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = cli::Args::parse();

    info!(
        "Command: {:?}, Bencoded value: {:?}",
        args.command, args.bencoded_value
    );

    match args.command {
        cli::Command::Decode => {
            let decoded_value = BencodeDecoder::new(&args.bencoded_value).parse()?;
            println!("{}", decoded_value.to_string());
        }
    }
    Ok(())
}
