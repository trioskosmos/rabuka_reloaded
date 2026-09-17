use crate::helpers::*;
use crate::test_modules::support::ability_trigger_and_deck_setup::*;

#[test]
fn full_group_cost_at_least_twenty_draw_and_topdeck_preserves_hand_count() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!N-bp4-031-L");
    let n = g.id("PL!N-bp1-001-R"); // cost 9 虹ヶ咲
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [n, n, n]; // 27 >= 20
    g.state.player1.hand.cards.push(l);
    for _ in 0..5 {
        g.state.player1.hand.cards.push(g.id("PL!-sd1-010-SD"));
    }
    // Fill P1 deck with 虹ヶ咲 so the draw-3 finds matching cards
    g.state.player1.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player1.main_deck.cards.push(n);
    }
    g.state.player2.main_deck.cards.clear();
    for _ in 0..30 {
        g.state.player2.main_deck.cards.push(f);
    }
    g.give_energy(5);
    let h = g.state.player1.hand.cards.len();
    trigger_printed_ability_and_resolve_choices(&mut g, l, "ライブ開始時");
    assert_eq!(g.state.player1.hand.cards.len(), h, "draw3 put3 = net0");
}
#[test]
fn full_group_cost_below_twenty_preserves_hand_count() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let l = g.id("PL!N-bp4-031-L");
    let n = g.id("PL!N-bp4-013-N"); // cost 4
    let f = g.id("PL!-sd1-010-SD");
    g.state.player1.stage.stage = [n, n, n]; // 12 < 20
    g.state.player1.hand.cards.push(l);
    g.state.player1.hand.cards.push(f);
    fill_both_main_decks(&mut g, f);
    g.give_energy(5);
    let h = g.state.player1.hand.cards.len();
    trigger_printed_ability_and_resolve_choices(&mut g, l, "ライブ開始時");
    assert_eq!(g.state.player1.hand.cards.len(), h, "no draw");
}
