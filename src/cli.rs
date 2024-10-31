use std::path::PathBuf;

use clap::{Parser, ValueEnum};

/// Available commands for the bittorrent client implementation
///
/// Usage: `bittorrent decode <bencoded_value>`
///
/// - `Decode`: Decode the input file and print the decoded value
#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum Command {
    /// Decode input and print decoded value
    Decode,
}

/// Command line arguments for the Lox interpreter
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The command to execute (decode)
    #[arg(value_enum)]
    pub command: Command,

    /// Bencoded value to decode
    #[arg(value_name = "BENCODED_FILE")]
    pub bencoded_file: PathBuf,
}

impl Args {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
