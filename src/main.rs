use tracing::info;

mod cli;
mod decode;

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = cli::Args::parse();

    info!(
        "Command: {:?}, Filename: {:?}",
        args.command,
        args.bencoded_file.display()
    );

    match args.command {
        cli::Command::Decode => {
            let encoded_value = std::fs::read_to_string(&args.bencoded_file).unwrap();
            let decoded_value = decode::decode_bencoded_value(&encoded_value);
            println!("{}", decoded_value.to_string());
        } // _ => {
          //     println!("unknown command: {:?}", args.command)
          // }
    }
}
