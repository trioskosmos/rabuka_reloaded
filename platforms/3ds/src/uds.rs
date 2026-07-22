// UDS local wireless multiplayer for 3DS Rabuka Reloaded.
//
// Protocol:
//   Host creates a UDS network, client scans and connects.
//   Both run identical GameState copies; only actions are transmitted.
//
// Message types (u8 tag + payload):
//   0x01 = SyncSetup  — host sends deck order (seed + card IDs) to client
//   0x02 = SyncAction  — player sends their chosen action to opponent
//   0x03 = SyncPing    — keepalive / turn acknowledgment
//   0x04 = SyncQuit    — player is leaving

// C shim FFI — all UDS calls go through ctru_shim.c
extern "C" {
    fn _3ds_uds_init(is_host: bool) -> i32;
    fn _3ds_uds_exit();
    fn _3ds_uds_send(data: *const u8, len: u32) -> i32;
    fn _3ds_uds_recv(buf: *mut u8, buf_len: u32, out_len: *mut u32) -> i32;
    fn _3ds_uds_is_connected() -> bool;
}

pub const MSG_SYNC_SETUP: u8 = 0x01;
pub const MSG_SYNC_ACTION: u8 = 0x02;
pub const MSG_SYNC_PING: u8 = 0x03;
pub const MSG_SYNC_QUIT: u8 = 0x04;

/// Initialize UDS as host or client. Returns Ok(()) on success.
pub fn uds_init(is_host: bool) -> Result<(), i32> {
    let rc = unsafe { _3ds_uds_init(is_host) };
    if rc == 0 {
        Ok(())
    } else {
        Err(rc)
    }
}

/// Shut down UDS.
pub fn uds_exit() {
    unsafe { _3ds_uds_exit() };
}

/// Send raw bytes over UDS. Returns bytes sent or error.
pub fn uds_send(data: &[u8]) -> Result<usize, i32> {
    let rc = unsafe { _3ds_uds_send(data.as_ptr(), data.len() as u32) };
    if rc >= 0 {
        Ok(rc as usize)
    } else {
        Err(rc)
    }
}

/// Receive raw bytes from UDS (non-blocking). Returns bytes received or error.
pub fn uds_recv(buf: &mut [u8]) -> Result<usize, i32> {
    let mut out_len: u32 = 0;
    let rc = unsafe { _3ds_uds_recv(buf.as_mut_ptr(), buf.len() as u32, &mut out_len) };
    if rc >= 0 {
        Ok(out_len as usize)
    } else {
        Err(rc)
    }
}

/// Check if UDS is connected.
pub fn uds_is_connected() -> bool {
    unsafe { _3ds_uds_is_connected() }
}

// --- High-level message helpers ---

/// Deck sync payload: seed (u64) + number of cards (u16) + card IDs (i16 each).
/// This ensures both consoles shuffle identically.
pub struct DeckSync {
    pub seed: u64,
    pub p1_main: Vec<i16>,
    pub p1_energy: Vec<i16>,
    pub p2_main: Vec<i16>,
    pub p2_energy: Vec<i16>,
}

impl DeckSync {
    /// Serialize to bytes: tag(1) + seed(8) + lens(4×u16) + cards(×i16)
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(
            1 + 8
                + 8
                + (self.p1_main.len()
                    + self.p1_energy.len()
                    + self.p2_main.len()
                    + self.p2_energy.len())
                    * 2,
        );
        v.push(MSG_SYNC_SETUP);
        v.extend_from_slice(&self.seed.to_le_bytes());
        v.extend_from_slice(&(self.p1_main.len() as u16).to_le_bytes());
        v.extend_from_slice(&(self.p1_energy.len() as u16).to_le_bytes());
        v.extend_from_slice(&(self.p2_main.len() as u16).to_le_bytes());
        v.extend_from_slice(&(self.p2_energy.len() as u16).to_le_bytes());
        for id in &self.p1_main {
            v.extend_from_slice(&id.to_le_bytes());
        }
        for id in &self.p1_energy {
            v.extend_from_slice(&id.to_le_bytes());
        }
        for id in &self.p2_main {
            v.extend_from_slice(&id.to_le_bytes());
        }
        for id in &self.p2_energy {
            v.extend_from_slice(&id.to_le_bytes());
        }
        v
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 1 + 8 + 8 {
            return None;
        }
        let mut off = 1; // skip tag
        let seed = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
        off += 8;
        let len1m = u16::from_le_bytes(data[off..off + 2].try_into().ok()?) as usize;
        off += 2;
        let len1e = u16::from_le_bytes(data[off..off + 2].try_into().ok()?) as usize;
        off += 2;
        let len2m = u16::from_le_bytes(data[off..off + 2].try_into().ok()?) as usize;
        off += 2;
        let len2e = u16::from_le_bytes(data[off..off + 2].try_into().ok()?) as usize;
        off += 2;
        let total = len1m + len1e + len2m + len2e;
        if data.len() < off + total * 2 {
            return None;
        }
        let mut p1_main = Vec::with_capacity(len1m);
        for _ in 0..len1m {
            p1_main.push(i16::from_le_bytes(data[off..off + 2].try_into().ok()?));
            off += 2;
        }
        let mut p1_energy = Vec::with_capacity(len1e);
        for _ in 0..len1e {
            p1_energy.push(i16::from_le_bytes(data[off..off + 2].try_into().ok()?));
            off += 2;
        }
        let mut p2_main = Vec::with_capacity(len2m);
        for _ in 0..len2m {
            p2_main.push(i16::from_le_bytes(data[off..off + 2].try_into().ok()?));
            off += 2;
        }
        let mut p2_energy = Vec::with_capacity(len2e);
        for _ in 0..len2e {
            p2_energy.push(i16::from_le_bytes(data[off..off + 2].try_into().ok()?));
            off += 2;
        }
        Some(DeckSync {
            seed,
            p1_main,
            p1_energy,
            p2_main,
            p2_energy,
        })
    }
}

/// Action sync payload: action_type tag + parameters.
/// We serialize the action as a compact binary message.
pub struct ActionSync {
    pub action_tag: u16, // ActionType discriminant
    pub card_id: Option<i16>,
    pub card_indices: Vec<usize>,
    pub stage_area: u8, // 0=none, 1=Left, 2=Center, 3=Right
    pub use_baton_touch: bool,
    pub ability_index: Option<u16>,
}

impl ActionSync {
    pub fn to_bytes(&self) -> Vec<u8> {
        let idx_len = self.card_indices.len();
        let mut v = Vec::with_capacity(1 + 2 + 2 + 1 + idx_len * 2 + 1 + 1 + 2);
        v.push(MSG_SYNC_ACTION);
        v.extend_from_slice(&self.action_tag.to_le_bytes());
        match self.card_id {
            Some(id) => {
                v.push(1);
                v.extend_from_slice(&id.to_le_bytes());
            }
            None => v.push(0),
        }
        v.push(idx_len as u8);
        for idx in &self.card_indices {
            v.extend_from_slice(&(*idx as u16).to_le_bytes());
        }
        v.push(self.stage_area);
        v.push(if self.use_baton_touch { 1 } else { 0 });
        match self.ability_index {
            Some(ai) => {
                v.push(1);
                v.extend_from_slice(&ai.to_le_bytes());
            }
            None => v.push(0),
        }
        v
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.is_empty() || data[0] != MSG_SYNC_ACTION {
            return None;
        }
        let mut off = 1;
        let action_tag = u16::from_le_bytes(data[off..off + 2].try_into().ok()?);
        off += 2;
        let has_card_id = data[off];
        off += 1;
        let card_id = if has_card_id == 1 {
            let id = i16::from_le_bytes(data[off..off + 2].try_into().ok()?);
            off += 2;
            Some(id)
        } else {
            None
        };
        let idx_len = data[off] as usize;
        off += 1;
        let mut card_indices = Vec::with_capacity(idx_len);
        for _ in 0..idx_len {
            card_indices.push(u16::from_le_bytes(data[off..off + 2].try_into().ok()?) as usize);
            off += 2;
        }
        let stage_area = data[off];
        off += 1;
        let use_baton_touch = data[off] != 0;
        off += 1;
        let has_abi = data[off];
        off += 1;
        let ability_index = if has_abi == 1 {
            Some(u16::from_le_bytes(data[off..off + 2].try_into().ok()?))
        } else {
            None
        };
        Some(ActionSync {
            action_tag,
            card_id,
            card_indices,
            stage_area,
            use_baton_touch,
            ability_index,
        })
    }
}
