use rand::Rng;

use crate::torrent::peer::PeerId;

pub fn generate_peer_id() -> PeerId {
    let mut rng = rand::thread_rng();
    let mut id = [0u8; 20];
    rng.fill(&mut id);
    id
}
