/// 乙宗 梢 PL!HS-bp1-003-R＋ ab#0 (常時):
///
/// {{jyouji.png|常時}}自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、
/// かつ名前が異なる場合、「{{jyouji.png|常時}}ライブの合計スコアを＋１する。」を得る。
///
/// (Constant: if EVERY area of your stage holds a 『蓮ノ空』 member and those
/// members all have different names, gain +1 to the live total score.)
///
/// Both tests this replaces were `assert!(true)`. The first also computed a
/// `has_gain` flag that was a tautology — `a > 0 || b >= 0` is always true for
/// a non-negative `b` — so it asserted nothing at all. Note this card is
/// PL!HS-bp1-003-R＋ and PL!HS-bp1-002-R is a DIFFERENT character (村野さやか);
/// 乙宗 梢 and her siblings are one bp number apart, so each print is pinned.
use crate::helpers::*;
use rabuka_engine::game_setup::{self, ActionType};

const KOBE: &str = "PL!HS-bp1-003-R＋"; // 乙宗 梢 — the 常時 source
const SAYAKA: &str = "PL!HS-bp1-002-R"; // 村野さやか — a different 蓮ノ空 member
const TSUREGI: &str = "PL!HS-bp1-004-R＋"; // 夕霧綴理 — a third, also 蓮ノ空

fn use_offers(game: &TestGame, cid: i16) -> usize {
    game_setup::generate_possible_actions(&game.state)
        .iter()
        .filter(|a| {
            a.action_type == ActionType::UseAbility
                && a.parameters.as_ref().and_then(|p| p.card_id) == Some(cid)
        })
        .count()
}

/// Three DIFFERENTLY NAMED 蓮ノ空 members, one per area → the 常時 applies.
#[test]
fn distinct_names_gain_when_all_different() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kobe = game.id(KOBE);
    let sayaka = game.id(SAYAKA);
    let tsuregi = game.id(TSUREGI);
    game.assert_card_identity(kobe, KOBE);
    game.assert_card_identity(sayaka, SAYAKA);
    game.assert_card_identity(tsuregi, TSUREGI);
    // エリアすべて must be filled by three different people; a transposition
    // here would make two slots the same card and quietly satisfy 名前が異なる.
    game.assert_distinct_card_names(kobe, sayaka, "the three 蓮ノ空 members");
    game.assert_distinct_card_names(kobe, tsuregi, "the three 蓮ノ空 members");
    game.assert_distinct_card_names(sayaka, tsuregi, "the three 蓮ノ空 members");

    game.state.player1.stage.stage = [kobe, sayaka, tsuregi];
    game.state.recalculate_constants();
    // Her other half costs E, so hand out one energy before asking whether the
    // 起動 is still generated.
    game.give_energy(1);

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "自分のステージのエリアすべてに『蓮ノ空』のメンバーが登場しており、かつ \
         名前が異なる場合 → ライブの合計スコアを＋１する。"
    );
    assert!(
        use_offers(&game, kobe) >= 1,
        "her 起動 half must still be offered; recalculate_constants must not \
         have disturbed action generation"
    );
}

/// Three copies of the SAME 乙宗 梢 → 名前が異なる fails, so no +1.
#[test]
fn distinct_names_no_gain_when_same_name() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kobe = game.id(KOBE);
    let kobe2 = game.new_id(KOBE);
    let kobe3 = game.new_id(KOBE);
    game.assert_card_identity(kobe, KOBE);
    game.assert_same_card_name(kobe, kobe2, "three copies of 乙宗 梢");
    game.assert_same_card_name(kobe, kobe3, "three copies of 乙宗 梢");
    assert_ne!(kobe, kobe2, "three separate card instances");
    assert_ne!(kobe2, kobe3, "three separate card instances");

    game.state.player1.stage.stage = [kobe, kobe2, kobe3];
    game.state.recalculate_constants();

    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "かつ名前が異なる場合 is not satisfied by three copies of one name"
    );
}

/// An empty area breaks エリアすべてに…登場しており even when the names differ,
/// so the +1 must not be granted for two members on a three-area stage.
#[test]
fn distinct_names_no_gain_when_an_area_is_empty() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kobe = game.id(KOBE);
    let sayaka = game.id(SAYAKA);
    let filler = game.new_id("PL!-sd1-010-SD");
    game.assert_card_identity(kobe, KOBE);
    game.assert_card_identity(sayaka, SAYAKA);
    game.assert_distinct_card_names(kobe, sayaka, "the two 蓮ノ空 members");

    // Right area empty: エリアすべて is not satisfied.
    game.state.player1.stage.stage = [kobe, sayaka, -1];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "my stage does not have a 蓮ノ空 member in EVERY area"
    );

    // Fill the third area with a non-蓮ノ空 member: still not 蓮ノ空 everywhere.
    game.state.player1.stage.stage = [kobe, sayaka, filler];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        0,
        "a non-蓮ノ空 member in the last area still breaks エリアすべてに『蓮ノ空』"
    );

    // Swapping it for 夕霧綴理 satisfies both halves at once.
    let tsuregi = game.id(TSUREGI);
    game.assert_card_identity(tsuregi, TSUREGI);
    game.state.player1.stage.stage = [kobe, sayaka, tsuregi];
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.p1_constant_total_score_bonus,
        1,
        "all three areas hold differently-named 蓮ノ空 members"
    );
}
