/// Comprehensive edges for PL!S-PR-045-PR idx560
/// 登場 コスト7のメンバーからバトンタッチして登場した場合、カードを2枚引き、手札を1枚控え室に置く。
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn deck_len(g: &TestGame) -> usize { g.state.player1.main_deck.cards.len() }
fn hand_len(g: &TestGame) -> usize { g.state.player1.hand.cards.len() }

// Helper that attempts baton over replaced_no to LeftSide, returns whether draw happened
fn try_baton(replaced_no: &str) -> (bool, usize, usize) {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let replaced = g.id(replaced_no);
    g.state.player1.stage.stage[0] = replaced;
    let me = g.new_id("PL!S-PR-045-PR");
    g.state.player1.hand.cards.push(me);
    g.give_energy(25);
    let d1 = g.new_id(FILLER);
    let d2 = g.new_id(FILLER);
    g.state.player1.main_deck.cards.push(d1);
    g.state.player1.main_deck.cards.push(d2);
    let deck_before = deck_len(&g);
    let hand_before = hand_len(&g);
    g.play_to_stage(me, MemberArea::LeftSide);
    let had_choice = g.has_pending_choice();
    if had_choice {
        // Should be hand discard 1
        assert_eq!(g.pending_choice_type().as_deref(), Some("SelectCard"));
        g.select_indices(&[0]);
    }
    let deck_after = deck_len(&g);
    let drew = deck_before - deck_after == 2;
    let _hand_after = hand_len(&g);
    (had_choice && drew, deck_before, deck_after)
}

#[test]
fn pr045_cost7_draws() {
    let (drew, _, _) = try_baton("PL!-sd1-007-SD"); // cost7
    assert!(drew, "cost7 baton should draw 2");
}

#[test]
fn pr045_cost6_no_draw() {
    let (drew, _, _) = try_baton("PL!-sd1-003-SD"); // cost 6
    assert!(!drew, "cost6 baton should NOT draw (fixed)");
}

#[test]
fn pr045_cost8_no_draw() {
    let (drew, _, _) = try_baton("PL!SP-bp5-111-R"); // cost 8
    assert!(!drew, "cost8 baton should NOT draw (fixed)");
}

#[test]
fn pr045_cost4_no_draw() {
    let (drew, _, _) = try_baton("PL!-sd1-001-SD"); // cost 4
    assert!(!drew, "cost4 baton should NOT draw (fixed)");
}

#[test]
fn pr045_non_baton_no_draw_even_with_cost7_on_stage() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let cost7 = g.id("PL!-sd1-007-SD");
    g.state.player1.stage.stage[0] = cost7;
    let me = g.new_id("PL!S-PR-045-PR");
    g.state.player1.hand.cards.push(me);
    g.give_energy(25);
    let d1 = g.new_id(FILLER);
    let d2 = g.new_id(FILLER);
    g.state.player1.main_deck.cards.push(d1);
    g.state.player1.main_deck.cards.push(d2);
    let deck_before = deck_len(&g);
    // Play to empty RightSide, not LeftSide where cost7 sits -> no baton
    g.play_to_stage(me, MemberArea::RightSide);
    // Should have no pending choice (no draw)
    assert!(!g.has_pending_choice(), "non-baton debut should not trigger draw");
    assert_eq!(deck_len(&g), deck_before, "deck should not shrink");
}

#[test]
fn pr045_baton_over_different_cost7_still_draws() {
    let (drew, _, _) = try_baton("PL!-sd1-007-SD"); // cost7
    let (drew2, _, _) = try_baton("PL!-sd1-007-SD"); // same cost7 second copy via new_id
    assert!(drew && drew2, "both cost7 batons should draw");
}

#[test]
fn pr045_hand_size_after_draw_discard_net_plus1() {
    let db = load_real_database();
    let mut g = TestGame::new(db);
    let cost7 = g.id("PL!-sd1-007-SD");
    g.state.player1.stage.stage[0] = cost7;
    let me = g.new_id("PL!S-PR-045-PR");
    g.state.player1.hand.cards.push(me);
    g.give_energy(25);
    let d1 = g.new_id(FILLER);
    let d2 = g.new_id(FILLER);
    g.state.player1.main_deck.cards.push(d1);
    g.state.player1.main_deck.cards.push(d2);
    let hand_before = hand_len(&g); // 1 (me)
    g.play_to_stage(me, MemberArea::LeftSide);
    assert!(g.has_pending_choice());
    g.select_indices(&[0]);
    // Net: drew 2, discarded 1, played 1 -> hand should be before (1) -1 (played) +2 (draw) -1 (discard) =1
    assert_eq!(hand_len(&g), 1, "hand net should be 1 after baton 7 draw2 discard1");
}
