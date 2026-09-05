//! Deterministic-lockstep link multiplayer (the 3DS design, ported down).
//!
//! Both consoles build IDENTICAL states (same seed + same decks through
//! [`crate::game::match_runner::build_match_state`]), then only chosen
//! actions cross the wire — like VsAi where the "AI" is a human on the
//! second console. Message tags and the action layout mirror
//! `platforms/3ds/src/uds.rs` so a future GBA<->3DS game stays
//! wire-compatible.
//!
//! Setup exchange reuses the SRAM deck image ([`crate::game::sav`]): each
//! side sends its own deck entry, both sides build P1+P2 from the pair,
//! and the host's seed wins. The entry doubles as the future link packet
//! exactly as documented there.
//!
//! Two simplifications, both documented:
//! - RPS picks cross the wire sequentially (P1 picks, P2 picks knowing
//!   it). Same information flow as local 2-player on one screen; a
//!   simultaneous blind exchange can replace it later without touching
//!   the loop.
//! - Header-only rows (mulligan/live-card headers) are not pickable over
//!   the link: their wire tag collides with `RockChoice` (both 0), so
//!   both sides filter them from the pick list AND the match lookup.
//!
//! `no_std`-safe (`alloc` only).

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::string::String;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

use crate::card::Card;
use crate::game::game_setup::{self, Action, ActionType};
use crate::game::match_runner::build_match_state;
use crate::game::menu::{select_action, show_result};
use crate::game::platform_ui::PlatformUi;
use crate::game::sav::SavDeck;
use crate::game_state::{GameResult, GameState, Phase};

/// Setup exchange: seed + one deck entry.
pub const MSG_SETUP: u8 = 0x01;
/// One picked action ([`LinkAction`] payload).
pub const MSG_ACTION: u8 = 0x02;
/// Keepalive (accepted, never required on a wired link).
pub const MSG_PING: u8 = 0x03;
/// Opponent is leaving; abort the match.
pub const MSG_QUIT: u8 = 0x04;
/// Receiver got the action; sender stops waiting. Mirrors the 3DS ack that
/// exists because UDS is unreliable — cheap insurance on a cable too.
pub const MSG_ACK: u8 = 0x07;

/// Byte transport. The GBA implements this over SIO multiplayer mode;
/// tests use a loopback pair.
pub trait LinkTransport {
    /// Link still alive?
    fn is_up(&mut self) -> bool;
    /// Queue one packet for the peer.
    fn send_packet(&mut self, packet: &[u8]) -> bool;
    /// One complete inbound packet, if any.
    fn recv_packet(&mut self) -> Option<Vec<u8>>;
    /// Local abort request (B button on GBA). Checked every wait-spin so
    /// a dead peer returns to the menu promptly instead of only tripping
    /// the spin budget. Default off (loopback tests never abort).
    fn poll_cancel(&mut self) -> bool {
        false
    }
}

/// A picked action on the wire. Layout is the 3DS `ActionSync` format
/// verbatim: tag(u16 LE) + card_id flag + card_id(i16) + index count +
/// indices(u16) + area(u8) + baton(u8) + ability flag + ability(u16) +
/// seq(u32). `card_index` is deliberately absent (write-only compat field
/// the executor never reads); the receiver recovers the executable row
/// with [`find_local_action`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAction {
    pub tag: u16,
    pub card_id: Option<i16>,
    pub card_indices: Vec<usize>,
    /// 0 = none, 1 = left, 2 = center, 3 = right.
    pub stage_area: u8,
    pub use_baton_touch: bool,
    pub ability_index: Option<u16>,
    pub seq: u32,
}

impl LinkAction {
    /// Encode the sender's pick. The leading byte is [`MSG_ACTION`].
    pub fn from_action(a: &Action, seq: u32) -> Self {
        let params = a.parameters.as_ref();
        LinkAction {
            tag: a.action_type.to_tag(),
            card_id: params.and_then(|p| p.card_id),
            card_indices: params
                .and_then(|p| p.card_indices.clone())
                .unwrap_or_default(),
            stage_area: match params.and_then(|p| p.stage_area.as_deref()) {
                Some("left") => 1,
                Some("center") => 2,
                Some("right") => 3,
                _ => 0,
            },
            use_baton_touch: params.and_then(|p| p.use_baton_touch).unwrap_or(false),
            ability_index: params.and_then(|p| p.ability_index).map(|i| i as u16),
            seq,
        }
    }

    /// Serialize with the [`MSG_ACTION`] tag byte first.
    pub fn encode(&self) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::with_capacity(32 + self.card_indices.len() * 2);
        v.push(MSG_ACTION);
        v.extend_from_slice(&self.tag.to_le_bytes());
        match self.card_id {
            Some(id) => {
                v.push(1);
                v.extend_from_slice(&id.to_le_bytes());
            }
            None => v.push(0),
        }
        v.push(self.card_indices.len().min(255) as u8);
        for idx in &self.card_indices {
            v.extend_from_slice(&(*idx as u16).to_le_bytes());
        }
        v.push(self.stage_area);
        v.push(u8::from(self.use_baton_touch));
        match self.ability_index {
            Some(ai) => {
                v.push(1);
                v.extend_from_slice(&ai.to_le_bytes());
            }
            None => v.push(0),
        }
        v.extend_from_slice(&self.seq.to_le_bytes());
        v
    }

    /// Parse one [`MSG_ACTION`] packet. `None` on tag/length mismatch.
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 2 || data[0] != MSG_ACTION {
            return None;
        }
        let mut off = 1;
        let tag = u16::from_le_bytes(data.get(off..off + 2)?.try_into().ok()?);
        off += 2;
        let has_card_id = *data.get(off)? != 0;
        off += 1;
        let card_id = if has_card_id {
            let id = i16::from_le_bytes(data.get(off..off + 2)?.try_into().ok()?);
            off += 2;
            Some(id)
        } else {
            None
        };
        let idx_len = *data.get(off)? as usize;
        off += 1;
        let mut card_indices: Vec<usize> = Vec::with_capacity(idx_len);
        for _ in 0..idx_len {
            card_indices.push(u16::from_le_bytes(data.get(off..off + 2)?.try_into().ok()?) as usize);
            off += 2;
        }
        let stage_area = *data.get(off)?;
        off += 1;
        let use_baton_touch = *data.get(off)? != 0;
        off += 1;
        let has_abi = *data.get(off)? != 0;
        off += 1;
        let ability_index = if has_abi {
            Some(u16::from_le_bytes(data.get(off..off + 2)?.try_into().ok()?))
        } else {
            None
        };
        if has_abi {
            off += 2;
        }
        // `seq` is optional on the wire (older/partial packets read as 0).
        let seq = data
            .get(off..off + 4)
            .and_then(|s| s.try_into().ok())
            .map(u32::from_le_bytes)
            .unwrap_or(0);
        Some(LinkAction {
            tag,
            card_id,
            card_indices,
            stage_area,
            use_baton_touch,
            ability_index,
            seq,
        })
    }
}

/// Rows that may cross the link. Header-only rows share wire tag 0 with
/// `RockChoice`, so both sides filter them from the pick list and the
/// match lookup below.
fn link_pickable(a: &Action) -> bool {
    !matches!(
        a.action_type,
        ActionType::MulliganHeader | ActionType::LiveCardHeader
    )
}

/// Find the locally-generated action matching a received link action.
/// Both sides generate identical lists (the lockstep invariant), so the
/// (tag, card_id, indices, area, baton, ability) key recovers the row.
pub fn find_local_action(acts: &[Action], link: &LinkAction) -> Option<usize> {
    let want_area = |a: &Action| -> u8 {
        match a.parameters.as_ref().and_then(|p| p.stage_area.as_deref()) {
            Some("left") => 1,
            Some("center") => 2,
            Some("right") => 3,
            _ => 0,
        }
    };
    acts.iter().position(|a| {
        link_pickable(a)
            && a.action_type.to_tag() == link.tag
            && a.action_type == ActionType::from_tag(link.tag)
            && a.parameters.as_ref().and_then(|p| p.card_id) == link.card_id
            && a.parameters
                .as_ref()
                .and_then(|p| p.card_indices.clone())
                .unwrap_or_default()
                == link.card_indices
            && want_area(a) == link.stage_area
            && a.parameters.as_ref().and_then(|p| p.use_baton_touch).unwrap_or(false)
                == link.use_baton_touch
            && a.parameters.as_ref().and_then(|p| p.ability_index).map(|i| i as u16)
                == link.ability_index
    })
}

/// Setup packet: [`MSG_SETUP`] + seed(u64 LE) + one [`SavDeck`] image.
/// The image is self-delimiting and self-validating through the sav codec.
pub fn encode_setup(seed: u64, deck: &SavDeck) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::new();
    v.push(MSG_SETUP);
    v.extend_from_slice(&seed.to_le_bytes());
    if let Ok(mut image) = crate::game::sav::encode_sav(core::slice::from_ref(deck)) {
        v.append(&mut image);
    }
    v
}

/// Parse a setup packet into `(seed, deck)`. Rejects anything that is not
/// exactly one valid deck.
pub fn decode_setup(data: &[u8]) -> Option<(u64, SavDeck)> {
    if data.len() < 1 + 8 || data[0] != MSG_SETUP {
        return None;
    }
    let seed = u64::from_le_bytes(data[1..9].try_into().ok()?);
    let mut decks = crate::game::sav::decode_sav(&data[9..]).ok()?;
    if decks.len() != 1 {
        return None;
    }
    Some((seed, decks.pop().unwrap_or(SavDeck {
        name: String::new(),
        cards: Vec::new(),
    })))
}

/// Acknowledgement for `seq`.
fn ack_packet(seq: u32) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(5);
    v.push(MSG_ACK);
    v.extend_from_slice(&seq.to_le_bytes());
    v
}

enum Inbound {
    None,
    Action(LinkAction),
    Ack(u32),
    Quit,
}

/// Drain one inbound packet: ack every valid action (so a lost ack never
/// stalls the sender), accept it only when its seq is new (retransmits
/// are re-acked and ignored). Acks surface to the caller for matching.
fn poll_inbound<T: LinkTransport>(link: &mut T, peer_seq: &mut u32) -> Inbound {
    let pkt = match link.recv_packet() {
        Some(p) => p,
        None => return Inbound::None,
    };
    match pkt.first() {
        Some(&MSG_ACTION) => match LinkAction::decode(&pkt) {
            Some(la) => {
                link.send_packet(&ack_packet(la.seq));
                if la.seq != *peer_seq {
                    *peer_seq = la.seq;
                    Inbound::Action(la)
                } else {
                    log::debug!("[LINK] dup action seq={} ignored", la.seq);
                    Inbound::None
                }
            }
            None => Inbound::None,
        },
        Some(&MSG_ACK) if pkt.len() >= 5 => {
            Inbound::Ack(u32::from_le_bytes(pkt[1..5].try_into().unwrap_or([0; 4])))
        }
        Some(&MSG_QUIT) => Inbound::Quit,
        _ => Inbound::None,
    }
}

/// Spin budget for one reliable exchange (~30 s at 60 fps). Only trips on
/// a dead link — normal packets cross in frames.
const SPIN_BUDGET: usize = 60 * 30;

/// Send one action packet; spin until its ack (resending each second).
/// Inbound actions met along the way are acked and stashed for the next
/// receive — without this, simultaneous picks (RPS opener) deadlock, with
/// both sides waiting for an ack neither will send. Returns false when
/// the peer quits, the link dies, or budget exhausts.
fn send_reliable<U: PlatformUi, T: LinkTransport>(
    ui: &mut U,
    link: &mut T,
    payload: &[u8],
    seq: u32,
    peer_seq: &mut u32,
    stashed: &mut Option<LinkAction>,
) -> bool {
    if !link.send_packet(payload) {
        return false;
    }
    let mut spins = 0usize;
    loop {
        if !link.is_up() {
            return false;
        }
        if link.poll_cancel() {
            // Best-effort quit so the peer aborts promptly too.
            link.send_packet(&[MSG_QUIT]);
            return false;
        }
        // Drain everything waiting: acks complete this send, actions get
        // stashed, quits abort.
        loop {
            match poll_inbound(link, peer_seq) {
                Inbound::Action(la) => {
                    // Only one stash slot is needed: strictly after the
                    // RPS opener, sends and receives alternate, so at most
                    // one concurrent pick can be in flight.
                    if stashed.is_none() {
                        *stashed = Some(la);
                    }
                }
                Inbound::Ack(s) if s == seq => return true,
                Inbound::Quit => return false,
                _ => break,
            }
        }
        spins += 1;
        if spins % 60 == 0 && !link.send_packet(payload) {
            return false;
        }
        if spins >= SPIN_BUDGET {
            log::debug!("[LINK] ack timeout seq={}", seq);
            return false;
        }
        ui.wait_vblank();
    }
}

enum RecvOutcome {
    Action(LinkAction),
    Quit,
    Down,
}

/// Spin until the peer's next action arrives, consuming the stash first
/// (a pick that arrived while we were sending). Every valid receipt is
/// acked on the way so the sender never stalls on a lost ack.
fn recv_action<U: PlatformUi, T: LinkTransport>(
    ui: &mut U,
    link: &mut T,
    peer_seq: &mut u32,
    stashed: &mut Option<LinkAction>,
) -> RecvOutcome {
    if let Some(la) = stashed.take() {
        return RecvOutcome::Action(la);
    }
    let mut spins = 0usize;
    loop {
        if !link.is_up() {
            return RecvOutcome::Down;
        }
        if link.poll_cancel() {
            link.send_packet(&[MSG_QUIT]);
            return RecvOutcome::Quit;
        }
        loop {
            match poll_inbound(link, peer_seq) {
                Inbound::Action(la) => return RecvOutcome::Action(la),
                Inbound::Quit => return RecvOutcome::Quit,
                _ => break,
            }
        }
        spins += 1;
        if spins >= SPIN_BUDGET {
            log::debug!("[LINK] action timeout");
            return RecvOutcome::Down;
        }
        ui.wait_vblank();
    }
}

/// Run a full link match. `local` is 0 on the P1 console, 1 on P2.
/// Routing reuses the engine contract: [`GameState::can_player_act`] picks
/// whose decision each step is, so turns and pending choices take the same
/// branch — local decisions are picked on the shared menu, sent, and
/// executed; remote ones arrive, match the local list, and execute.
///
/// Returns once a terminal `GameResult` is reached (or the link drops,
/// mirroring `run_match`'s break-on-abort contract).
pub struct LinkMatchReport {
    pub result: GameResult,
    pub final_state: GameState,
}

/// Run a full link match. `local` is 0 on the P1 console, 1 on P2.
/// Routing reuses the engine contract: [`GameState::can_player_act`] picks
/// whose decision each step is, so turns and pending choices take the same
/// branch — local decisions are picked on the shared menu, sent, and
/// executed; remote ones arrive, match the local list, and execute.
///
/// The final state travels with the report so tests can prove lockstep
/// (identical digests on both sides) without playing a full match.
pub fn run_link_match<U: PlatformUi, T: LinkTransport>(
    ui: &mut U,
    link: &mut T,
    local: u8,
    p1_cards: &[&str],
    p2_cards: &[&str],
    all_cards: Vec<Card>,
) -> LinkMatchReport {
    let mut gs: GameState = build_match_state(p1_cards, p2_cards, all_cards);
    let mut my_seq: u32 = 0;
    let mut peer_seq: u32 = u32::MAX;
    // A peer pick that arrives while we are sending (only the RPS opener
    // sends both ways at once). Consumed by the next receive.
    let mut stashed: Option<LinkAction> = None;

    loop {
        crate::turn::TurnEngine::check_victory_condition(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(ui, &gs);
            break;
        }
        game_setup::settle_auto(&mut gs);
        if gs.game_result != GameResult::Ongoing {
            show_result(ui, &gs);
            break;
        }
        if gs.is_loop_detected() {
            show_result(ui, &gs);
            break;
        }

        let acts = game_setup::generate_possible_actions(&gs);
        if acts.is_empty() {
            crate::turn::TurnEngine::advance_phase(&mut gs);
            gs.reset_loop_detection();
            continue;
        }
        // Header-only rows never cross the wire (tag collision, see above);
        // both sides filter identically, so sub-indices stay aligned.
        let pickable: Vec<usize> = acts
            .iter()
            .enumerate()
            .filter(|(_, a)| link_pickable(a))
            .map(|(i, _)| i)
            .collect();
        if pickable.is_empty() {
            crate::turn::TurnEngine::advance_phase(&mut gs);
            gs.reset_loop_detection();
            continue;
        }

        if gs.can_player_act(local as i32) {
            let sub_acts: Vec<Action> =
                pickable.iter().map(|&i| acts[i].clone()).collect();
            let idx = pickable[select_action(ui, &gs, &sub_acts)];
            let la = LinkAction::from_action(&acts[idx], my_seq);
            my_seq = my_seq.wrapping_add(1);
            if !send_reliable(ui, link, &la.encode(), la.seq, &mut peer_seq, &mut stashed) {
                break;
            }
            // RPS picks execute positionally (1st -> P1) unless the PVP
            // session stamps the picker: a P2 console executing its own
            // pick first would otherwise record it as P1's and re-pick
            // forever. Other phases route absolutely and ignore the stamp.
            if gs.current_phase == Phase::RockPaperScissors {
                gs.pending_rps_player_id = Some(local);
            }
            let _ = game_setup::execute_action(&mut gs, &acts[idx]);
        } else {
            match recv_action(ui, link, &mut peer_seq, &mut stashed) {
                RecvOutcome::Action(la) => match find_local_action(&acts, &la) {
                    Some(i) => {
                        if gs.current_phase == Phase::RockPaperScissors {
                            gs.pending_rps_player_id = Some(1 - local);
                        }
                        let _ = game_setup::execute_action(&mut gs, &acts[i]);
                    }
                    None => {
                        log::debug!("[LINK] no local match for tag={}", la.tag);
                        break;
                    }
                },
                RecvOutcome::Quit | RecvOutcome::Down => break,
            }
        }
        gs.reset_loop_detection();
        game_setup::settle_auto(&mut gs);
    }

    LinkMatchReport {
        result: gs.game_result.clone(),
        final_state: gs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::platform_ui::PlatformUi;
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    struct Hub {
        to_a: VecDeque<Vec<u8>>,
        to_b: VecDeque<Vec<u8>>,
        budget: usize,
        alive: bool,
        sent_a: usize,
        sent_b: usize,
    }

    /// Cross-connected loopback pair: A's sends land in B's inbox and back.
    /// `budget` bounds total packets so a desync bug aborts instead of
    /// hanging the suite (libtest has no per-test timeout).
    #[derive(Clone)]
    struct TestTransport {
        hub: Arc<Mutex<Hub>>,
        is_a: bool,
    }

    impl TestTransport {
        fn pair(budget: usize) -> (Self, Self) {
            let hub = Arc::new(Mutex::new(Hub {
                to_a: VecDeque::new(),
                to_b: VecDeque::new(),
                budget,
                alive: true,
                sent_a: 0,
                sent_b: 0,
            }));
            (
                TestTransport { hub: hub.clone(), is_a: true },
                TestTransport { hub, is_a: false },
            )
        }
    }

    impl LinkTransport for TestTransport {
        fn is_up(&mut self) -> bool {
            let hub = self.hub.lock().unwrap();
            hub.alive && hub.budget > 0
        }
        fn send_packet(&mut self, packet: &[u8]) -> bool {
            let mut hub = self.hub.lock().unwrap();
            if !hub.alive || hub.budget == 0 {
                hub.alive = false;
                return false;
            }
            hub.budget -= 1;
            if self.is_a {
                hub.to_b.push_back(packet.to_vec());
                hub.sent_a += 1;
            } else {
                hub.to_a.push_back(packet.to_vec());
                hub.sent_b += 1;
            }
            true
        }
        fn recv_packet(&mut self) -> Option<Vec<u8>> {
            let mut hub = self.hub.lock().unwrap();
            if self.is_a {
                hub.to_a.pop_front()
            } else {
                hub.to_b.pop_front()
            }
        }
    }

    /// Auto-picking console: first row always, except RPS where side 1
    /// takes the second row (Paper beats Rock: no draw loop). First-row
    /// picks are the progress options everywhere (Confirm / Pass), so a
    /// tiny-deck game mills to terminal in dozens of decisions, not
    /// thousands. The decision count (not frames or headers) would drive
    /// fancier policies; first-row needs no counting at all.
    struct TestUi {
        local: u8,
        frame: Cell<usize>,
        rows: Vec<String>,
    }

    impl TestUi {
        fn new(local: u8) -> Self {
            TestUi { local, frame: Cell::new(0), rows: Vec::new() }
        }
        fn downs(&self) -> usize {
            if self.rows.iter().any(|r| r.contains("Rock")) {
                return self.local as usize;
            }
            0
        }
    }

    impl PlatformUi for TestUi {
        fn clear_screen(&mut self) {
            // NOTE: no frame reset here — select_action clears every
            // frame, so the frame counts polls since the last A press.
            self.rows.clear();
        }
        fn println(&mut self, text: &str) {
            if self.rows.len() < 12 {
                self.rows.push(text.to_string());
            }
        }
        fn swap_buffers(&mut self) {}
        fn poll_input(&mut self) {
            self.frame.set(self.frame.get() + 1);
        }
        fn just_pressed_a(&self) -> bool {
            // One A press consumes one menu; the frame restarts so the
            // next menu's policy counts from zero.
            if self.frame.get() == self.downs() + 1 {
                self.frame.set(0);
                true
            } else {
                false
            }
        }
        fn just_pressed_b(&self) -> bool {
            false
        }
        fn just_pressed_up(&self) -> bool {
            false
        }
        fn just_pressed_down(&self) -> bool {
            let f = self.frame.get();
            f >= 1 && f <= self.downs()
        }
        fn just_pressed_start(&self) -> bool {
            false
        }
        fn wait_vblank(&mut self) {}
    }

    #[test]
    fn link_action_codec_roundtrip() {
        let la = LinkAction {
            tag: 6,
            card_id: Some(42),
            card_indices: vec![1, 2, 3],
            stage_area: 2,
            use_baton_touch: true,
            ability_index: Some(7),
            seq: 0xDEAD_BEEF,
        };
        let bytes = la.encode();
        assert_eq!(bytes[0], MSG_ACTION);
        assert_eq!(LinkAction::decode(&bytes), Some(la));
        let bare = LinkAction {
            tag: 22,
            card_id: None,
            card_indices: vec![],
            stage_area: 0,
            use_baton_touch: false,
            ability_index: None,
            seq: 0,
        };
        assert_eq!(LinkAction::decode(&bare.encode()), Some(bare));
        assert_eq!(LinkAction::decode(&[]), None);
        assert_eq!(LinkAction::decode(&[0x09]), None);
    }

    #[test]
    fn setup_codec_roundtrip() {
        let deck = SavDeck {
            name: String::from(" 体力測定* "),
            cards: vec![String::from("PL!S-bp2-022-L"), String::from("LL-bp2-001-R＋")],
        };
        let bytes = encode_setup(0x1234_5678_9ABC_DEF0, &deck);
        assert_eq!(bytes[0], MSG_SETUP);
        let (seed, back) = decode_setup(&bytes).expect("setup decodes");
        assert_eq!(seed, 0x1234_5678_9ABC_DEF0);
        assert_eq!(back, deck);
        assert_eq!(decode_setup(&bytes[..4]), None);
    }

    #[test]
    fn find_local_action_matches_and_rejects() {
        use crate::game::game_setup::{ActionParameters, ActionType};
        let mk = |t: ActionType, cid: Option<i16>| Action {
            description: String::from("row"),
            description_ja: None,
            action_type: t,
            parameters: Some(ActionParameters {
                card_id: cid,
                card_index: None,
                card_indices: None,
                stage_area: None,
                use_baton_touch: None,
                card_name: None,
                card_no: None,
                ability_index: None,
                source_ability: None,
                base_cost: None,
                final_cost: None,
                available_areas: None,
                double_baton_pairs: None,
                disabled: None,
            }),
            selected: None,
        };
        let acts = vec![
            mk(ActionType::RockChoice, None),
            mk(ActionType::PaperChoice, None),
            mk(ActionType::ChoiceDecision, Some(1)),
        ];
        let la = LinkAction::from_action(&acts[2], 9);
        assert_eq!(find_local_action(&acts, &la), Some(2));
        let mut wrong = la.clone();
        wrong.card_id = Some(0);
        assert_eq!(find_local_action(&acts, &wrong), None);
    }

    #[test]
    fn loopback_transport_moves_packets_both_ways() {
        let (mut ta, mut tb) = TestTransport::pair(10);
        assert!(ta.send_packet(&[1, 2, 3]));
        assert!(tb.send_packet(&[4, 5]));
        assert_eq!(tb.recv_packet(), Some(vec![1, 2, 3]));
        assert_eq!(ta.recv_packet(), Some(vec![4, 5]));
        assert_eq!(ta.recv_packet(), None);
        assert!(ta.is_up() && tb.is_up());
    }

    /// Full lockstep spot-check over loopback: both sides run concurrently
    /// (threads — sequential runs could never exchange a packet) through a
    /// small packet budget (~25 decisions: RPS, first-attacker pick,
    /// mulligans, live sets, passes), then their full state digests must
    /// match exactly. Any lockstep divergence (routing, codec, ordering)
    /// fails this test. Tiny decks (6 members + 3 lives a side;
    /// composition warnings are warnings only) keep every decision cheap,
    /// so this runs in seconds even in debug profile.
    #[test]
    fn loopback_full_match_stays_in_lockstep() {
        let json = include_str!("../../../cards/cards.json");
        let all_json =
            crate::card_loader::CardLoader::load_cards_from_strs(json).expect("cards load");
        let mut members: Vec<&crate::card::Card> =
            all_json.iter().filter(|c| c.is_member()).collect();
        members.sort_by(|a, b| a.card_no.as_ref().cmp(b.card_no.as_ref()));
        let mut lives: Vec<&crate::card::Card> =
            all_json.iter().filter(|c| c.is_live()).collect();
        lives.sort_by(|a, b| a.card_no.as_ref().cmp(b.card_no.as_ref()));
        assert!(members.len() >= 12 && lives.len() >= 6);
        let mut all_cards: Vec<crate::card::Card> = members[..12]
            .iter()
            .chain(lives[..6].iter())
            .map(|c| (*c).clone())
            .collect();
        crate::card_loader::CardLoader::attach_abilities(&mut all_cards);
        let p1deck: Vec<String> = members[..6]
            .iter()
            .chain(lives[..3].iter())
            .map(|c| c.card_no.as_ref().to_string())
            .collect();
        let p2deck: Vec<String> = members[6..12]
            .iter()
            .chain(lives[3..6].iter())
            .map(|c| c.card_no.as_ref().to_string())
            .collect();
        let p1: Vec<&str> = p1deck.iter().map(|s| s.as_str()).collect();
        let p2: Vec<&str> = p2deck.iter().map(|s| s.as_str()).collect();

        let (mut ta, mut tb) = TestTransport::pair(20_000);
        let hub = ta.hub.clone();
        let all_cards_b = all_cards.clone();

        // Both sides run concurrently: sequential runs could never
        // exchange a packet (side A would time out waiting for an ack
        // side B hasn't started sending yet).
        let (ra, rb) = std::thread::scope(|s| {
            let hb = s.spawn(|| {
                let mut ub = TestUi::new(1);
                crate::rng::seed(0x5EED);
                run_link_match(&mut ub, &mut tb, 1, &p1, &p2, all_cards_b)
            });
            let mut ua = TestUi::new(0);
            crate::rng::seed(0x5EED);
            let ra = run_link_match(&mut ua, &mut ta, 0, &p1, &p2, all_cards);
            (ra, hb.join().expect("side B finishes"))
        });

        assert_eq!(ra.result, rb.result, "both sides agree on the result");

        fn digest(gs: &GameState) -> String {
            format!(
                "{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{:?}|{:?}|{:?}|{}",
                gs.current_phase,
                gs.turn_number,
                gs.player1.hand.cards.len(),
                gs.player1.main_deck.cards.len(),
                gs.player1.energy_zone.cards.len(),
                gs.player1.live_card_zone.cards.len(),
                gs.player1.success_live_card_zone.cards.len(),
                gs.player1.waitroom.cards.len(),
                gs.player2.hand.cards.len(),
                gs.player2.main_deck.cards.len(),
                gs.player2.energy_zone.cards.len(),
                gs.player1_rps_choice,
                gs.player2_rps_choice,
                gs.game_result,
                gs.rule_log.len()
            )
        }
        assert_eq!(
            digest(&ra.final_state),
            digest(&rb.final_state),
            "both sides hold identical states"
        );
        let hub = hub.lock().unwrap();
        assert!(hub.sent_a > 20, "a real exchange happened, not a stub");
    }
}
