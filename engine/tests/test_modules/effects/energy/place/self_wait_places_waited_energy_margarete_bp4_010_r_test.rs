use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

// PL!SP-bp4-010-R Wien (was happy-path + empty-deck only)
// {{起動}}{{ターン1回}}{{E}}このメンバーをウェイトにする：自分のエネルギー
// デッキから、エネルギーカードを1枚ウェイト状態で置く。
// THE self_cost regression trap: upfront validation must resolve "self" to
// the activating card even though gs.activating_card is unset at offer time.

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

#[test]
fn margarete_bp4_010_r_activation_waits_self_and_places_waited_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);

    let zone_before = game.state.player1.energy_zone.cards.len();
    let active_before = game.state.player1.energy_zone.active_count();

    game.activate_ability(me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "activation cost waits this member"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 1,
        "one energy card placed into the energy zone"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before - 1,
        "the energy cost consumed one active energy; the placed card is WAITED"
    );
    // The placed card itself is waited, not just the net count.
    let placed = *game.state.player1.energy_zone.cards.last().unwrap();
    assert_eq!(
        game.state.mods.get_orientation_modifier(placed),
        Some("wait"),
        "placed energy enters waited"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(me, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(use_offers(&game, me), 0, "no re-offer after consuming");
}

#[test]
fn margarete_bp4_010_r_empty_energy_deck_still_waits_self_without_placing_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    let zone_before = game.state.player1.energy_zone.cards.len();

    game.activate_ability(me);

    assert_eq!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "the wait is the cost and applies regardless"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before,
        "empty energy deck -> nothing placed"
    );
}

/// 1枚 out of the energy DECK is the deck's top card, not a choice.
///
/// The other four tests can't see this: their energy decks hold a single
/// identical card, so "the one card that moved" proves nothing about WHICH
/// card was taken. Two different energy cards can. This pins the rule that
/// matters — your energy deck is face-down, so 1枚 resolves without a prompt
/// and takes the top card — and would fail loudly if that ever turned into a
/// prompt (or silently started taking a random card).
#[test]
fn margarete_bp4_010_r_energy_deck_pick_is_the_top_card_without_a_prompt() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);

    // Two genuinely different energy cards in the energy DECK, in a known order.
    let top = game.id("LL-E-001-SD");
    let below = game.id("PL!-sd1-023-P");
    game.assert_card_identity(top, "LL-E-001-SD");
    game.assert_card_identity(below, "PL!-sd1-023-P");
    game.state.player1.energy_deck.cards.clear();
    game.state.player1.energy_deck.cards.push(top);
    game.state.player1.energy_deck.cards.push(below);
    let deck_before = game.state.player1.energy_deck.cards.len();

    game.activate_ability(me);

    assert!(
        !game.has_pending_choice(),
        "the energy deck is face-down: 1枚 is the top card, so there is no \
         choice to make (a prompt here would mean a hidden zone became \
         player-choosable)"
    );
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before - 1,
        "exactly one energy card left the deck"
    );
    let placed = *game
        .state
        .player1
        .energy_zone
        .cards
        .last()
        .expect("an energy card was placed into the zone");
    assert_eq!(
        placed, top,
        "the card that entered the energy zone must be the deck's TOP card"
    );
    assert_ne!(
        placed, below,
        "the deeper energy card must not be the one taken"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(placed),
        Some("wait"),
        "the placed energy enters waited"
    );
    assert!(
        game.state.player1.energy_deck.cards.contains(&below),
        "the card below the top stayed in the energy deck"
    );
}

/// Edge: already-wait Wien → wait-self cost unpayable → not offered, refused.
/// This is THE self_cost threading trap: upfront validation must resolve
/// "self" to the activating card (not gs.activating_card=None).
#[test]
fn margarete_bp4_010_r_already_wait_refused() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    fill_energy_deck(&mut game, 0, 2);
    game.state.mods.add_orientation_modifier(me, "wait");

    assert_eq!(use_offers(&game, me), 0, "waited Wien: not offered");
    let result = game.try_activate_ability(me);
    assert!(
        result.is_err(),
        "wait-self on an already-wait member must be refused, got {:?}",
        result
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "refused: energy untouched"
    );
}

/// Edge: 0 energy → not offered, refused, self not waited.
#[test]
fn margarete_bp4_010_r_no_energy_refused_without_waiting_self() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id("PL!-sd1-010-SD");
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-bp4-010-R");
    game.state.player1.stage.stage[1] = me;
    fill_energy_deck(&mut game, 0, 2);

    assert_eq!(use_offers(&game, me), 0, "0 energy: not offered");
    let result = game.try_activate_ability(me);
    assert!(result.is_err(), "0 energy: refused, got {:?}", result);
    assert_ne!(
        game.state.mods.get_orientation_modifier(me),
        Some("wait"),
        "refused: self not waited"
    );
}
