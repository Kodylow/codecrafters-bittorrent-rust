//! BitTorrent metainfo file parser and utilities.
//!
//! A torrent file (also known as a metainfo file) contains metadata about files to be shared
//! in the BitTorrent protocol. This module provides functionality to parse and work with these files.
//!
//! # Structure
//!
//! The torrent file is a bencoded dictionary containing:
//!
//! - `announce`: URL of the tracker server that coordinates peers
//! - `info`: Dictionary containing core metadata about the file(s):
//!   - `name`: Suggested filename/directory name
//!   - `length`: Total size in bytes (single-file torrents only)
//!   - `piece length`: Number of bytes per piece
//!   - `pieces`: Concatenated SHA-1 hashes of all pieces
//!
//! This implementation currently only supports single-file torrents. Multi-file torrents
//! have a different structure in the info dictionary and are not supported.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fmt;

use crate::bencode::{bvalue::BValue, Bencode};

use super::magnet_link::MagnetLink;

/// Represents a parsed BitTorrent metainfo file.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentMetainfo {
    /// URL of the tracker server
    pub announce: String,
    /// Core metadata about the torrent content
    pub info: TorrentInfo,
}

impl TorrentMetainfo {
    /// Parse a torrent file from its raw bytes.
    ///
    /// # Arguments
    ///
    /// * `bytes` - Raw bytes of the torrent file
    ///
    /// # Returns
    ///
    /// The parsed `TorrentMetainfo` structure wrapped in a `Result`
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let bvalue = Bencode::decode_bytes(bytes)?;
        match bvalue {
            BValue::Dict(dict) => {
                let announce = match dict.get("announce") {
                    Some(BValue::String(s)) => String::from_utf8_lossy(s).into_owned(),
                    _ => return Err(anyhow::anyhow!("Missing or invalid announce field")),
                };

                let info = match dict.get("info") {
                    Some(BValue::Dict(info_dict)) => {
                        let name = match info_dict.get("name") {
                            Some(BValue::String(s)) => String::from_utf8_lossy(s).into_owned(),
                            _ => return Err(anyhow::anyhow!("Missing or invalid name field")),
                        };

                        let length = match info_dict.get("length") {
                            Some(BValue::Integer(n)) => *n as usize,
                            _ => return Err(anyhow::anyhow!("Missing or invalid length field")),
                        };

                        let piece_length = match info_dict.get("piece length") {
                            Some(BValue::Integer(n)) => *n as usize,
                            _ => {
                                return Err(anyhow::anyhow!(
                                    "Missing or invalid piece length field"
                                ))
                            }
                        };

                        let pieces = match info_dict.get("pieces") {
                            Some(BValue::String(s)) => s.clone(),
                            _ => return Err(anyhow::anyhow!("Missing or invalid pieces field")),
                        };

                        TorrentInfo {
                            name,
                            length,
                            piece_length,
                            pieces,
                        }
                    }
                    _ => return Err(anyhow::anyhow!("Missing or invalid info dictionary")),
                };

                Ok(TorrentMetainfo { announce, info })
            }
            _ => Err(anyhow::anyhow!("Invalid torrent file format")),
        }
    }

    /// Parse a magnet link.
    pub async fn from_magnet(magnet_link: &str) -> Result<Self> {
        let magnet = MagnetLink::parse(magnet_link)?;

        let torrent_info = TorrentInfo {
            name: magnet.name,
            length: 0,
            piece_length: 0,
            pieces: vec![],
        };

        Ok(TorrentMetainfo {
            announce: magnet.tracker,
            info: torrent_info,
        })
    }

    /// Calculate the SHA-1 hash of the bencoded info dictionary.
    ///
    /// This hash uniquely identifies the torrent and is used in peer protocol
    /// handshakes and tracker communications.
    ///
    /// # Returns
    ///
    /// A 20-byte array containing the SHA-1 hash
    pub fn info_hash(&self) -> Result<[u8; 20]> {
        let info_bvalue = BValue::from(&self.info);
        let encoded = info_bvalue.to_bytes()?;
        let mut hasher = Sha1::new();
        hasher.update(&encoded);
        let hash = hasher.finalize();
        Ok(hash.into())
    }
}

impl fmt::Display for TorrentMetainfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Tracker URL: {}\n", self.announce)?;
        write!(f, "Length: {}\n", self.info.length)?;
        if let Ok(hash) = self.info_hash() {
            write!(f, "Info Hash: {}\n", hex::encode(hash))?;
        }
        write!(f, "Piece Length: {}\n", self.info.piece_length)?;
        writeln!(f, "Piece Hashes:")?;
        for hash in self.info.piece_hashes() {
            writeln!(f, "{}", hex::encode(hash))?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct TorrentInfo {
    pub name: String,
    pub length: usize,
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    pub pieces: Vec<u8>,
}

impl TorrentInfo {
    pub fn piece_hashes(&self) -> Vec<[u8; 20]> {
        self.pieces
            .chunks_exact(20)
            .map(|chunk| chunk.try_into().unwrap())
            .collect()
    }

    pub fn total_pieces(&self) -> usize {
        self.pieces.len() / 20
    }

    pub fn piece_size(&self, piece_index: usize) -> usize {
        if piece_index == self.total_pieces() - 1 {
            let remainder = self.length % self.piece_length;
            if remainder == 0 {
                self.piece_length
            } else {
                remainder
            }
        } else {
            self.piece_length
        }
    }
}
