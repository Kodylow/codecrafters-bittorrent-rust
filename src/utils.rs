use rand::Rng;

use crate::torrent::peer::PeerId;

pub fn generate_peer_id() -> PeerId {
    let mut rng = rand::thread_rng();
    let mut id = [0u8; 20];
    rng.fill(&mut id);
    id
}

pub fn serialize_peer_id(peer_id: &[u8]) -> String {
    peer_id
        .iter()
        .map(|&b| format!("{:02x}", b))
        .collect::<String>()
        .chars()
        .take(20)
        .collect()
}

pub fn peer_id_to_string(peer_id: &[u8]) -> String {
    peer_id.iter().map(|&b| format!("{:02x}", b)).collect()
}
