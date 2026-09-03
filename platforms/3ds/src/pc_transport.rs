#![cfg(feature = "3ds")]

use crate::ffi::*;
use crate::transport::{PcTransport, Transport, TransportType};

pub struct PcMultiplayer {
    transport: PcTransport,
    is_host: bool,
    deck_sync_sent: bool,
    deck_sync_received: bool,
    pending_deck_sync: Option<Vec<u8>>,
    pub ip_buffer: [u8; 16],
    pub ip_cursor: usize,
}

impl PcMultiplayer {
    pub fn new() -> Self {
        Self {
            transport: PcTransport::new(),
            is_host: false,
            deck_sync_sent: false,
            deck_sync_received: false,
            pending_deck_sync: None,
            ip_buffer: *b"192.168.1.100\0\0\0",
            ip_cursor: 0,
        }
    }

    pub fn init_host(&mut self) -> Result<(), i32> {
        self.is_host = true;
        self.transport.init(true)
    }

    pub fn init_client(&mut self, ip: &str) -> Result<(), i32> {
        self.is_host = false;
        self.transport.init(false)?;
        self.transport.connect(ip, crate::transport::PC_TRANSPORT_PORT)
    }

    pub fn poll(&mut self) -> bool {
        self.transport.poll()
    }

    pub fn is_connected(&self) -> bool {
        self.transport.is_connected()
    }

    pub fn send_action(&mut self, action: &crate::uds::ActionSync) -> Result<(), i32> {
        let bytes = action.to_bytes();
        self.transport.send(&bytes).map(|_| ())
    }

    pub fn recv_action(&mut self) -> Option<crate::uds::ActionSync> {
        let mut buf = [0u8; 256];
        match self.transport.recv(&mut buf) {
            Ok(len) if len > 0 => crate::uds::ActionSync::from_bytes(&buf[..len]),
            _ => None,
        }
    }

    pub fn send_deck_sync(&mut self, deck_sync: &crate::uds::DeckSync) -> Result<(), i32> {
        let bytes = deck_sync.to_bytes();
        self.transport.send(&bytes).map(|_| ())
    }

    pub fn recv_deck_sync(&mut self) -> Option<crate::uds::DeckSync> {
        let mut buf = [0u8; 1024];
        match self.transport.recv(&mut buf) {
            Ok(len) if len > 0 => crate::uds::DeckSync::from_bytes(&buf[..len]),
            _ => None,
        }
    }

    pub fn shutdown(&mut self) {
        self.transport.shutdown();
    }

    pub fn ip_str(&self) -> String {
        let end = self.ip_buffer.iter().position(|&b| b == 0).unwrap_or(self.ip_buffer.len());
        String::from_utf8_lossy(&self.ip_buffer[..end]).to_string()
    }

    pub fn set_ip(&mut self, ip: &str) {
        let bytes = ip.as_bytes();
        let len = bytes.len().min(self.ip_buffer.len() - 1);
        self.ip_buffer[..len].copy_from_slice(&bytes[..len]);
        self.ip_buffer[len] = 0;
    }

    pub fn edit_ip(&mut self, keys: u32) -> bool {
        let mut changed = false;
        if keys & 0x00000200 != 0 {
            self.ip_cursor = (self.ip_cursor + 1).min(15);
            changed = true;
        }
        if keys & 0x00000100 != 0 && self.ip_cursor > 0 {
            self.ip_cursor -= 1;
            changed = true;
        }
        if keys & 0x00000080 != 0 {
            let mut octets: Vec<u8> = self.ip_str().split('.').filter_map(|s| s.parse().ok()).collect();
            if octets.len() == 4 {
                octets[self.ip_cursor.min(3)] = octets[self.ip_cursor.min(3)].saturating_add(1);
                self.set_ip(&octets.iter().map(|o| o.to_string()).collect::<Vec<_>>().join("."));
                changed = true;
            }
        }
        if keys & 0x00000040 != 0 {
            let mut octets: Vec<u8> = self.ip_str().split('.').filter_map(|s| s.parse().ok()).collect();
            if octets.len() == 4 {
                octets[self.ip_cursor.min(3)] = octets[self.ip_cursor.min(3)].saturating_sub(1);
                self.set_ip(&octets.iter().map(|o| o.to_string()).collect::<Vec<_>>().join("."));
                changed = true;
            }
        }
        changed
    }
}