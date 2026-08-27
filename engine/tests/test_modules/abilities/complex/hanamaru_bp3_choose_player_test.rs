/// PL!S-bp3-007-R 国木田花丸 (ab#0) choose_target_player
/// {{kidou.png|起動}}{{turn1.png|ターン1回}}{{icon_energy.png|E}}：自分か相手を選ぶ。自分は、そのプレイヤーの控え室にあるライブカードを1枚、そのプレイヤーのデッキの一番下に置く。そうした場合、自分はカードを1枚引く。
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const HANAMARU: &str = "PL!S-bp3-007-R";
const LIVE1: &str = "PL!-sd1-020-SD";
const LIVE2: &str = "PL!-sd1-021-SD";

fn put_hanamaru(game: &mut TestGame) -> i16 {
    let cid = game.id(HANAMARU);
    game.add_to_stage(MemberArea::Center, cid);
    game.give_energy(1);
    // ensure deck has at least 1 card so the "draw 1" after moving live to bottom doesn't just redraw the moved live
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(filler);
    game.state.player2.main_deck.cards.push(game.id("PL!-sd1-010-SD"));
    cid
}

#[test]
fn hanamaru_choose_self_moves_own_live_and_draws() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = put_hanamaru(&mut g);
    let live = g.id(LIVE1);
    g.state.player1.waitroom.cards.push(live);
    let hand_before = g.state.player1.hand.cards.len();
    g.activate_ability(hanamaru);
    g.select_choice_option(0); // self
    // Live selection auto-resolves when exactly one live is eligible — no prompt.
    assert!(
        !g.has_pending_choice(),
        "single-candidate live selection should auto-resolve without prompting"
    );
    g.drain_auto_ability_choices();
    // waitroom should no longer contain a live (the one we put)
    assert!(!g.state.player1.waitroom.cards.iter().any(|&cid| g.db.get_card(cid).map_or(false, |c| c.is_live())), "live moved from discard");
    let last = g.state.player1.main_deck.cards.last().copied();
    assert!(last.is_some_and(|id| g.db.get_card(id).map_or(false, |c| c.is_live())), "deck bottom should be a live, got {:?}", last.map(|id| g.db.get_card(id).map(|c| c.card_no.clone())));
    assert_eq!(g.state.player1.hand.cards.len(), hand_before + 1, "drew 1");
    assert!(g.state.player1.energy_zone.active_count() == 0, "paid 1E");
}

#[test]
fn hanamaru_choose_opponent_moves_opponent_live() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = put_hanamaru(&mut g);
    let opp_live = g.id(LIVE2);
    g.state.player2.waitroom.cards.push(opp_live);
    let p1_hand_before = g.state.player1.hand.cards.len();
    g.activate_ability(hanamaru);
    g.select_choice_option(1); // opponent
    // Live selection auto-resolves when exactly one live is eligible — no prompt.
    assert!(
        !g.has_pending_choice(),
        "single-candidate live selection should auto-resolve without prompting"
    );
    g.drain_auto_ability_choices();
    assert!(!g.state.player2.waitroom.cards.iter().any(|&cid| g.db.get_card(cid).map_or(false, |c| c.is_live())), "opp live moved");
    let last = g.state.player2.main_deck.cards.last().copied();
    assert!(last.is_some_and(|id| g.db.get_card(id).map_or(false, |c| c.is_live())), "opp deck bottom should be live");
    assert_eq!(g.state.player1.hand.cards.len(), p1_hand_before + 1, "self draws even when targeting opponent");
}

#[test]
fn hanamaru_no_live_in_chosen_discard_no_move_no_draw() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = put_hanamaru(&mut g);
    // p1 waitroom has no live, only member
    let mem = g.id("PL!-sd1-010-SD");
    g.state.player1.waitroom.cards.push(mem);
    let hand_before = g.state.player1.hand.cards.len();
    g.activate_ability(hanamaru);
    g.select_choice_option(0); // self
    // Observed: with zero eligible lives the move auto-skips — no prompt is created.
    assert!(
        !g.has_pending_choice(),
        "zero-candidate live selection should auto-skip without prompting"
    );
    assert_eq!(g.state.player1.hand.cards.len(), hand_before, "no live => no draw (conditional)");
    assert!(g.state.player1.waitroom.cards.contains(&mem), "member untouched");
}

#[test]
fn hanamaru_opponent_no_live_no_draw() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = put_hanamaru(&mut g);
    g.state.player2.waitroom.cards.push(g.id("PL!-sd1-010-SD")); // member only
    let hand_before = g.state.player1.hand.cards.len();
    g.activate_ability(hanamaru);
    g.select_choice_option(1); // opponent has no live
    // Observed: with zero eligible lives the move auto-skips — no prompt is created.
    assert!(
        !g.has_pending_choice(),
        "zero-candidate live selection should auto-skip without prompting"
    );
    assert_eq!(g.state.player1.hand.cards.len(), hand_before, "no draw when opponent has no live");
}

#[test]
fn hanamaru_targeted_player_determines_deck_bottom_owner() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = put_hanamaru(&mut g);
    let live_p1 = g.id(LIVE1);
    g.state.player1.waitroom.cards.push(live_p1);
    g.activate_ability(hanamaru);
    g.select_choice_option(0);
    // Live selection auto-resolves when exactly one live is eligible — no prompt.
    assert!(
        !g.has_pending_choice(),
        "single-candidate live selection should auto-resolve without prompting"
    );
    g.drain_auto_ability_choices();
    assert!(g.state.player1.energy_zone.active_count() == 0);
    assert_eq!(
        g.state.player1.main_deck.cards.last(),
        Some(&live_p1),
        "self target: live should be on self deck bottom"
    );
    assert_eq!(g.state.player1.hand.cards.len(), 1, "drew 1 after moving");
}

#[test]
fn hanamaru_cannot_activate_without_energy() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = g.id(HANAMARU);
    g.add_to_stage(MemberArea::Center, hanamaru);
    let hand_before = g.state.player1.hand.cards.len();
    // no energy given – cost is pay 1E mandatory, must not create pending choice or draw
    let _ = g.try_activate_ability(hanamaru);
    assert!(
        !g.has_pending_choice(),
        "with 0 active energy, pay 1E cost cannot be paid so no pending choice should appear"
    );
    assert_eq!(
        g.state.player1.hand.cards.len(),
        hand_before,
        "no draw when cost not paid"
    );
    assert_eq!(g.state.player1.energy_zone.active_count(), 0);
}

#[test]
fn hanamaru_turn_limit_once() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let hanamaru = put_hanamaru(&mut g);
    let live = g.id(LIVE1);
    g.state.player1.waitroom.cards.push(live);
    g.activate_ability(hanamaru);
    g.select_choice_option(0);
    // Live selection auto-resolves when exactly one live is eligible — no prompt.
    assert!(
        !g.has_pending_choice(),
        "single-candidate live selection should auto-resolve without prompting"
    );
    g.drain_auto_ability_choices();
    // second activation same turn should fail
    let live2 = g.id(LIVE2);
    g.state.player1.waitroom.cards.push(live2);
    // need to replenish energy for second attempt (first consumed 1)
    g.give_energy(1);
    let res = g.try_activate_ability(hanamaru);
    assert!(res.is_err(), "turn1 limit should block second activation, got {res:?}");
}
