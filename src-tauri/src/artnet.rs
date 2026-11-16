//! Minimal Art-Net (ArtDmx) sender.

use std::net::{SocketAddr, UdpSocket};

pub const ARTNET_PORT: u16 = 6454;

pub struct ArtNetSender {
    socket: UdpSocket,
    sequence: u8,
}

impl ArtNetSender {
    pub fn new() -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_broadcast(true)?;
        socket.set_nonblocking(true)?;
        Ok(Self { socket, sequence: 0 })
    }

    pub fn send(&mut self, ip: &str, port_address: u16, data: &[u8; 512]) -> std::io::Result<()> {
        self.sequence = self.sequence.wrapping_add(1).max(1);
        let pkt = art_dmx(self.sequence, port_address, data);
        let addr: SocketAddr = format!("{ip}:{ARTNET_PORT}")
            .parse()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
        self.socket.send_to(&pkt, addr).map(|_| ())
    }
}

pub fn art_dmx(sequence: u8, port_address: u16, data: &[u8; 512]) -> Vec<u8> {
    let mut p = Vec::with_capacity(18 + 512);
    p.extend_from_slice(b"Art-Net\0");
    p.extend_from_slice(&0x5000u16.to_le_bytes()); // OpDmx
    p.extend_from_slice(&14u16.to_be_bytes()); // protocol version
    p.push(sequence);
    p.push(0); // physical
    p.push((port_address & 0xff) as u8); // SubUni
    p.push(((port_address >> 8) & 0x7f) as u8); // Net
    p.extend_from_slice(&512u16.to_be_bytes());
    p.extend_from_slice(data);
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_layout() {
        let mut d = [0u8; 512];
        d[0] = 7;
        let p = art_dmx(3, 0x0123, &d);
        assert_eq!(&p[0..8], b"Art-Net\0");
        assert_eq!(p[8], 0x00);
        assert_eq!(p[9], 0x50);
        assert_eq!(p[12], 3);
        assert_eq!(p[14], 0x23);
        assert_eq!(p[15], 0x01);
        assert_eq!(p[18], 7);
        assert_eq!(p.len(), 530);
    }
}
