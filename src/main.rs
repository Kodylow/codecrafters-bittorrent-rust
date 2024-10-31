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
        "Command: {:?}, Bencoded value: {:?}",
        args.command, args.bencoded_value
    );

    match args.command {
        cli::Command::Decode => {
            let decoded_value = decode::decode_bencoded_value(&args.bencoded_value);
            println!("{}", decoded_value.to_string());
        } // _ => {
          //     println!("unknown command: {:?}", args.command)
          // }
    }
}
