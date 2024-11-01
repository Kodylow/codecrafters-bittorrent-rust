#[derive(Debug)]
pub enum Message {
    KeepAlive,
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request {
        index: u32,
        begin: u32,
        length: u32,
    },
    Piece {
        index: u32,
        begin: u32,
        block: Vec<u8>,
    },
    Cancel {
        index: u32,
        begin: u32,
        length: u32,
    },
}

impl Default for Message {
    fn default() -> Self {
        Message::KeepAlive
    }
}

impl Message {
    pub fn to_bytes(&self) -> Vec<u8> {
        // TODO: Implement message serialization
        unimplemented!()
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        // TODO: Implement message deserialization
        unimplemented!()
    }
}
