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
        let mut bytes = Vec::new();
        match self {
            Message::KeepAlive => return bytes,
            Message::Choke => {
                bytes.extend_from_slice(&1u32.to_be_bytes());
                bytes.push(0);
            }
            Message::Unchoke => {
                bytes.extend_from_slice(&1u32.to_be_bytes());
                bytes.push(1);
            }
            Message::Interested => {
                bytes.extend_from_slice(&1u32.to_be_bytes());
                bytes.push(2);
            }
            Message::NotInterested => {
                bytes.extend_from_slice(&1u32.to_be_bytes());
                bytes.push(3);
            }
            Message::Have(index) => {
                bytes.extend_from_slice(&5u32.to_be_bytes());
                bytes.push(4);
                bytes.extend_from_slice(&index.to_be_bytes());
            }
            Message::Bitfield(data) => {
                bytes.extend_from_slice(&(1 + data.len() as u32).to_be_bytes());
                bytes.push(5);
                bytes.extend_from_slice(data);
            }
            Message::Request {
                index,
                begin,
                length,
            } => {
                bytes.extend_from_slice(&13u32.to_be_bytes());
                bytes.push(6);
                bytes.extend_from_slice(&index.to_be_bytes());
                bytes.extend_from_slice(&begin.to_be_bytes());
                bytes.extend_from_slice(&length.to_be_bytes());
            }
            Message::Piece {
                index,
                begin,
                block,
            } => {
                bytes.extend_from_slice(&(9 + block.len() as u32).to_be_bytes());
                bytes.push(7);
                bytes.extend_from_slice(&index.to_be_bytes());
                bytes.extend_from_slice(&begin.to_be_bytes());
                bytes.extend_from_slice(block);
            }
            Message::Cancel {
                index,
                begin,
                length,
            } => {
                bytes.extend_from_slice(&13u32.to_be_bytes());
                bytes.push(8);
                bytes.extend_from_slice(&index.to_be_bytes());
                bytes.extend_from_slice(&begin.to_be_bytes());
                bytes.extend_from_slice(&length.to_be_bytes());
            }
        }
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        if bytes.is_empty() {
            return Ok(Message::KeepAlive);
        }

        let id = bytes[0];
        let payload = &bytes[1..];

        match id {
            0 => Ok(Message::Choke),
            1 => Ok(Message::Unchoke),
            2 => Ok(Message::Interested),
            3 => Ok(Message::NotInterested),
            4 => {
                let index = u32::from_be_bytes(payload[..4].try_into()?);
                Ok(Message::Have(index))
            }
            5 => Ok(Message::Bitfield(payload.to_vec())),
            6 => {
                let index = u32::from_be_bytes(payload[..4].try_into()?);
                let begin = u32::from_be_bytes(payload[4..8].try_into()?);
                let length = u32::from_be_bytes(payload[8..12].try_into()?);
                Ok(Message::Request {
                    index,
                    begin,
                    length,
                })
            }
            7 => {
                let index = u32::from_be_bytes(payload[..4].try_into()?);
                let begin = u32::from_be_bytes(payload[4..8].try_into()?);
                let block = payload[8..].to_vec();
                Ok(Message::Piece {
                    index,
                    begin,
                    block,
                })
            }
            8 => {
                let index = u32::from_be_bytes(payload[..4].try_into()?);
                let begin = u32::from_be_bytes(payload[4..8].try_into()?);
                let length = u32::from_be_bytes(payload[8..12].try_into()?);
                Ok(Message::Cancel {
                    index,
                    begin,
                    length,
                })
            }
            _ => Err(anyhow::anyhow!("Unknown message ID: {}", id)),
        }
    }
}
