use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

const FILLER: &str = "PL!N-sd1-010-SD";
// PL!SP-sd1-002-SD is a 『Liella!』 card (fetchable); FILLER is not.
const LIELLA: &str = "PL!SP-sd1-002-SD";

/// Stage Wien (R print — identity asserted), fund energy, put exactly
/// `top_five` on deck top (index 0 = top), one discardable card in hand.
fn wien_setup(game: &mut TestGame, top_five: Vec<i16>) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!SP-bp1-010-R");
    assert_eq!(
        game.db.get_card(me).unwrap().card_no,
        "PL!SP-bp1-010-R",
        "staged the R print of Wien, not the P print"
    );
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let mut deck = top_five;
    while deck.len() < 10 {
        deck.push(filler);
    }
    game.state.player1.main_deck.cards = deck.into();
    me
}

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

fn drain(game: &mut TestGame, pick: &[usize]) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(pick);
    }
    assert!(
        !game.has_pending_choice(),
        "prompts should terminate after answering"
    );
}

#[test]
fn pl_sp_bp1_010_r_energy_discard_activation_fetches_liella_from_look_five() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp1-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let kanon = game.new_id("PL!SP-sd1-002-SD");
    game.state.player1.main_deck.cards.insert(0, filler);
    game.state.player1.main_deck.cards.insert(0, kanon);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&kanon),
        "『Liella!』 card revealed to hand after paying 2E + discard"
    );
    // Full cost accounting the original never checked:
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "2E of 5 paid"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "1 in hand - 1 cost discard + 1 fetched = 1"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        5,
        "1 cost discard + 4 unselected looked cards to waitroom"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(me, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(use_offers(&game, me), 0, "no re-offer after consuming");
}

/// Edge: no 『Liella!』 among the top 5 — nothing fetched, all 5 + the
/// cost discard reach the waitroom.
#[test]
fn pl_sp_bp1_010_r_no_liella_in_five_all_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f: Vec<i16> = (0..5).map(|_| game.new_id(FILLER)).collect();
    let me = wien_setup(&mut game, f);

    let hand_before = game.state.player1.hand.cards.len();
    game.activate_ability(me);
    drain(&mut game, &[0]);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "cost paid even with no valid fetch"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 1,
        "only the cost discard left the hand — no fetch"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        6,
        "1 cost discard + all 5 looked cards to waitroom"
    );
}

/// Edge: 『Liella!』 present but the optional fetch is declined — the
/// would-be fetch joins the rest in the waitroom.
#[test]
fn pl_sp_bp1_010_r_decline_fetch_liella_goes_to_waitroom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon = game.new_id(LIELLA);
    let f: Vec<i16> = (0..4).map(|_| game.new_id(FILLER)).collect();
    let mut top = vec![kanon];
    top.extend(f);
    let me = wien_setup(&mut game, top);

    game.activate_ability(me);
    // First prompt is the mandatory cost discard; answer it, then decline
    // the optional fetch with an empty selection.
    assert!(game.has_pending_choice(), "cost discard prompted");
    game.select_indices(&[0]);
    assert!(game.has_pending_choice(), "fetch choice prompted");
    game.select_indices(&[]);
    drain(&mut game, &[0]);

    assert!(
        !game.state.player1.hand.cards.contains(&kanon),
        "declined fetch must not reach the hand"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&kanon),
        "declined fetch joins the rest in the waitroom"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        6,
        "1 cost discard + all 5 looked cards to waitroom"
    );
}

/// Edge: cost unpayable on energy (needs 2) — must not be offered.
#[test]
fn pl_sp_bp1_010_r_unaffordable_energy_not_offered() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f: Vec<i16> = (0..5).map(|_| game.new_id(FILLER)).collect();
    let me = wien_setup(&mut game, f);
    game.state.player1.energy_zone.cards.clear();
    game.state.player1.energy_zone.set_active_count(0);
    game.give_energy(1);
    assert_eq!(
        use_offers(&game, me),
        0,
        "2E cost with 1 energy must not be offered"
    );
}

/// Empty hand means the mandatory discard-1 cannot be paid, so as written
/// the ability is NOT offered and direct activation is refused with Err —
/// before anything is paid, recorded, or queued (Rule 9.4.2.3/Q56).
#[test]
fn pl_sp_bp1_010_r_empty_hand_not_offered_activation_refused() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let f: Vec<i16> = (0..5).map(|_| game.new_id(FILLER)).collect();
    let me = wien_setup(&mut game, f);
    game.state.player1.hand.cards.clear();

    assert_eq!(
        use_offers(&game, me),
        0,
        "mandatory discard with empty hand must not be offered"
    );

    let active_before = game.state.player1.energy_zone.active_count();
    let result = game.try_activate_ability(me);
    assert!(
        result.is_err(),
        "activation with no card to discard must be refused, not fizzled"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "refused activation must not partially pay energy"
    );
    assert!(
        !game.state
            .turn_limited_abilities_used
            .contains_key(&(me, 0, game.state.turn_number)),
        "refused activation must not consume the ターン1回 budget"
    );
    assert!(
        !game.has_pending_choice(),
        "refused activation must leave no dangling prompt"
    );
    assert!(
        game.state.ability_queue.is_empty(),
        "refused activation must not linger in the queue"
    );
}
