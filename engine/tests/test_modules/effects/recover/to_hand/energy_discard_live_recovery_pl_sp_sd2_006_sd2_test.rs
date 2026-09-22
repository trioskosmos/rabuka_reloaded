use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

const FILLER: &str = "PL!-sd1-010-SD";

// PL!SP-sd2-006-SD2 桜小路きな子 (was positive-path only)
// {{起動}}{{ターン1回}}{{E}}{{E}}手札を1枚控え室に置く：自分の控え室から
// 『Liella!』のライブカードを1枚手札に加える。
// Same cost shape as Kasumi (2E + discard → recover live); mirrors her
// hostile matrix: payment accounting, empty-hand refusal, limit.
fn kinako_setup(game: &mut TestGame) -> i16 {
    let filler = game.new_id(FILLER);
    fill_decks(game, filler);
    let me = game.id("PL!SP-sd2-006-SD2");
    assert_eq!(
        game.db.get_card(me).unwrap().card_no,
        "PL!SP-sd2-006-SD2",
        "staged the SD2 print of Kinako"
    );
    game.state.player1.stage.stage[1] = me;
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

fn drain(game: &mut TestGame) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }
    assert!(
        !game.has_pending_choice(),
        "prompts should terminate after answering"
    );
}

#[test]
fn pl_sp_sd2_006_sd2_energy_discard_activation_recovers_liella_live() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    fill_decks(&mut game, filler);

    let me = game.id("PL!SP-sd2-006-SD2");
    game.state.player1.stage.stage[1] = me;
    game.give_energy(5);
    let cost_target = game.new_id(FILLER);
    game.add_to_hand(cost_target);
    let live = game.new_id("PL!SP-bp1-026-L");
    game.state.player1.waitroom.cards.push(live);

    game.activate_ability(me);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[0]);
    }

    assert!(
        game.state.player1.hand.cards.contains(&live),
        "Liella! live card retrieved from the waitroom"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "live left the waitroom"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "2E of 5 paid"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        1,
        "only the cost discard reached the waitroom (live left it)"
    );
    assert!(
        game.state
            .turn_limited_abilities_used
            .contains_key(&(me, 0, game.state.turn_number)),
        "ターン1回 use recorded"
    );
    assert_eq!(use_offers(&game, me), 0, "no re-offer after consuming");
}

/// Edge: empty hand → mandatory discard unpayable → not offered, refused.
#[test]
fn pl_sp_sd2_006_sd2_empty_hand_not_offered_activation_refused() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kinako_setup(&mut game);
    game.give_energy(5);
    let live = game.new_id("PL!SP-bp1-026-L");
    game.state.player1.waitroom.cards.push(live);

    assert_eq!(use_offers(&game, me), 0, "empty hand: not offered");
    let active_before = game.state.player1.energy_zone.active_count();
    let result = game.try_activate_ability(me);
    assert!(
        result.is_err(),
        "empty hand: refused, got {:?}",
        result
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "no partial energy payment"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&live),
        "refused: live stays in waitroom"
    );
}

/// Edge: no Liella! live in waitroom → the recovery has no target.
/// Must terminate with no live gained (Kasumi no-target pattern).
#[test]
fn pl_sp_sd2_006_sd2_no_liella_live_terminates_without_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let me = kinako_setup(&mut game);
    game.give_energy(5);
    game.add_to_hand(game.new_id(FILLER));
    let other_live = game.new_id("PL!HS-bp1-019-L"); // 蓮ノ空 live, not Liella!
    game.add_to_discard(other_live);
    let hand_before = game.state.player1.hand.cards.len();

    // Whatever the engine decides (offer+fizzle or no offer), it terminates
    // and gains nothing.
    let mut activations = 0usize;
    for _ in 0..10 {
        if use_offers(&game, me) == 0 {
            break;
        }
        activations += 1;
        if game.try_activate_ability(me).is_err() {
            break;
        }
        drain(&mut game);
    }
    assert!(
        activations <= 1,
        "must not loop without a valid target"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&other_live),
        "non-Liella live must not be recovered"
    );
    assert!(
        game.state.player1.hand.cards.len() <= hand_before,
        "no net gain without a valid target (hand {} -> {})",
        hand_before,
        game.state.player1.hand.cards.len()
    );
}
