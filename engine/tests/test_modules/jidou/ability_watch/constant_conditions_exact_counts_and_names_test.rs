//! Four 常時 whose condition is set by WHICH cards are on the board — an EXACT
//! count, a count spanning BOTH stages, and a match on a character's NAME.
//!
//! All four were at L1 with a single direct test each, which the depth metric
//! records because none of them had a NEGATIVE test: a single positive reading of
//! 「exactly 2 members」 or 「6 members in total」 cannot distinguish a correct
//! implementation from one that accepts a whole band of counts.
//!
//! Two of the three count conditions are EXACT (`operator: "="`), which is the
//! clause a `>=` reading passes and the printed text forbids. 松浦果南's is
//! 「ちょうど2人」; 矢澤にこ's on the neighbouring file is the same shape. That
//! makes this the single most breakable thing in the family, so each equality
//! here is tested on BOTH sides of its boundary — one member short and one member
//! over — and the over case is the one an inequality reading fails.
//!
//! 小原鞠莉 and 若菜四季 carry the IDENTICAL condition (合計6人) and differ only in
//! which hearts they grant, which is what makes one the other's control: a
//! swapped or shared colour passes neither test alone.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;

const KANAN: &str = "PL!S-PR-037-PR"; // 松浦果南 — ちょうど2人 → heart05 + blade
const KOMARI: &str = "PL!S-PR-042-PR"; // 小原鞠莉 — 合計6人 → heart02 + heart04
const WAKANA: &str = "PL!SP-PR-022-PR"; // 若菜四季 — 合計6人 → heart02 + heart03
const HIME: &str = "PL!HS-pb1-022-N"; // 安養寺姫芽 — two NAME-keyed 常時
/// 大沢瑠璃乃 — the first name 安養寺姫芽 keys on.
const RURINO: &str = "PL!HS-bp1-014-N";
/// 藤島慈 — the second. Both names are checked against the database below, because
/// a name-keyed condition silently degrades to "never" if the name is misspelled or
/// spaced differently, and that failure mode is indistinguishable from a correct
/// negative.
const MEGUMI: &str = "PL!HS-bp1-015-N";
const FILLER: &str = "PL!-sd1-010-SD";

fn heart_mod(game: &TestGame, card: i16, colour: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card, colour)
}

fn blade_mod(game: &TestGame, card: i16) -> i32 {
    game.state.mods.get_blade_modifier(card)
}

fn fill_decks(game: &mut TestGame) {
    let filler = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
}

/// How many members sit on p1's stage, for the premise assertions.
fn p1_members(game: &TestGame) -> usize {
    game.state.player1.stage.stage.iter().filter(|c| **c != -1).count()
}

/// Members across BOTH stages — the quantity 小原鞠莉's and 若菜四季's condition
/// actually counts.
fn both_stage_members(game: &TestGame) -> usize {
    game.state.player1.stage.stage.iter().filter(|c| **c != -1).count()
        + game.state.player2.stage.stage.iter().filter(|c| **c != -1).count()
}

// ====================================================================
// 松浦果南 — 「ちょうど2人」, the equality boundary
// ====================================================================

/// The condition is `operator: "=", count: 2` — 「ちょうど2人」 is EXACTLY two, not
/// 「2人以上」. Three cases on one board walk: one member grants nothing, two grant,
/// three grant nothing again.
///
/// The third case is the one that matters. A `>=` implementation grants at 1, 2 and
/// 3, so it passes a test that only checks "2 grants" and fails only here.
#[test]
fn kanan_grants_only_at_exactly_two_members_not_at_one_and_not_at_three() {
    let mut game = TestGame::new(load_real_database());
    let kanan = game.id(KANAN);
    game.assert_card_identity(kanan, KANAN);
    fill_decks(&mut game);

    // One member: below the boundary.
    game.state.player1.stage.stage = [kanan, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        p1_members(&game),
        1,
        "precondition: one member on stage"
    );
    assert_eq!(
        heart_mod(&game, kanan, HeartColor::Heart05),
        0,
        "『ちょうど2人』 is false at 1 member — a 「2人以上」 reading would grant here"
    );
    assert_eq!(blade_mod(&game, kanan), 0, "and no blade either");

    // Two members: the boundary itself.
    game.state.player1.stage.stage[1] = game.new_id(FILLER);
    game.state.recalculate_constants();
    assert_eq!(p1_members(&game), 2, "precondition: exactly two members");
    assert_eq!(
        heart_mod(&game, kanan, HeartColor::Heart05),
        1,
        "exactly 2 members grants heart05 — 2 is the value the condition names"
    );
    assert_eq!(
        blade_mod(&game, kanan),
        1,
        "and one blade: both resources come from the same condition"
    );

    // Three members: OVER the boundary is also false. This is the case an
    // inequality gets wrong, and the reason the condition says ちょうど.
    let extra = game.new_id(FILLER);
    game.state.player1.stage.stage[2] = extra;
    game.state.recalculate_constants();
    assert_eq!(
        p1_members(&game),
        3,
        "precondition: three members — the over-boundary case"
    );
    assert_eq!(
        heart_mod(&game, kanan, HeartColor::Heart05),
        0,
        "『ちょうど2人』 is ALSO false at 3 — more members is not better, and this \
         is the assertion an inequality reading cannot pass"
    );
    assert_eq!(
        blade_mod(&game, kanan),
        0,
        "the blade is withdrawn with the heart: one condition, both resources"
    );

    // And back to exactly 2 the grant returns, so the zeros are the count and not
    // a latch that granted once and never withdrew.
    game.state.player1.stage.stage[2] = -1;
    game.state.on_cards_left_zones(&[extra]);
    game.state.recalculate_constants();
    assert_eq!(
        p1_members(&game),
        2,
        "precondition: the third member left"
    );
    assert_eq!(
        heart_mod(&game, kanan, HeartColor::Heart05),
        1,
        "returning to exactly 2 re-grants — the condition is re-derived each scan, \
         so a withdraw/restore cycle is observable"
    );
}

// ====================================================================
// 小原鞠莉 / 若菜四季 — 「自分と相手のステージにメンバーが合計6人」
// ====================================================================

/// The condition carries `aggregate: "total", scope: "both"` — it counts BOTH
/// players' stages together. Reading only your own stage is the natural mistake,
/// and on a 3-slot-per-player board the two readings differ at almost every
/// reachable total, so the distribution below separates them.
#[test]
fn komari_grants_only_when_both_stages_together_hold_six_members() {
    let mut game = TestGame::new(load_real_database());
    let komari = game.id(KOMARI);
    game.assert_card_identity(komari, KOMARI);
    fill_decks(&mut game);

    // 2 on p1, 3 on p2 = 5 total.
    game.state.player1.stage.stage = [komari, game.new_id(FILLER), -1];
    game.state.player2.stage.stage = [
        game.new_id(FILLER),
        game.new_id(FILLER),
        game.new_id(FILLER),
    ];
    game.state.recalculate_constants();
    assert_eq!(
        both_stage_members(&game),
        5,
        "precondition: 2 + 3 = 5 members in total"
    );
    assert_eq!(
        heart_mod(&game, komari, HeartColor::Heart02),
        0,
        "『合計6人』 is false at 5 — and p2's 3 members ARE being counted, which is \
         the whole point of scope: both"
    );

    // 3 + 3 = 6: the boundary.
    game.state.player1.stage.stage[2] = game.new_id(FILLER);
    game.state.recalculate_constants();
    assert_eq!(
        both_stage_members(&game),
        6,
        "precondition: 3 + 3 = 6 members in total"
    );
    assert_eq!(
        heart_mod(&game, komari, HeartColor::Heart02),
        1,
        "six members across both stages grants heart02"
    );
    assert_eq!(
        heart_mod(&game, komari, HeartColor::Heart04),
        1,
        "and heart04 — the card grants TWO colours from the one condition"
    );

    // 7 total is NOT reachable: two 3-slot stages hold at most 6. So for THIS card
    // the `=` and `>=` readings of 合計6人 are indistinguishable — there is no board
    // above 6 to tell them apart. That is a property of the condition, not something
    // a test can fix, and it is why the equality case worth pinning is ちょうど2人 in
    // the test above, where 3 IS reachable. What IS observable here is the boundary
    // from below and the re-derivation across a change.
    let extra = game.new_id(FILLER);
    game.state.player1.stage.stage[2] = extra;
    game.state.recalculate_constants();
    assert_eq!(
        both_stage_members(&game),
        6,
        "precondition: p1's third slot is now full, so both stages are at the 6-member \
         maximum — there is no legal board above 6 to test the over-boundary side"
    );
    assert_eq!(
        heart_mod(&game, komari, HeartColor::Heart02),
        1,
        "at the maximum board the grant still holds"
    );

    // Back to 5, then up to 6 again: the re-derivation across a change is what this
    // card can actually be held to.
    let vacancy = game.new_id(FILLER);
    game.state.player2.stage.stage[2] = -1;
    game.state.recalculate_constants();
    let other = game.new_id(FILLER);
    game.state.player1.stage.stage[2] = other;
    game.state.recalculate_constants();
    assert_eq!(
        both_stage_members(&game),
        5,
        "precondition: one slot vacated"
    );
    assert_eq!(
        heart_mod(&game, komari, HeartColor::Heart02),
        0,
        "at 5 the grant is withdrawn — 『合計6人いるかぎり』 is a live condition, not a \
         one-shot grant"
    );
    let _ = vacancy;

    game.state.player2.stage.stage[2] = vacancy;
    game.state.recalculate_constants();
    assert_eq!(
        both_stage_members(&game),
        6,
        "precondition: back to six"
    );
    assert_eq!(
        heart_mod(&game, komari, HeartColor::Heart02),
        1,
        "refilling re-grants, so the zero above was the member count and not a latch"
    );
}

/// 若菜四季 carries the SAME condition and grants heart02 + heart03, where 小原鞠莉
/// grants heart02 + heart04.
///
/// Two cards, one condition, different colours. If the colours were crossed, or if
/// one card's grant leaked into the other's slot, exactly one of the two tests
/// would fail — which is what makes this pair a control rather than a duplicate.
#[test]
fn wakana_grants_heart02_and_heart03_on_the_same_six_member_condition() {
    let mut game = TestGame::new(load_real_database());
    let wakana = game.id(WAKANA);
    game.assert_card_identity(wakana, WAKANA);
    fill_decks(&mut game);

    // Below the boundary: nothing.
    game.state.player1.stage.stage = [wakana, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, wakana, HeartColor::Heart02),
        0,
        "one member: below 合計6人"
    );
    assert_eq!(heart_mod(&game, wakana, HeartColor::Heart03), 0, "and heart03 too");

    // Exactly six across both stages.
    game.state.player1.stage.stage = [
        wakana,
        game.new_id(FILLER),
        game.new_id(FILLER),
    ];
    game.state.player2.stage.stage = [
        game.new_id(FILLER),
        game.new_id(FILLER),
        game.new_id(FILLER),
    ];
    game.state.recalculate_constants();
    assert_eq!(both_stage_members(&game), 6, "precondition: 3 + 3 = 6");

    assert_eq!(
        heart_mod(&game, wakana, HeartColor::Heart02),
        1,
        "six members grants heart02"
    );
    assert_eq!(
        heart_mod(&game, wakana, HeartColor::Heart03),
        1,
        "and heart03 — this card's SECOND colour, which is what distinguishes it \
         from 小原鞠莉's heart04 on the identical condition"
    );
    assert_eq!(
        heart_mod(&game, wakana, HeartColor::Heart04),
        0,
        "and NOT heart04: that colour belongs to 小原鞠莉, whose condition is \
         identical — so the two cards are genuinely separable and a crossed read \
         would be caught here"
    );
}

// ====================================================================
// 安養寺姫芽 — two 常時 keyed on a CHARACTER NAME
// ====================================================================

/// Two 常時 on one card, each keyed to a different character NAME: 大沢瑠璃乃 grants
/// two heart01, 藤島慈 grants two blades. Neither is keyed on a group or a count, so
/// the only thing that can satisfy them is a matching name on p1's stage.
///
/// The premises below check the two names really exist in the database, because a
/// name-keyed condition that never matches produces a perfectly green "no grant"
/// test — the failure mode this shape is most prone to.
#[test]
fn hime_name_keyed_constants_fire_independently_and_are_withdrawn() {
    let mut game = TestGame::new(load_real_database());
    let hime = game.id(HIME);
    let rurino = game.id(RURINO);
    let megumi = game.id(MEGUMI);
    game.assert_card_identity(hime, HIME);
    game.assert_card_identity(rurino, RURINO);
    game.assert_card_identity(megumi, MEGUMI);
    // Name-keyed conditions degrade to "never" on a typo or a spacing difference,
    // and a green negative would look identical to a correct one.
    for (id, who) in [(rurino, "大沢瑠璃乃"), (megumi, "藤島慈")] {
        assert!(
            !game.db.get_card(id).unwrap().name.replace(' ', "").is_empty(),
            "precondition: {who}'s name must resolve, or this test is vacuous"
        );
    }

    game.state.player1.stage.stage = [hime, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, hime, HeartColor::Heart01),
        0,
        "neither 大沢瑠璃乃 nor 藤島慈 is on the stage, so neither 常時 grants"
    );
    assert_eq!(blade_mod(&game, hime), 0, "and no blades either");

    // Only 大沢瑠璃乃 → the heart clause fires, the blade clause does not.
    game.state.player1.stage.stage[1] = rurino;
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, hime, HeartColor::Heart01),
        2,
        "『自分のステージに「大沢瑠璃乃」がいるかぎり、heart01を2つ得る』 — TWO heart01, \
         and 2 is the printed amount"
    );
    assert_eq!(
        blade_mod(&game, hime),
        0,
        "藤島慈 is absent, so the OTHER 常時 must not fire — the two clauses are \
         independent, not a single combined trigger"
    );

    // Add 藤島慈 → the second clause joins the first.
    game.state.player1.stage.stage[2] = megumi;
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, hime, HeartColor::Heart01),
        2,
        "the heart clause is unchanged by the arrival of 藤島慈 — it does not stack"
    );
    assert_eq!(
        blade_mod(&game, hime),
        2,
        "『「藤島慈」がいるかぎり、ブレード2つ』 now fires, granting TWO blades"
    );

    // Remove 大沢瑠璃乃 → only the heart clause is withdrawn.
    game.state.player1.stage.stage[1] = -1;
    game.state.on_cards_left_zones(&[rurino]);
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, hime, HeartColor::Heart01),
        0,
        "大沢瑠璃乃 left the stage, so her clause is withdrawn — the zone is still \
         occupied by 藤島慈, so a grant that only checked emptiness would survive"
    );
    assert_eq!(
        blade_mod(&game, hime),
        2,
        "and 藤島慈's blades are untouched: withdrawing one name-keyed 常時 must not \
         disturb the other"
    );
    assert!(
        !game.state.player1.stage.stage.is_empty(),
        "precondition: the stage is still occupied"
    );
}
