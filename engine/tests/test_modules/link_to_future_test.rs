/// Tests for PL!HS-bp2-020-L (Link to the FUTURE) — set_card_identity + LiveStart score
///
/// ab#0 (常時):
///   すべての領域にあるこのカードは『スリーズブーケ』、『DOLLCHESTRA』、
///   『みらくらぱーく！』として扱う。
///
/// ab#1 (ライブ開始時):
///   自分のステージにいる名前の異なる『蓮ノ空』のメンバー1人につき、
///   このカードのスコアを＋２する。
///
/// Action types: set_card_identity (ab#0) — unique, only this card
///               modify_score (ab#1) — per distinct member on stage
use crate::helpers::*;

fn advance_to_live_card_set_p1(game: &mut TestGame) {
    assert_eq!(game.state.current_phase.to_string(), "Main");
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

/// LiveStart with 3 distinct 蓮ノ空 members on stage → +6 score (3 * 2).
#[test]
fn link_to_future_three_distinct_members_plus_6() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let link = game.id("PL!HS-bp2-020-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Three distinct 蓮ノ空 members on stage
    let hasu_a = game.id("PL!HS-bp1-001-R"); // 日野下花帆
    let hasu_b = game.id("PL!HS-sd1-001-SD"); // 村野さやか
    let hasu_c = game.id("PL!HS-sd1-002-SD"); // 乙宗梢

    game.state.player1.stage.stage = [hasu_a, hasu_b, hasu_c];

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(link);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(link);
    advance_to_live_start(&mut game);

    let live_card_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_card_id);

    assert_eq!(
        score_mod, 6,
        "3 distinct 蓮ノ空 members should give +6 score mod (2 each)"
    );
    eprintln!(
        "[LINK] Score mod with 3 distinct 蓮ノ空 members: {} ✓",
        score_mod
    );
}

/// LiveStart with 1 蓮ノ空 member → +2 score.
#[test]
fn link_to_future_one_member_plus_2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let link = game.id("PL!HS-bp2-020-L");
    let filler = game.id("PL!-sd1-010-SD");
    let hasu = game.id("PL!HS-bp1-001-R");

    game.state.player1.stage.stage = [-1, hasu, -1];

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(link);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(link);
    advance_to_live_start(&mut game);

    let live_card_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_card_id);

    assert_eq!(score_mod, 2, "1 蓮ノ空 member should give +2 score mod");
    eprintln!("[LINK] Score mod with 1 member: {} ✓", score_mod);
}

/// LiveStart with 0 蓮ノ空 members → +0 score added on top of base score 0.
#[test]
fn link_to_future_zero_members_score_0() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let link = game.id("PL!HS-bp2-020-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [-1, -1, -1];

    for _ in 0..30 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.hand.cards.push(link);
    game.state.player1.hand.cards.push(filler);

    advance_to_live_card_set_p1(&mut game);
    game.set_live_card(link);
    advance_to_live_start(&mut game);

    let live_card_id = game.state.player1.live_card_zone.cards[0];
    let score_mod = game.state.mods.get_score_modifier(live_card_id);

    assert_eq!(
        score_mod, 0,
        "Score should be 0 when no 蓮ノ空 members on stage"
    );
}
