use anyhow::Result;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use tracing::info;

use super::message::Message;

const PROTOCOL: &str = "BitTorrent protocol";
const PEER_ID: [u8; 20] = *b"00112233445566778899";

#[derive(Debug)]
pub struct Peer {
    addr: SocketAddr,
    stream: Option<TcpStream>,
    pub peer_id: Option<[u8; 20]>,
    info_hash: [u8; 20],
    am_choking: bool,
    am_interested: bool,
    peer_choking: bool,
    peer_interested: bool,
}

impl Peer {
    pub fn new(addr: SocketAddr, info_hash: [u8; 20]) -> Self {
        Self {
            addr,
            stream: None,
            peer_id: None,
            info_hash,
            am_choking: true,
            am_interested: false,
            peer_choking: true,
            peer_interested: false,
        }
    }

    pub fn connect(&mut self) -> Result<()> {
        info!("Connecting to peer: {}", self.addr);
        let stream = TcpStream::connect(self.addr)?;
        self.stream = Some(stream);
        self.handshake()?;
        Ok(())
    }

    fn handshake(&mut self) -> Result<()> {
        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Not connected"))?;

        // Construct handshake message
        let mut message = Vec::with_capacity(68);
        message.push(19);
        message.extend_from_slice(PROTOCOL.as_bytes());
        message.extend_from_slice(&[0u8; 8]);
        message.extend_from_slice(&self.info_hash);
        message.extend_from_slice(&PEER_ID);

        // Send handshake
        stream.write_all(&message)?;
        info!("Sent handshake message");

        // Read response
        let mut response = [0u8; 68];
        stream.read_exact(&mut response)?;
        info!("Received handshake response");

        // Verify protocol
        if response[1..20] != *PROTOCOL.as_bytes() {
            return Err(anyhow::anyhow!("Invalid protocol in handshake response"));
        }

        // Verify info hash
        if response[28..48] != self.info_hash {
            return Err(anyhow::anyhow!("Info hash mismatch in handshake"));
        }

        // Store peer ID
        let mut peer_id = [0u8; 20];
        peer_id.copy_from_slice(&response[48..68]);
        self.peer_id = Some(peer_id);

        Ok(())
    }

    pub fn send_message(&mut self, message: Message) -> Result<()> {
        // TODO: Implement message sending
        Ok(())
    }

    pub fn receive_message(&mut self) -> Result<Message> {
        // TODO: Implement message receiving
        Ok(Message::default())
    }
}
