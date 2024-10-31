use clap::{Parser, Subcommand};

/// Command line arguments for the Lox interpreter
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
    /// Decode a bencoded file
    DecodeFile {
        /// Path to the bencoded file
        input_path: String,
        /// Output path for the decoded file
        output_path: String,
    },
    /// Encode a file to a bencoded file
    EncodeFile {
        /// Path to the file to encode
        input_path: String,
        /// Output path for the bencoded file
        output_path: String,
    },
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
