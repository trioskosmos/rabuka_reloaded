/// BP07 parser fix B4: PL!SP-bp7-005-R＋ 葉月 恋 ab#0.
///
/// 自動 ターン1回：
///   このメンバーが登場するか、自分のエネルギーがエネルギー置き場から
///   エネルギーデッキに置かれたとき、自分のエネルギーデッキから、エネルギー
///   カードを1枚ウェイト状態で置く。そのエネルギーカードは、次のターンの
///   アクティブフェイズにアクティブしない。
///
/// (Auto, once per turn) When THIS member appears, OR when your energy is placed
/// from the energy zone to the energy deck, place 1 energy card from your energy
/// deck in WAIT state. That energy card does not activate next turn's active phase.
///
/// The parser defect: only the appearance leg ("このメンバーが登場する") survived;
/// the second OR leg ("…エネルギーがエネルギー置き場からエネルギーデッキに
/// 置かれたとき") was dropped. These tests pin BOTH trigger paths.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// Put `count` energy cards in player1's energy deck.
fn seed_energy_deck(game: &mut TestGame, count: usize) {
    for _ in 0..count {
        let energy = game.id("LL-E-001-SD");
        game.state.player1.energy_deck.cards.push(energy);
    }
}

/// Count of energy cards currently in WAIT state in player1's energy zone.
fn wait_energy_count(game: &TestGame) -> usize {
    let zone = &game.state.player1.energy_zone;
    zone.cards.len().saturating_sub(zone.active_count() as usize)
}

fn trigger_auto(game: &mut TestGame) {
    let pid = game.state.player1.id.clone();
    rabuka_engine::turn::TurnEngine::trigger_auto_abilities_for_player(&mut game.state, &pid);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

// ====================================================================
// Trigger leg (a): "このメンバーが登場する" — the member appears
// ====================================================================

/// 葉月恋 appears (played to stage) → 1 energy moves energy_deck → energy_zone,
/// and the placed energy is WAIT (does not add to the active count).
#[test]
fn ren_appearance_triggers_energy_place() {
    const REN_COST: u32 = 9;

    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    seed_energy_deck(&mut game, 5);
    let ren = game.id("PL!SP-bp7-005-R＋");
    game.add_to_hand(ren);
    game.give_energy(REN_COST as usize);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    game.play_to_stage(ren, MemberArea::Center);
    game.drain_auto_ability_choices();

    // Playing 葉月恋 pays her cost (active drops by 9) yet places 1 energy
    // (deck → zone). The placed card must NOT add to the active count.
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before - 1,
        "appearance should move 1 energy out of the energy deck: {} → {}",
        deck_before,
        game.state.player1.energy_deck.cards.len()
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "appearance should place 1 energy into the energy zone: {} → {}",
        zone_before,
        game.state.player1.energy_zone.cards.len()
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - REN_COST as u8,
        "placed energy is WAIT: active only drops by the play cost (9), nothing more"
    );
}

// ====================================================================
// Trigger leg (b): "自分のエネルギーがエネルギー置き場からエネルギーデッキに置かれたとき"
// ====================================================================

/// Energy moves energy_zone → energy_deck → triggers the place-1-wait.
#[test]
fn ren_energy_to_deck_triggers_energy_place() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let wait_before = wait_energy_count(&game);

    // Energy moved from the energy zone to the energy deck. The engine records
    // energy moves with a real card id (move_cards.rs / effects/state.rs), so
    // push the id of an energy card that is actually in the deck.
    let moved = game.state.player1.energy_deck.cards[0];
    game.state
        .push_movement_event(moved, "energy_zone", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    let deck_after = game.state.player1.energy_deck.cards.len();
    let wait_after = wait_energy_count(&game);
    assert_eq!(
        deck_after, deck_before - 1,
        "energy_zone→energy_deck should move 1 energy out of the energy deck: {} → {}",
        deck_before, deck_after
    );
    assert_eq!(
        wait_after, wait_before + 1,
        "energy_zone→energy_deck should place 1 energy in WAIT: {} → {}",
        wait_before, wait_after
    );
}

// ====================================================================
// Edge cases
// ====================================================================

/// The placed energy must be in WAIT, not active.
#[test]
fn ren_placed_energy_is_wait_not_active() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let active_before = game.state.player1.energy_zone.active_count();
    let total_before = game.state.player1.energy_zone.cards.len();

    let moved = game.state.player1.energy_deck.cards[0];
    game.state
        .push_movement_event(moved, "energy_zone", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "the placed energy must be WAIT — active count unchanged"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        total_before + 1,
        "total energy zone cards should increase by 1"
    );
}

/// ターン1回 — once per turn even if BOTH legs fire in the same turn.
#[test]
fn ren_once_per_turn_when_both_triggers() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 葉月恋 already on stage; energy deck has several cards.
    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let deck_before = game.state.player1.energy_deck.cards.len();

    // First trigger: energy_zone → energy_deck.
    let moved = game.state.player1.energy_deck.cards[0];
    game.state
        .push_movement_event(moved, "energy_zone", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    let deck_after_first = game.state.player1.energy_deck.cards.len();
    assert_eq!(
        deck_after_first,
        deck_before - 1,
        "first trigger should consume 1 energy (once per turn)"
    );

    // Second trigger in the same turn: appearance (via a debut record) + another
    // energy_zone → energy_deck event. The ターン1回 gate must block a second fire.
    game.state.record_card_appearance(ren, "stage");
    let moved2 = game.state.player1.energy_deck.cards[0];
    game.state
        .push_movement_event(moved2, "energy_zone", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_after_first,
        "ターン1回: second trigger in the same turn must NOT place another energy"
    );
}

/// Empty energy deck → no card can be placed (no crash, no phantom placement).
#[test]
fn ren_empty_energy_deck_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];

    let total_before = game.state.player1.energy_zone.cards.len();

    game.state
        .push_movement_event(-1, "energy_zone", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        total_before,
        "empty energy deck → nothing placed"
    );
}

/// No trigger event at all → the ability must NOT fire.
#[test]
fn ren_no_trigger_no_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let total_before = game.state.player1.energy_zone.cards.len();

    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        total_before,
        "no trigger → no energy placed"
    );
}

// ====================================================================
// Additional edge cases
// ====================================================================

/// An OPPONENT's energy_zone→energy_deck move must NOT trigger 葉月恋's
/// ability ("自分のエネルギー" = your energy only).
#[test]
fn ren_opponent_energy_move_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let deck_before = game.state.player1.energy_deck.cards.len();

    let moved = game.state.player1.energy_deck.cards[0];
    // cause_player_id "p2" → opponent energy move.
    game.state
        .push_movement_event(moved, "energy_zone", "energy_deck", None, "p2", true);
    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "opponent's energy_zone→energy_deck must not trigger (self-only)"
    );
}

/// The member appears via a DIFFERENT character's appearance event — the
/// appearance leg requires THIS member (葉月恋) to actually debut. Pushing a
/// movement event for another card must not fire the appearance leg.
#[test]
fn ren_other_member_appearance_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let deck_before = game.state.player1.energy_deck.cards.len();

    let other = game.id("PL!SP-bp7-006-R＋");
    // Record an appearance for a DIFFERENT member, not 葉月恋.
    game.state.record_card_appearance(other, "stage");
    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "another member's appearance must not trigger 葉月恋's self-appearance leg"
    );
}

/// Two consecutive energy_zone→energy_deck moves in the SAME scan batch still
/// only place ONE energy (the OR/once-per-turn fires a single instance).
#[test]
fn ren_multiple_energy_moves_single_fire() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let deck_before = game.state.player1.energy_deck.cards.len();
    let wait_before = wait_energy_count(&game);

    // Two distinct energy cards moved zone → deck in one batch.
    let moved1 = game.state.player1.energy_deck.cards[0];
    let moved2 = game.state.player1.energy_deck.cards[1];
    game.state
        .push_movement_event(moved1, "energy_zone", "energy_deck", None, "p1", true);
    game.state
        .push_movement_event(moved2, "energy_zone", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    // Effect places exactly 1 card from the energy deck.
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before - 1,
        "two matching moves in one batch → single once-per-turn placement"
    );
    assert_eq!(
        wait_energy_count(&game),
        wait_before + 1,
        "exactly one new WAIT energy"
    );
}

/// Energy moved to the energy deck is currently a LIVE-card-zone / waitroom
/// move (source != energy_zone) → the destination-only condition must NOT
/// fire (source must be energy_zone).
#[test]
fn ren_wrong_source_energy_move_does_not_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ren = game.id("PL!SP-bp7-005-R＋");
    game.state.player1.stage.stage = [-1, ren, -1];
    seed_energy_deck(&mut game, 5);

    let deck_before = game.state.player1.energy_deck.cards.len();

    let moved = game.state.player1.energy_deck.cards[0];
    // Source is discard/waitroom, not energy_zone → condition must not match.
    game.state
        .push_movement_event(moved, "waitroom", "energy_deck", None, "p1", true);
    trigger_auto(&mut game);

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "energy_zone→energy_deck source is required; waitroom→energy_deck must not fire"
    );
}
