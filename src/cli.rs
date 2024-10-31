use clap::{Parser, Subcommand};

/// Command line arguments for the bittorrent client implementation
#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

/// Available commands for the bittorrent client implementation
#[derive(Subcommand, Debug)]
pub enum Command {
    /// Decode a bencoded string
    Decode {
        /// The bencoded string to decode
        input: String,
    },
    /// Encode a string to a bencoded string
    Encode {
        /// The string to encode
        input: String,
    },
    /// Info about a torrent file
    Info {
        /// The path to the torrent file
        path: String,
    },
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
