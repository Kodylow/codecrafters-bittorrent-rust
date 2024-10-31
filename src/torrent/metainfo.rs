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

use crate::bencode::{bvalue::BValue, Bencode};

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

    /// Calculate the SHA-1 hash of the bencoded info dictionary.
    ///
    /// This hash uniquely identifies the torrent and is used in peer protocol
    /// handshakes and tracker communications.
    ///
    /// # Returns
    ///
    /// A 20-byte array containing the SHA-1 hash
    pub fn info_hash(&self) -> Result<[u8; 20]> {
        let info_encoded = Bencode::encode(&serde_json::to_value(&self.info)?)?;
        let mut hasher = Sha1::new();
        hasher.update(&info_encoded);
        let hash = hasher.finalize();
        Ok(hash.into())
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
