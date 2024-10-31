use anyhow::Result;
use bencode::Bencode;
use tracing::info;

mod bencode;
mod cli;

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = cli::Args::parse();

    match args.command {
        cli::Command::Decode { input } => {
            info!("Decoding input: {}", input);
            let decoded_value = Bencode::decode_str::<String>(&input)?;
            println!("{}", decoded_value);
        }
        cli::Command::Encode { input } => {
            info!("Encoding input: {}", input);
            let encoded_value = Bencode::encode_str(&input)?;
            println!("{}", encoded_value);
        }
        cli::Command::DecodeFile {
            input_path,
            output_path,
        } => {
            info!("Decoding file: {} to {}", input_path, output_path);
            let decoded_value = Bencode::decode_file(&input_path)?;
            println!("{:?}", decoded_value);
        }
        cli::Command::EncodeFile {
            input_path,
            output_path,
        } => {
            info!("Encoding file: {} to {}", input_path, output_path);
            Bencode::encode_file(&input_path, &output_path)?;
            info!("Encoded file: {} to {}", input_path, output_path);
        }
    }
    Ok(())
}
