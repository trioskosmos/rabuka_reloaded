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
    fn _3ds_uds_scan_networks(out_ids: *mut u16, max_out: i32) -> i32;
    fn _3ds_uds_connect_network(node_id: u16) -> i32;
}

pub const MSG_SYNC_SETUP: u8 = 0x01;
pub const MSG_SYNC_ACTION: u8 = 0x02;
pub const MSG_SYNC_PING: u8 = 0x03;
pub const MSG_SYNC_QUIT: u8 = 0x04;
pub const MSG_SYNC_STATE: u8 = 0x05;
pub const MSG_SYNC_STATE_ACK: u8 = 0x06;
pub const MSG_SYNC_ACTION_ACK: u8 = 0x07;

/// Max payload per UDS packet (safe for a single data frame).
/// UDS data frames carry up to 0x3D4 (980) bytes; 900 leaves margin for
/// frame overhead while nearly doubling the old 480-byte payload, halving
/// the number of packets (and receive frames) per state transfer.
pub const UDS_CHUNK_SIZE: usize = 900;

/// Split a large byte payload into MSG_SYNC_STATE chunks for one transmission.
/// Header: [0x05, seq_lo, seq_hi, total:u8, idx:u8] + payload.
/// First chunk (idx==0) is prefixed with a 4-byte little-endian total length.
/// `seq` distinguishes consecutive transmissions so the receiver can ignore
/// chunks from an older transmission that arrive late (UDS is unreliable).
pub fn state_chunks(data: &[u8], seq: u16) -> Vec<Vec<u8>> {
    let total_chunks = (data.len() + UDS_CHUNK_SIZE - 1) / UDS_CHUNK_SIZE;
    let mut out = Vec::with_capacity(total_chunks);
    let mut offset = 0usize;
    for idx in 0..total_chunks {
        let end = (offset + UDS_CHUNK_SIZE).min(data.len());
        let mut chunk = Vec::with_capacity(end - offset + 6 + 4);
        chunk.push(MSG_SYNC_STATE);
        chunk.push((seq & 0xFF) as u8);
        chunk.push((seq >> 8) as u8);
        chunk.push(total_chunks as u8);
        chunk.push(idx as u8);
        if idx == 0 {
            chunk.extend_from_slice(&(data.len() as u32).to_le_bytes());
        }
        chunk.extend_from_slice(&data[offset..end]);
        out.push(chunk);
        offset = end;
    }
    out
}

/// Build an MSG_SYNC_STATE_ACK packet for a received state seq.
/// Format: [0x06, seq_lo, seq_hi, bitmap_len, bitmap...]
/// The bitmap is a per-chunk received-flag list (MSB-first within each byte,
/// chunk 0 = bit 0 of byte 0). This lets the host selectively retransmit only
/// the chunks that were dropped instead of the whole batch. `chunks_seen` is
/// the receiver's reassembly bitmap; it may be partial (sent each frame while
/// a transfer is in progress) or complete (sent on reassembly).
pub fn state_ack(seq: u16, chunks_seen: &[bool]) -> Vec<u8> {
    let bitmap_len = (chunks_seen.len().div_ceil(8)).min(255);
    let mut v = Vec::with_capacity(4 + bitmap_len);
    v.push(MSG_SYNC_STATE_ACK);
    v.push((seq & 0xFF) as u8);
    v.push((seq >> 8) as u8);
    v.push(bitmap_len as u8);
    for byte in 0..bitmap_len {
        let mut bits = 0u8;
        for bit in 0..8 {
            let idx = byte * 8 + bit;
            if idx < chunks_seen.len() && chunks_seen[idx] {
                bits |= 1 << bit;
            }
        }
        v.push(bits);
    }
    v
}

/// Build an MSG_SYNC_ACTION_ACK packet acknowledging a processed client action.
/// The client stops retransmitting its action once it receives this, instead of
/// waiting for a full state round-trip. This decoupling breaks the retransmit
/// storm where a duplicate action made the host re-stage a fresh state seq,
/// resetting the client's partial reassembly and looping forever.
pub fn action_ack(action_seq: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(5);
    v.push(MSG_SYNC_ACTION_ACK);
    v.extend_from_slice(&action_seq.to_le_bytes());
    v
}

/// Parse an MSG_SYNC_ACTION_ACK packet, returning the acknowledged action_seq.
pub fn parse_action_ack(data: &[u8]) -> Option<u32> {
    if data.len() < 5 || data[0] != MSG_SYNC_ACTION_ACK {
        return None;
    }
    Some(u32::from_le_bytes(data[1..5].try_into().ok()?))
}

/// A reassembler for an in-progress MSG_SYNC_STATE transfer.
#[derive(Clone)]
pub struct StateReceiver {
    expected_total: usize,
    received: Vec<u8>,
    chunks_seen: Vec<bool>,
    current_seq: Option<u16>,
    /// True once the current_seq transfer has been consumed by take().
    done: bool,
}

impl StateReceiver {
    pub fn new() -> Self {
        StateReceiver {
            expected_total: 0,
            received: Vec::new(),
            chunks_seen: Vec::new(),
            current_seq: None,
            done: false,
        }
    }

    /// Reset all reassembly state (e.g. when a transfer should start fresh).
    pub fn reset(&mut self) {
        self.expected_total = 0;
        self.received = Vec::new();
        self.chunks_seen = Vec::new();
        self.current_seq = None;
        self.done = false;
    }

    /// The seq of the transfer currently being reassembled (None when done).
    pub fn in_progress_seq(&self) -> Option<u16> {
        if self.done {
            None
        } else {
            self.current_seq
        }
    }

    /// Build an ACK for the in-progress transfer carrying the current bitmap.
    /// Sent periodically so the host prunes already-received chunks from its
    /// retransmit set instead of resending everything. Returns None once the
    /// transfer is consumed (done), so the client stops spamming ACKs.
    pub fn partial_ack(&self) -> Option<Vec<u8>> {
        let seq = self.current_seq?;
        if self.done || self.chunks_seen.is_empty() {
            return None;
        }
        Some(state_ack(seq, &self.chunks_seen))
    }

    /// True when the given seq is a *completed* transfer being retransmitted —
    /// the caller should re-send the completed ACK so the host stops. This
    /// heals a dropped final ACK without re-adopting the stale state.
    pub fn wants_reack(&self, seq: u16) -> bool {
        self.done && self.current_seq == Some(seq)
    }

    /// Full-bitmap ACK for the last completed transfer (for re-ACK retransmits).
    pub fn completed_ack(&self) -> Option<Vec<u8>> {
        if !self.done {
            return None;
        }
        let seq = self.current_seq?;
        if self.chunks_seen.is_empty() {
            return None;
        }
        Some(state_ack(seq, &self.chunks_seen))
    }

    /// Feed one received chunk. Returns true when the full state is ready to take().
    ///
    /// Two correctness rules prevent the desync + retransmit storm seen in play:
    ///  1. A repeated chunk 0 of an in-progress transfer is treated as a normal
    ///     chunk — it must NOT wipe `received`/`chunks_seen`. The host retransmits
    ///     the whole batch (including chunk 0) on every retry, so re-initializing
    ///     on each chunk 0 meant the client never finished reassembling, never
    ///     ACKed, and the host retransmitted forever.
    ///  2. A chunk from a *stale* (older) seq is ignored instead of aborting the
    ///     in-progress transfer. Previously any different seq aborted the current
    ///     transfer, so a late chunk from an older authoritative state could
    ///     preempt a newer one and revert the board on the client.
    /// Only a strictly-newer seq starts a fresh transfer.
    pub fn feed(&mut self, chunk: &[u8]) -> bool {
        if chunk.len() < 5 || chunk[0] != MSG_SYNC_STATE {
            return false;
        }
        let seq = (chunk[1] as u16) | ((chunk[2] as u16) << 8);
        let total = chunk[3] as usize;
        let idx = chunk[4] as usize;
        if total == 0 || idx >= total {
            return false;
        }
        match self.current_seq {
            Some(cur) if cur == seq => {
                // Same transfer. If already consumed, let the caller re-ACK.
                if self.done {
                    return false;
                }
            }
            Some(cur) if (seq.wrapping_sub(cur) as i16) > 0 => {
                // Newer transmission: abandon the old one and start fresh.
                self.reset();
                self.current_seq = Some(seq);
            }
            Some(_) => {
                // Stale (older) transmission — ignore to avoid reverting the board.
                return false;
            }
            None => {
                self.current_seq = Some(seq);
            }
        }
        // Initialize the transfer from chunk 0 exactly once. Starting from a
        // non-zero chunk is impossible (we don't know the total length yet).
        if self.chunks_seen.is_empty() {
            if idx != 0 {
                self.current_seq = None;
                return false;
            }
            if chunk.len() < 9 {
                self.current_seq = None;
                return false;
            }
            self.expected_total = u32::from_le_bytes(chunk[5..9].try_into().unwrap()) as usize;
            self.received = Vec::with_capacity(self.expected_total);
            self.chunks_seen = vec![false; total];
        }
        let payload_start = if idx == 0 { 9 } else { 5 };
        if chunk.len() <= payload_start {
            return false;
        }
        if !self.chunks_seen[idx] {
            self.chunks_seen[idx] = true;
            self.received.extend_from_slice(&chunk[payload_start..]);
        }
        // Done when all chunks are seen
        self.chunks_seen.iter().all(|&b| b)
    }

    /// Take the reassembled state bytes. Caller must verify it matches the
    /// seq of the ACK it sends.
    pub fn take(&mut self) -> Option<Vec<u8>> {
        if self.expected_total == 0 || self.received.len() < self.expected_total {
            return None;
        }
        let out = self.received.split_off(0);
        self.expected_total = 0;
        // Keep chunks_seen + current_seq so a retransmitted completed transfer
        // can be re-ACKed (wants_reack) instead of re-adopted — re-adopting a
        // stale state is what caused the board to revert on one side.
        self.done = true;
        Some(out)
    }
}

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

/// Scan for available host networks. Returns list of node_ids.
pub fn uds_scan_networks() -> Vec<u16> {
    let mut ids = [0u16; 8];
    let count = unsafe { _3ds_uds_scan_networks(ids.as_mut_ptr(), 8) };
    ids[..count as usize].to_vec()
}

/// Connect to a specific host network by node_id.
pub fn uds_connect_network(node_id: u16) -> Result<(), i32> {
    let rc = unsafe { _3ds_uds_connect_network(node_id) };
    if rc == 0 {
        Ok(())
    } else {
        Err(rc)
    }
}

// --- High-level message helpers ---

/// Deck sync payload: seed (u64) + template IDs (u16 each) for both players' decks.
/// Template IDs are deterministic (same on both machines from load_or_create).
/// Client calls create_copy for each template_id to get matching instance IDs.
pub struct DeckSync {
    pub seed: u64,
    pub p1_main_templates: Vec<u16>,
    pub p1_energy_templates: Vec<u16>,
    pub p2_main_templates: Vec<u16>,
    pub p2_energy_templates: Vec<u16>,
}

impl DeckSync {
    /// Serialize to bytes: tag(1) + seed(8) + lens(4×u16) + template_ids(×u16)
    pub fn to_bytes(&self) -> Vec<u8> {
        let total = self.p1_main_templates.len()
            + self.p1_energy_templates.len()
            + self.p2_main_templates.len()
            + self.p2_energy_templates.len();
        let mut v = Vec::with_capacity(1 + 8 + 8 + total * 2);
        v.push(MSG_SYNC_SETUP);
        v.extend_from_slice(&self.seed.to_le_bytes());
        v.extend_from_slice(&(self.p1_main_templates.len() as u16).to_le_bytes());
        v.extend_from_slice(&(self.p1_energy_templates.len() as u16).to_le_bytes());
        v.extend_from_slice(&(self.p2_main_templates.len() as u16).to_le_bytes());
        v.extend_from_slice(&(self.p2_energy_templates.len() as u16).to_le_bytes());
        for id in &self.p1_main_templates {
            v.extend_from_slice(&id.to_le_bytes());
        }
        for id in &self.p1_energy_templates {
            v.extend_from_slice(&id.to_le_bytes());
        }
        for id in &self.p2_main_templates {
            v.extend_from_slice(&id.to_le_bytes());
        }
        for id in &self.p2_energy_templates {
            v.extend_from_slice(&id.to_le_bytes());
        }
        v
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 1 + 8 + 8 {
            return None;
        }
        let mut off = 1;
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
        let read_u16s = |data: &[u8], off: &mut usize, count: usize| -> Option<Vec<u16>> {
            let mut v = Vec::with_capacity(count);
            for _ in 0..count {
                v.push(u16::from_le_bytes(data[*off..*off + 2].try_into().ok()?));
                *off += 2;
            }
            Some(v)
        };
        let p1_main_templates = read_u16s(data, &mut off, len1m)?;
        let p1_energy_templates = read_u16s(data, &mut off, len1e)?;
        let p2_main_templates = read_u16s(data, &mut off, len2m)?;
        let p2_energy_templates = read_u16s(data, &mut off, len2e)?;
        Some(DeckSync {
            seed,
            p1_main_templates,
            p1_energy_templates,
            p2_main_templates,
            p2_energy_templates,
        })
    }
}

/// Action sync payload: action_type tag + parameters.
/// We serialize the action as a compact binary message.
/// `action_seq` is a monotonically increasing client-side counter. The host
/// uses it to dedup retransmitted packets (UDS is unreliable) so an action is
/// only ever executed once, while retransmits still get a state reply.
pub struct ActionSync {
    pub action_tag: u16, // ActionType discriminant
    pub card_id: Option<i16>,
    pub card_indices: Vec<usize>,
    pub stage_area: u8, // 0=none, 1=Left, 2=Center, 3=Right
    pub use_baton_touch: bool,
    pub ability_index: Option<u16>,
    pub action_seq: u32,
}

impl ActionSync {
    pub fn to_bytes(&self) -> Vec<u8> {
        let idx_len = self.card_indices.len();
        let mut v = Vec::with_capacity(1 + 2 + 2 + 1 + idx_len * 2 + 1 + 1 + 2 + 4);
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
        v.extend_from_slice(&self.action_seq.to_le_bytes());
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
        let action_seq = if data.len() >= off + 4 {
            u32::from_le_bytes(data[off..off + 4].try_into().ok()?)
        } else {
            0
        };
        Some(ActionSync {
            action_tag,
            card_id,
            card_indices,
            stage_area,
            use_baton_touch,
            ability_index,
            action_seq,
        })
    }
}
