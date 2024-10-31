use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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
