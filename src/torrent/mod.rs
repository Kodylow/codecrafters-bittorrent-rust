mod file;
mod info;

use anyhow::Result;
use info::TorrentInfo;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

#[derive(Debug, Serialize, Deserialize)]
pub struct Torrent {
    pub announce: String,
    pub info: TorrentInfo,
}

impl Torrent {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        serde_bencode::from_bytes(bytes).map_err(|_| anyhow::anyhow!("Failed to parse torrent"))
    }

    pub fn info_hash(&self) -> [u8; 20] {
        let info_encoded = serde_bencode::to_bytes(&self.info)
            .expect("serialization of valid info dict should never fail");
        let mut hasher = Sha1::new();
        hasher.update(&info_encoded);
        hasher.finalize().into()
    }
}
