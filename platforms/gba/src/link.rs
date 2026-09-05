//! Link-cable multiplayer transport (SIO multiplayer mode).
//!
//! Register map and transfer discipline stolen from two prior-art repos
//! (see `research/gba_homebrew`):
//! - `gba-link-connection` `LinkRawCable`: init assigns `SIOCNT = 1<<13`
//!   then ORs the baud, either side may set START, spin on the busy bit
//!   with a cancel hook, `0xFFFF` is reserved (slot empty) so idle is
//!   `0x0000`, default 115200 bps.
//! - `serial-experiments-gba`: parent is always slot 0, child IDs are valid
//!   only after the first transfer, ready/error bit positions.
//!
//! Polled only — a turn-based card game has no use for serial IRQs. Above
//! single-word exchange sits a tiny packet layer (`[len][bytes][checksum]`
//! in LE words) so the engine moves whole action/setup packets while both
//! sides pump exchange rounds continuously. Either side can send at any
//! time, which is what makes simultaneous picks (the RPS opener) just work.
//!
//! Disconnect policy: per-round timeouts never fail fast (the peer may
//! still be in its menu); `is_up` trips only after sustained consecutive
//! failures, and B aborts any wait instantly. The engine therefore returns
//! to the menu on a pulled cable instead of freezing.
//!
//! `no_std`, no agb dependency (raw MMIO only).

extern crate alloc;

use alloc::vec::Vec;

use rabuka_engine::game::platform_ui::LinkTransport;

const REG_SIOCNT: *mut u16 = 0x0400_0128 as *mut u16;
const REG_SEND: *mut u16 = 0x0400_012A as *mut u16;
const REG_MULTI_BASE: *const u16 = 0x0400_0120 as *const u16;
const REG_RCNT: *mut u16 = 0x0400_0134 as *mut u16;
const REG_KEYS: *const u16 = 0x0400_0130 as *const u16;

/// SIOCNT bit 2 (read): 0 = parent (slot 0, may start transfers).
const BIT_PARENT: u16 = 1 << 2;
/// SIOCNT bits 4-5 (read): multiplayer ID, valid after first transfer.
const ID_SHIFT: u16 = 4;
/// SIOCNT bit 6 (read): transfer error.
const BIT_ERROR: u16 = 1 << 6;
/// SIOCNT bit 7: start (parent) / busy.
const BIT_START: u16 = 1 << 7;
/// Multiplayer mode: SIOCNT bit12 = 0, bit13 = 1.
const MODE_MP: u16 = 1 << 13;
/// Baud 115200 bps.
const BAUD_115200: u16 = 3;
/// Empty slot / no peer (never sent as payload).
pub const NO_DATA: u16 = 0xFFFF;
/// Idle word: keeps rounds turning with nothing to say.
const IDLE_WORD: u16 = 0x0000;
/// KEYB_B bit in REG_KEYS (active low).
const KEY_B: u16 = 1 << 1;

/// Spin iterations per exchange phase before it counts as missed.
/// A live round completes in microseconds; this only trips with no peer.
const ROUND_SPIN_BUDGET: usize = 100_000;
/// Consecutive missed rounds before the link reads dead. Generous on
/// purpose: menus don't pump, so a peer choosing a deck looks exactly
/// like silence. The engine's own ack/action budgets abort first; this
/// is the backstop for a pulled cable.
const MISS_THRESHOLD: usize = 8192;
/// Exchange rounds pumped per transport call (bulk setup crosses in a
/// few calls; single actions flush immediately).
const PUMP_ROUNDS: usize = 32;

fn reg_read(addr: *const u16) -> u16 {
    unsafe { core::ptr::read_volatile(addr) }
}

fn reg_write(addr: *mut u16, v: u16) {
    unsafe { core::ptr::write_volatile(addr, v) }
}

/// Link-cable endpoint. Create one per match attempt; drop to forget.
pub struct LinkCable {
    parent: bool,
    my_slot: usize,
    peer_slot: Option<usize>,
    tx_words: Vec<u16>,
    rx_words: Vec<u16>,
    rx_need: Option<usize>,
    rx_ready: Vec<Vec<u8>>,
    misses: usize,
    cancelled: bool,
    connected: bool,
}

impl LinkCable {
    /// Enter multiplayer mode and take one probe round. Returns the cable
    /// whether or not a peer is present — presence is discovered by traffic,
    /// so bring-up never blocks.
    pub fn init() -> Self {
        // Serial mode on RCNT (bits 14-15 = 00), multiplayer + 115200 on
        // SIOCNT, send register parked at idle.
        let rcnt = reg_read(REG_RCNT as *const u16);
        reg_write(REG_RCNT, rcnt & !((1 << 14) | (1 << 15)));
        reg_write(REG_SIOCNT, MODE_MP | BAUD_115200);
        reg_write(REG_SEND, IDLE_WORD);
        let parent = reg_read(REG_SIOCNT as *const u16) & BIT_PARENT == 0;
        log::debug!("[LINK_HW] init parent={}", parent);
        LinkCable {
            parent,
            my_slot: if parent { 0 } else { usize::MAX },
            peer_slot: None,
            tx_words: Vec::new(),
            rx_words: Vec::new(),
            rx_need: None,
            rx_ready: Vec::new(),
            misses: 0,
            cancelled: false,
            connected: true,
        }
    }

    /// One exchange round carrying `send`. Returns all four slots'
    /// received words on success, `None` on timeout/error.
    ///
    /// Start discipline (stolen from LinkRawCable): either side may set
    /// START, but only the parent's write clocks a round — a slave's is
    /// ignored. So the parent sets START then spins for clear, while a
    /// child spins for set (parent started) then clear.
    fn exchange_round(&mut self, send: u16) -> Option<[u16; 4]> {
        reg_write(REG_SEND, send);
        if self.parent {
            reg_write(REG_SIOCNT, reg_read(REG_SIOCNT as *const u16) | BIT_START);
        } else {
            let mut spins = 0usize;
            while reg_read(REG_SIOCNT as *const u16) & BIT_START == 0 {
                spins += 1;
                if spins >= ROUND_SPIN_BUDGET {
                    return None;
                }
            }
        }
        let mut spins = 0usize;
        while reg_read(REG_SIOCNT as *const u16) & BIT_START != 0 {
            spins += 1;
            if spins >= ROUND_SPIN_BUDGET {
                return None;
            }
        }
        let cnt = reg_read(REG_SIOCNT as *const u16);
        if cnt & BIT_ERROR != 0 {
            log::debug!("[LINK_HW] error bit");
            return None;
        }
        let mut out = [NO_DATA; 4];
        for i in 0..4 {
            out[i] = reg_read(unsafe { REG_MULTI_BASE.add(i) });
        }
        // First successful round nails down our slot (parent is 0 by
        // hardware contract; children read their assigned ID).
        if self.my_slot == usize::MAX {
            let id = ((cnt >> ID_SHIFT) & 0b11) as usize;
            self.my_slot = id.min(3);
            log::debug!("[LINK_HW] my_slot={}", self.my_slot);
        }
        Some(out)
    }

    /// Peer word for this round. Presence comes from successful rounds,
    /// never from word values: a complete round REQUIRES the peer's
    /// participation, so every word in the latched peer slot is data —
    /// including 0xFFFF (ChoiceSkip's card_id -1 crosses as FFFF halves).
    /// The parent latches the first live child slot; a child always
    /// listens to the parent's slot 0.
    fn peer_word(&mut self, slots: &[u16; 4]) -> Option<u16> {
        if self.parent {
            if self.peer_slot.is_none() {
                for i in 1..4 {
                    if slots[i] != NO_DATA {
                        self.peer_slot = Some(i);
                        log::debug!("[LINK_HW] peer_slot={}", i);
                        break;
                    }
                }
            }
            self.peer_slot.map(|p| slots[p])
        } else {
            self.peer_slot = Some(0);
            Some(slots[0])
        }
    }

    /// Run up to `rounds` exchanges: flush queued transmit words (idling
    /// when dry) and harvest peer words into the reassembly buffer.
    /// Checks B for cancel; counts misses toward the dead-link trip.
    pub fn pump(&mut self, rounds: usize) {
        for _ in 0..rounds {
            if reg_read(REG_KEYS) & KEY_B == 0 {
                self.cancelled = true;
            }
            let send = if self.tx_words.is_empty() {
                IDLE_WORD
            } else {
                self.tx_words.remove(0)
            };
            match self.exchange_round(send) {
                Some(slots) => {
                    self.misses = 0;
                    if let Some(w) = self.peer_word(&slots) {
                        self.ingest(w);
                    }
                }
                None => {
                    // The word we tried to send was never clocked out;
                    // requeue it ahead of anything newer.
                    self.tx_words.insert(0, send);
                    self.misses += 1;
                    if self.misses >= MISS_THRESHOLD {
                        self.connected = false;
                        log::debug!("[LINK_HW] dead after {} misses", self.misses);
                        return;
                    }
                }
            }
            if self.cancelled {
                return;
            }
        }
    }

    /// Feed one peer word into the packet reassembler. Frame: [nwords:u16]
    /// [nbytes:u16] [payload words] [checksum:u16 = wrapping word sum].
    /// Idle words outside a frame are ignored; a length that never
    /// completes is dropped by the next length word (self-resyncing).
    fn ingest(&mut self, w: u16) {
        if self.rx_need.is_none() {
            if w == IDLE_WORD || w == NO_DATA {
                return;
            }
            // First word of a frame is the payload word count. Sanity-cap
            // it: a full deck setup is ~900 words; anything absurd resyncs.
            if w == 0 || w as usize > 2048 {
                log::debug!("[LINK_HW] bad len word {}", w);
                return;
            }
            self.rx_words.clear();
            self.rx_need = Some(w as usize);
            return;
        }
        self.rx_words.push(w);
        let need = self.rx_need.unwrap_or(0);
        // +1 for the byte-count word, +1 for the checksum word.
        if self.rx_words.len() >= need + 2 {
            let words = core::mem::take(&mut self.rx_words);
            self.rx_need = None;
            let nbytes = words[0] as usize;
            let sum: u16 = words[1..1 + need.min(words.len().saturating_sub(2))]
                .iter()
                .fold(0u16, |a, &x| a.wrapping_add(x));
            let check = words.get(1 + need).copied().unwrap_or(0xFFFF);
            if sum != check {
                log::debug!("[LINK_HW] checksum mismatch");
                return;
            }
            let mut bytes: Vec<u8> = Vec::with_capacity(nbytes);
            for (i, &word) in words[1..1 + need].iter().enumerate() {
                if bytes.len() < nbytes {
                    bytes.push((word & 0xFF) as u8);
                    let _ = i;
                }
                if bytes.len() < nbytes {
                    bytes.push((word >> 8) as u8);
                }
            }
            bytes.truncate(nbytes);
            self.rx_ready.push(bytes);
        }
    }

    /// Reset liveness after a proven-alive exchange (menu dwell looks
    /// exactly like silence, so setup waits must not trip the dead-link
    /// counter the match relies on).
    pub fn mark_alive(&mut self) {
        self.misses = 0;
        self.connected = true;
    }

    /// Queue one packet for the peer (framed). Always succeeds unless the
    /// link already read dead; flushing happens in [`LinkCable::pump`].
    pub fn send_packet(&mut self, payload: &[u8]) -> bool {
        if !self.connected {
            return false;
        }
        let mut words: Vec<u16> = Vec::with_capacity(payload.len() / 2 + 3);
        words.push(0); // patched with the count below
        words.push(payload.len() as u16);
        let mut i = 0;
        while i < payload.len() {
            let lo = payload[i] as u16;
            let hi = if i + 1 < payload.len() {
                payload[i + 1] as u16
            } else {
                0
            };
            words.push(lo | (hi << 8));
            i += 2;
        }
        let nwords = (words.len() - 2) as u16;
        words[0] = nwords;
        let sum: u16 = words[2..].iter().fold(0u16, |a, &x| a.wrapping_add(x));
        words.push(sum);
        self.tx_words.extend(words);
        true
    }
}

impl LinkTransport for LinkCable {
    fn is_up(&mut self) -> bool {
        self.pump(4);
        self.connected
    }

    fn send_packet(&mut self, packet: &[u8]) -> bool {
        if !LinkCable::send_packet(self, packet) {
            return false;
        }
        self.pump(PUMP_ROUNDS);
        self.connected
    }

    fn recv_packet(&mut self) -> Option<Vec<u8>> {
        self.pump(PUMP_ROUNDS);
        if self.rx_ready.is_empty() {
            None
        } else {
            Some(self.rx_ready.remove(0))
        }
    }

    fn poll_cancel(&mut self) -> bool {
        self.pump(1);
        self.cancelled
    }
}
