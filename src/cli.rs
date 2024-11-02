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
    /// Peers for the torrent
    Peers {
        /// The path to the torrent file
        path: String,
    },
    /// Handshake with a peer
    Handshake {
        /// The path to the torrent file
        path: String,
        /// The peer address in format IP:PORT
        peer: String,
    },
    /// Download a piece from the torrent
    #[command(name = "download_piece")]
    DownloadPiece {
        /// The path to the torrent file
        path: String,
        /// The index of the piece to download
        piece_index: usize,
        /// The output file path
        #[arg(short)]
        output: String,
    },
    /// Download the complete torrent
    Download {
        /// The output file path
        #[arg(short)]
        output: String,

        /// The path to the torrent file
        path: String,
    },
    /// Parse a magnet link
    #[command(name = "magnet_parse")]
    MagnetParse {
        /// The magnet link
        magnet_link: String,
    },
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
