#![cfg(feature = "3ds")]

use crate::uds::{ActionSync, DeckSync};
use crate::ffi::*;

pub const PC_TRANSPORT_PORT: u16 = 7341;
pub const PC_TRANSPORT_MAGIC: u32 = 0x52424B50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransportType {
    Uds,
    Pc,
}

pub trait Transport: Send {
    fn transport_type(&self) -> TransportType;
    fn init(&mut self, is_host: bool) -> Result<(), i32>;
    fn connect(&mut self, ip: &str, port: u16) -> Result<(), i32>;
    fn send(&mut self, data: &[u8]) -> Result<usize, i32>;
    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, i32>;
    fn poll(&mut self) -> bool;
    fn is_connected(&self) -> bool;
    fn shutdown(&mut self);
}

pub struct UdsTransport {
    initialized: bool,
}

impl UdsTransport {
    pub fn new() -> Self {
        Self { initialized: false }
    }
}

impl Transport for UdsTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Uds
    }

    fn init(&mut self, is_host: bool) -> Result<(), i32> {
        let rc = unsafe { _3ds_uds_init(is_host) };
        if rc == 0 {
            self.initialized = true;
            Ok(())
        } else {
            Err(rc)
        }
    }

    fn connect(&mut self, _ip: &str, _port: u16) -> Result<(), i32> {
        Err(-1)
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, i32> {
        if !self.initialized {
            return Err(-1);
        }
        crate::uds::uds_send(data)
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        if !self.initialized {
            return Err(-1);
        }
        crate::uds::uds_recv(buf)
    }

    fn poll(&mut self) -> bool {
        self.initialized && unsafe { _3ds_uds_is_connected() }
    }

    fn is_connected(&self) -> bool {
        self.initialized && unsafe { _3ds_uds_is_connected() }
    }

    fn shutdown(&mut self) {
        if self.initialized {
            crate::uds::uds_exit();
            self.initialized = false;
        }
    }
}

pub struct PcTransport {
    sock: i32,
    connected: bool,
}

impl PcTransport {
    pub fn new() -> Self {
        Self {
            sock: -1,
            connected: false,
        }
    }

    fn create_socket(&mut self) -> Result<(), i32> {
        let sock = unsafe { _3ds_pc_socket() };
        if sock < 0 {
            return Err(-1);
        }
        self.sock = sock;
        Ok(())
    }
}

impl Transport for PcTransport {
    fn transport_type(&self) -> TransportType {
        TransportType::Pc
    }

    fn init(&mut self, _is_host: bool) -> Result<(), i32> {
        self.create_socket()
    }

    fn connect(&mut self, ip: &str, port: u16) -> Result<(), i32> {
        if self.sock < 0 {
            self.create_socket()?;
        }

        let ip_c = format!("{}\0", ip);
        let rc = unsafe { _3ds_pc_connect(self.sock, ip_c.as_ptr() as *const u8, port) };
        if rc < 0 {
            return Err(rc);
        }
        self.connected = true;
        Ok(())
    }

    fn send(&mut self, data: &[u8]) -> Result<usize, i32> {
        if !self.connected || self.sock < 0 {
            return Err(-1);
        }

        let mut buf = Vec::with_capacity(4 + data.len());
        buf.extend_from_slice(&PC_TRANSPORT_MAGIC.to_be_bytes());
        buf.extend_from_slice(data);

        let rc = unsafe { _3ds_pc_send(self.sock, buf.as_ptr(), buf.len() as u32) };
        if rc < 0 {
            Err(rc)
        } else {
            Ok(rc as usize)
        }
    }

    fn recv(&mut self, buf: &mut [u8]) -> Result<usize, i32> {
        if !self.connected || self.sock < 0 {
            return Err(-1);
        }

        let mut tmp = vec![0u8; buf.len().max(1024)];
        let rc = unsafe { _3ds_pc_recv(self.sock, tmp.as_mut_ptr(), tmp.len() as u32) };

        if rc <= 0 {
            return Err(rc);
        }

        if rc < 4 {
            return Err(-5);
        }

        let magic = u32::from_be_bytes([tmp[0], tmp[1], tmp[2], tmp[3]]);
        if magic != PC_TRANSPORT_MAGIC {
            return Err(-6);
        }

        let payload_len = (rc - 4) as usize;
        let copy_len = payload_len.min(buf.len());
        buf[..copy_len].copy_from_slice(&tmp[4..4 + copy_len]);

        Ok(copy_len)
    }

    fn poll(&mut self) -> bool {
        self.connected && self.sock >= 0
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn shutdown(&mut self) {
        if self.sock >= 0 {
            unsafe { _3ds_pc_close(self.sock) };
            self.sock = -1;
        }
        self.connected = false;
    }
}