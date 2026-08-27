/// Tests for PL!-pb1-006-R 西木野真姫 (Nishikino Maki):
///
/// Ability #0 (登場):
///   自分の控え室から『μ's』のライブカードを1枚までデッキの一番上に置く。
///   その後、相手のステージにウェイト状態のメンバーがいる場合、カードを1枚引く。
///
/// Sequential with conditional second step. Move is unconditional (max 1),
/// draw is conditional on opponent Wait member (state_condition wait).
/// Parser fix: _try_state extracts target/location/card_type/count/operator;
/// parse_ability handles 「その後、…場合」 sequential condition correctly
/// (no bogus top-level condition, no group leak to draw).
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const MAKI_PB1: &str = "PL!-pb1-006-R";
const FILLER: &str = "PL!-sd1-010-SD";
const MUS_LIVE: &str = "PL!-bp3-019-L";

// ============================================================
// Sub-action 1: Place μ's live card from discard to deck top
// ============================================================

/// Debut: μ's live card in discard → placed on deck top
#[test]
fn maki_pb1_place_mus_live_on_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let mus_live = game.id(MUS_LIVE);
    let filler = game.id(FILLER);

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.give_energy(20);

    game.play_to_stage(maki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // The μ's live was moved waitroom→deck_top, then immediately drawn
    // if opponent Wait? No opponent Wait here, so it stays on deck top.
    let deck_top = game.state.player1.main_deck.cards.first().copied().unwrap_or(-1);
    assert_eq!(deck_top, mus_live, "μ's live card should be placed on deck top");
}

/// Debut: no μ's live card in discard → nothing placed on deck
#[test]
fn maki_pb1_no_mus_live_in_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(game.id(FILLER));

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.give_energy(20);

    let deck_before: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();

    game.play_to_stage(maki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let deck_after: Vec<i16> = game.state.player1.main_deck.cards.iter().copied().collect();
    assert_eq!(deck_before, deck_after, "Deck unchanged when no μ's live card in discard");
}

// ============================================================
// Sub-action 2: Conditional draw based on opponent Wait state
// ============================================================

/// Debut: opponent has no Wait member → no draw (move still happens)
#[test]
fn maki_pb1_no_opponent_wait_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    let mus_live = game.id(MUS_LIVE);

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);

    let opp_member = game.id(FILLER);
    game.state.player2.stage.stage[0] = opp_member;
    // Active by default (no orientation modifier)

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.give_energy(20);

    let deck_before_len = game.state.player1.main_deck.cards.len();
    game.play_to_stage(maki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Hand: maki removed from hand (1 left), no draw → remains 1
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(hand_after, 1, "No draw without Wait member, got {}", hand_after);
    // Move still happened: waitroom→deck_top (+1), no draw → deck 10→11
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before_len + 1,
        "Move is unconditional even when draw condition fails"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.first().copied().unwrap_or(-1),
        mus_live
    );
}

/// Debut: opponent has Wait member → draw 1 card (move+draw net deck unchanged, hand unchanged)
#[test]
fn maki_pb1_opponent_wait_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    let mus_live = game.id(MUS_LIVE);

    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);

    // Set opponent Wait member BEFORE play_to_stage so debut resolves with it
    let opp_member = game.id(FILLER);
    game.state.player2.stage.stage[0] = opp_member;
    game.state.mods.add_orientation_modifier(opp_member, "wait");

    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }
    game.give_energy(20);

    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    game.play_to_stage(maki, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Move waitroom→deck_top (+1) then draw (-1) → net deck unchanged
    let deck_after = game.state.player1.main_deck.cards.len();
    let hand_after = game.state.player1.hand.cards.len();
    assert_eq!(
        deck_after, deck_before,
        "Move (+1) + Draw (-1) net deck unchanged: before={}, after={}",
        deck_before, deck_after
    );
    assert_eq!(
        hand_after, hand_before,
        "Net hand unchanged (play -1, draw +1): before={}, after={}",
        hand_before, hand_after
    );
    // Waitroom card was moved to deck then drawn to hand
    assert!(
        !game.state.player1.waitroom.cards.contains(&mus_live),
        "μ's live should leave waitroom"
    );
    assert!(
        game.state.player1.hand.cards.contains(&mus_live),
        "Draw should bring the just-placed μ's live (or top deck card) to hand; hand={:?}",
        game.state.player1.hand.cards
    );
}

// ============================================================
// Edge cases
// ============================================================

/// Wait in different positions still triggers (any stage slot)
#[test]
fn maki_pb1_wait_in_all_positions_triggers() {
    for pos in 0..3 {
        let db = load_real_database();
        let mut game = TestGame::new(db);
        let maki = game.id(MAKI_PB1);
        let filler = game.id(FILLER);
        let mus_live = game.id(MUS_LIVE);
        game.state.player1.hand.cards.push(maki);
        game.state.player1.hand.cards.push(filler);
        game.state.player1.waitroom.cards.push(mus_live);
        let opp = game.id(FILLER);
        game.state.player2.stage.stage[pos] = opp;
        game.state.mods.add_orientation_modifier(opp, "wait");
        for _ in 0..10 {
            game.state.player1.main_deck.cards.push(filler);
        }
        game.give_energy(20);
        let hand_before = game.state.player1.hand.cards.len();
        game.play_to_stage(maki, MemberArea::Center);
        while game.has_pending_choice() {
            game.select_indices(&[0]);
        }
        assert_eq!(
            game.state.player1.hand.cards.len(),
            hand_before,
            "Wait at pos {} should trigger draw",
            pos
        );
    }
}

/// Opponent has multiple Wait members → still only draw 1 (not per Wait)
#[test]
fn maki_pb1_multiple_waits_still_draw_one() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    let mus_live = game.id(MUS_LIVE);
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);
    for pos in 0..3 {
        let opp = game.id(FILLER);
        // Use distinct ids per pos to avoid HashMap collision on orientation? same id ok
        game.state.player2.stage.stage[pos] = opp + pos as i16;
        game.state.mods.add_orientation_modifier(opp + pos as i16, "wait");
    }
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(20);
    let deck_before = game.state.player1.main_deck.cards.len();
    game.play_to_stage(maki, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "Multiple Waits should still net deck unchanged (draw 1)"
    );
    assert_eq!(game.state.player1.hand.cards.len(), 2);
}

/// Active (non-Wait) opponent member should NOT trigger draw
#[test]
fn maki_pb1_active_opponent_no_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    let mus_live = game.id(MUS_LIVE);
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);
    let opp = game.id(FILLER);
    game.state.player2.stage.stage[1] = opp;
    game.state.mods.add_orientation_modifier(opp, "active");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(game.state.player1.hand.cards.len(), 1, "Active opponent should not trigger draw");
}

/// No μ's live in waitroom but Wait present → draw still happens, deck -1
#[test]
fn maki_pb1_no_mus_but_wait_still_draws() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    // Waitroom has non-μ's member, not live → move does nothing
    game.state.player1.waitroom.cards.push(filler);
    let opp = game.id(FILLER);
    game.state.player2.stage.stage[0] = opp;
    game.state.mods.add_orientation_modifier(opp, "wait");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(20);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();
    game.play_to_stage(maki, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // No card to move (skip choice?), then draw → deck -1, hand net 0
    // The move choice may be presented with 0 options and auto-skipped
    assert_eq!(
        game.state.player1.main_deck.cards.len() + 1,
        deck_before,
        "Draw should still fire even when no μ's live to place"
    );
    assert_eq!(game.state.player1.hand.cards.len(), hand_before);
}

/// Empty deck edge: move places card then draw draws it (deck empty → refresh not needed)
#[test]
fn maki_pb1_empty_deck_still_works() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    let mus_live = game.id(MUS_LIVE);
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);
    let opp = game.id(FILLER);
    game.state.player2.stage.stage[0] = opp;
    game.state.mods.add_orientation_modifier(opp, "wait");
    // No cards in deck
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    // After move, deck has 1 (mus_live), then draw consumes it → deck 0, hand has mus_live
    assert_eq!(game.state.player1.main_deck.cards.len(), 0);
    assert!(game.state.player1.hand.cards.contains(&mus_live));
}

/// Both players have Wait: only opponent Wait matters (self Wait alone → no draw)
#[test]
fn maki_pb1_self_wait_not_enough() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let maki = game.id(MAKI_PB1);
    let filler = game.id(FILLER);
    let mus_live = game.id(MUS_LIVE);
    game.state.player1.hand.cards.push(maki);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.waitroom.cards.push(mus_live);
    game.state.player2.stage.stage[0] = game.id(FILLER); // opponent active
    // Self has a Wait member but opponent doesn't
    let self_wait = game.id(FILLER);
    game.state.player1.stage.stage[0] = self_wait; // will be replaced by maki? actually maki goes to center, so left slot wait remains
    game.state.player1.stage.stage[1] = maki; // will be overwritten - instead pre-place self wait at left
    // Reset: put maki in hand, self wait at left before play
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = self_wait;
    game.state.mods.add_orientation_modifier(self_wait, "wait");
    // Ensure center empty for maki
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.give_energy(20);
    game.play_to_stage(maki, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Only opponent Wait should trigger, self Wait alone should not"
    );
}
