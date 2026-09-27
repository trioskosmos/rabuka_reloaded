//! Five more 常時 whose grant is set by state this deck can drive precisely: an
//! exact energy count, a stage-cost total compared against the OPPONENT, an AND of
//! two energy clauses, an exclusion clause (「ほかの」), and a set of three grants
//! keyed to the holder's own SLOT.
//!
//! All of these sat at L1 with a single direct test, which the depth metric records
//! for the same reason as the previous file: one positive reading cannot tell a
//! correct implementation from one that accepts a whole band. Each is therefore
//! tested on both sides of its boundary, and in the case of 矢澤にこ and 鬼塚冬毬
//! the "wrong side" is a legal board position the positive case never visits.
//!
//! 鬼塚冬毬 is the structurally interesting one. Her three 常時 carry
//! `condition: null` — the slot gate is the `{{leftside.png|左サイド}}` PREFIX on the
//! effect text, not a condition node. So nothing in the condition machinery is
//! involved, and moving her between slots is the only thing that changes which
//! grant applies. A test that only reads one slot cannot see the other two at all.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::zones::MemberArea;

const NIKO: &str = "PL!-PR-021-PR"; // 矢澤にこ — ちょうど7 energy → 2 blades
const NATSUMI: &str = "PL!SP-bp4-009-R"; // 鬼塚夏美 — stage cost total < opponent → 3 blades
const KAKKO: &str = "PL!SP-bp7-002-R"; // 唐 可可 — (energy >= 7 AND > opponent) → self cost +2
const FUYURI: &str = "PL!SP-bp5-011-R"; // 鬼塚冬毬 — three grants keyed to her own slot
const SAYAKA: &str = "PL!HS-bp6-012-R"; // 百生吟子 — 「ほかの」スリーズブeke member → 1 energy
/// A スリーズブケー member that is NOT 百生吟子, for her ほかの clause.
const OTHER_SUR: &str = "PL!HS-bp1-012-PR";
const FILLER: &str = "PL!-sd1-010-SD";

fn blade_mod(game: &TestGame, card: i16) -> i32 {
    game.state.mods.get_blade_modifier(card)
}

fn heart_mod(game: &TestGame, card: i16, colour: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card, colour)
}

fn cost_mod(game: &TestGame, card: i16) -> i32 {
    game.state.mods.get_cost_modifier(card)
}

fn energy_cards(game: &TestGame) -> usize {
    game.state.player1.energy_zone.cards.len()
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

/// Put exactly `total` ENERGY cards in p1's zone, all active.
fn set_energy(game: &mut TestGame, total: usize) {
    let energy = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..total {
        game.state.player1.energy_zone.cards.push(game.new_id("LL-E-001-SD"));
    }
    game.state.player1.energy_zone.set_active_count(total as u8);
    let _ = energy;
    assert_eq!(
        energy_cards(game),
        total,
        "precondition: the energy zone must hold exactly {total}"
    );
}

// ====================================================================
// 矢澤にこ — 「ちょうど7枚」 on the energy zone
// ====================================================================

/// `card_count_condition`, `location: energy_zone`, `count: 7, operator: "="`.
///
/// Unlike 合計6人 (which no legal board can exceed, because two 3-slot stages top
/// out at 6), the energy zone has no such ceiling, so BOTH sides of the equality are
/// reachable and this is a real `=` test: 7 grants, and 6 and 8 do not.
#[test]
fn niko_two_blades_only_at_exactly_seven_energy_cards() {
    let mut game = TestGame::new(load_real_database());
    let niko = game.id(NIKO);
    game.assert_card_identity(niko, NIKO);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [niko, -1, -1];

    // Below the boundary.
    set_energy(&mut game, 6);
    game.state.recalculate_constants();
    assert_eq!(
        blade_mod(&game, niko),
        0,
        "『ちょうど7枚』 is false at 6 — a 「7枚以上」 reading would grant here"
    );

    // The boundary.
    set_energy(&mut game, 7);
    game.state.recalculate_constants();
    assert_eq!(
        blade_mod(&game, niko),
        2,
        "exactly 7 energy grants 2 blades — the printed amount, not one"
    );

    // OVER the boundary, which is the case an inequality gets wrong and the one
    // 合計6人 could never test.
    set_energy(&mut game, 8);
    game.state.recalculate_constants();
    assert_eq!(
        blade_mod(&game, niko),
        0,
        "『ちょうど7枚』 is ALSO false at 8 — a 7枚以上 reading grants here, and this \
         is the assertion that separates the two"
    );

    // And back down, so the zero above is the count and not a one-shot grant.
    set_energy(&mut game, 7);
    game.state.recalculate_constants();
    assert_eq!(
        blade_mod(&game, niko),
        2,
        "returning to 7 re-grants: the condition is re-derived on every scan"
    );
}

// ====================================================================
// 鬼塚夏美 — own stage cost total, compared against the OPPONENT
// ====================================================================

/// `comparison_condition`, `aggregate: "total"`, `comparison_type: "cost"`,
/// `operator: "<"`, `comparison_target: "opponent"`.
///
/// The opponent's board is half the condition, which is the easiest half to omit.
/// All three members go on ONE side and the other side's total moves, so a
/// comparison against one's own stage cannot pass.
#[test]
fn natsumi_three_blades_while_own_stage_cost_total_is_below_the_opponents() {
    let mut game = TestGame::new(load_real_database());
    let natsumi = game.id(NATSUMI);
    game.assert_card_identity(natsumi, NATSUMI);
    fill_decks(&mut game);

    // 鬼塚夏美 prints cost 9. One filler (cost 6) with her → 15. Give p2 18 so
    // 15 < 18 holds.
    game.state.player1.stage.stage = [natsumi, -1, -1];
    game.state.player2.stage.stage = [
        game.new_id(FILLER),
        game.new_id(FILLER),
        game.new_id(FILLER),
    ];
    game.state.recalculate_constants();
    let stage_cost = |game: &TestGame, player: u8| -> u32 {
        let stage = if player == 1 {
            &game.state.player1.stage.stage
        } else {
            &game.state.player2.stage.stage
        };
        stage
            .iter()
            .filter(|c| **c != -1)
            .map(|c| game.state.card_database.get_card(*c).unwrap().cost.unwrap_or(0) as u32)
            .sum()
    };
    assert!(
        stage_cost(&game, 1) < stage_cost(&game, 2),
        "precondition: p1 {} must be BELOW p2 {} for the positive case",
        stage_cost(&game, 1),
        stage_cost(&game, 2)
    );
    assert_eq!(
        blade_mod(&game, natsumi),
        3,
        "own stage total below the opponent's grants 3 blades"
    );

    // p2 empties: 0 is no longer "higher than 15", so the grant goes.
    for slot in 0..3 {
        let leaving = game.state.player2.stage.stage[slot];
        game.state.player2.stage.stage[slot] = -1;
        if leaving != -1 {
            game.state.on_cards_left_zones(&[leaving]);
        }
    }
    game.state.recalculate_constants();
    assert!(
        stage_cost(&game, 1) > 0,
        "precondition: p1's total is still {} and p2's is now 0",
        stage_cost(&game, 1)
    );
    assert_eq!(
        blade_mod(&game, natsumi),
        0,
        "『相手より低い』 is false once the opponent's stage is empty — the comparison \
         really is against the OPPONENT's board, and an implementation comparing p1 \
         against itself would grant here"
    );
}

// ====================================================================
// 唐 可可 — an AND of two energy clauses, applied to her OWN cost
// ====================================================================

/// `compound: and` of [energy `>= 7`] and [energy `> opponent`], whose effect is
/// `modify_cost +2` on HERSELF.
///
/// Two clauses that fail independently, and an effect that changes a COST rather
/// than granting a resource — so the observable is `get_cost_modifier`, which
/// nothing in the neighbouring files reads.
#[test]
fn kakko_costs_two_more_only_when_both_energy_clauses_hold() {
    let mut game = TestGame::new(load_real_database());
    let kakko = game.id(KAKKO);
    game.assert_card_identity(kakko, KAKKO);
    fill_decks(&mut game);
    game.state.player1.stage.stage = [kakko, -1, -1];
    let energy = game.id("LL-E-001-SD");

    // Clause 1 fails: 6 < 7, even though p1 leads the opponent.
    set_energy(&mut game, 6);
    for _ in 0..3 {
        game.state
            .player2
            .energy_zone
            .cards
            .push(game.new_id("LL-E-001-SD"));
    }
    game.state.player2.energy_zone.set_active_count(3);
    game.state.recalculate_constants();
    assert!(
        game.state.player2.energy_zone.active_count()
            < game.state.player1.energy_zone.active_count(),
        "precondition: p1 leads, so ONLY the 7枚以上 clause is failing"
    );
    assert_eq!(
        cost_mod(&game, kakko),
        0,
        "6 energy is below 『7枚以上』, and the condition is an AND — one failing \
         clause withholds the whole +2"
    );

    // Both clauses hold: 7 and p1 still leads.
    set_energy(&mut game, 7);
    game.state.recalculate_constants();
    assert_eq!(
        cost_mod(&game, kakko),
        2,
        "7 energy AND p1 ahead grants the +2 to her own cost"
    );
    let _ = energy;

    // Clause 2 fails: p1 no longer leads, clause 1 still holds.
    game.state.player2.energy_zone.cards.clear();
    for _ in 0..9 {
        game.state
            .player2
            .energy_zone
            .cards
            .push(game.new_id("LL-E-001-SD"));
    }
    game.state.player2.energy_zone.set_active_count(9);
    game.state.recalculate_constants();
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        7,
        "precondition: p1 still has 7 energy, so 『7枚以上』 holds"
    );
    assert!(
        game.state.player2.energy_zone.active_count()
            > game.state.player1.energy_zone.active_count(),
        "precondition: p2 now leads, so ONLY the 相手より多い clause is failing"
    );
    assert_eq!(
        cost_mod(&game, kakko),
        0,
        "『自分のエネルギーが相手より多いかぎり』 is false, and an AND withholds the \
         grant — an OR reading would still grant here"
    );
}

// ====================================================================
// 百生吟子 — 「ほかの」, an explicit self-exclusion
// ====================================================================

/// `group_condition` with **`exclude_self: true`** — 「自分のステージに**ほかの**
/// 『スリーズブーケ』のメンバーがいる場合」.
///
/// The exclusion is the whole difficulty. A member of the right group who is the
/// card herself satisfies a naive `group_names` check and would grant; the printed
/// ほかの is what forbids it, and nothing else in the condition carries that.
#[test]
fn sayaka_no_energy_from_her_own_presence_but_yes_from_another_suriesubeke_member() {
    let mut game = TestGame::new(load_real_database());
    let sayaka = game.id(SAYAKA);
    let other = game.id(OTHER_SUR);
    game.assert_card_identity(sayaka, SAYAKA);
    game.assert_card_identity(other, OTHER_SUR);
    // The partner must be in the SAME unit as 吟子 herself, read from the database
    // rather than typed: the unit name is Japanese and a hand-written copy of it
    // is exactly the kind of near-miss that makes a group premise pass vacuously.
    let sayaka_unit = game
        .state
        .card_database
        .get_card(sayaka)
        .unwrap()
        .unit
        .clone();
    let other_unit = game
        .state
        .card_database
        .get_card(other)
        .unwrap()
        .unit
        .clone();
    assert!(
        sayaka_unit.is_some(),
        "precondition: 吟子 must print a unit, or the group clause is untested"
    );
    assert_eq!(
        other_unit, sayaka_unit,
        "precondition: the partner must carry the SAME unit as 吟子 ({sayaka_unit:?})"
    );
    assert!(
        !game.db.get_card(other).unwrap().name.contains("吟子"),
        "precondition: the partner must be a DIFFERENT character — a second print of \
         吟子 would make ほかの ambiguous"
    );
    fill_decks(&mut game);
    // 6 energy with 5 active and 1 WAITED. Playing 吟子 costs 2, taking active to 3,
    // and her effect activates the one waited card, taking it to 4. So the reading is
    // 4 WITH the effect and 3 WITHOUT it — an exact one-card difference that no cost
    // accounting can absorb. Leaving only 1 active would make the play itself
    // unpayable and hide the effect behind a cost failure.
    game.give_energy(6);
    game.state.player1.energy_zone.set_active_count(5);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        5,
        "precondition: 5 active"
    );
    assert_eq!(
        game.state.player1.energy_zone.cards.len()
            - game.state.player1.energy_zone.active_count() as usize,
        1,
        "precondition: exactly one waited energy for her 登場 to activate"
    );

    // NEGATIVE first, and it has to be a real PLAY: her effect is 登場, so staging
    // her directly never runs it. Alone on stage she is her own only same-unit
    // member, and ほかの excludes her — so the counter stops at the cost.
    game.add_to_hand(sayaka);
    game.play_to_stage(sayaka, MemberArea::Center);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        3,
        "playing 吟子 with NO other same-unit member on stage activates nothing: \
         5 - 2 (cost) = 3, and the waited card stays waited. 『ほかの』 is the whole \
         clause, and her own presence must not satisfy it."
    );

    // POSITIVE: a DIFFERENT member of the same unit is already on stage when she
    // arrives, which is the only way the condition can hold at 登場 time.
    let mut positive = TestGame::new(load_real_database());
    let sayaka2 = positive.id(SAYAKA);
    let other2 = positive.id(OTHER_SUR);
    fill_decks(&mut positive);
    positive.give_energy(6);
    positive.state.player1.energy_zone.set_active_count(5);
    positive.state.player1.stage.stage[0] = other2;
    positive.add_to_hand(sayaka2);

    positive.play_to_stage(sayaka2, MemberArea::Center);
    while positive.has_pending_choice() {
        positive.select_indices(&[]);
    }

    assert_eq!(
        positive.state.player1.energy_zone.active_count(),
        4,
        "with another same-unit member already staged the waited card is activated \
         too: 5 - 2 (cost) + 1 (effect) = 4. The one-card difference from the negative \
         case above is the whole claim, and it is what a missing ほかの exclusion \
         would erase."
    );
}

// ====================================================================
// 鬼塚冬毬 — three grants keyed to her own SLOT
// ====================================================================

/// Left slot → 3× heart02, centre → 3× heart03, right slot → 3× heart05.
///
/// All three 常時 carry `condition: null`: the slot gate is the
/// `{{leftside.png|左サイド}}` PREFIX on the effect text, so no condition node is
/// involved at all. That makes moving her the only thing that changes the answer,
/// and it is why all three must be read in ONE test — three tests each holding her
/// still would pin the same mechanism three times and never show the swap.
#[test]
fn fuyuri_which_three_hearts_she_grants_depends_entirely_on_her_slot() {
    let mut game = TestGame::new(load_real_database());
    let fuyuri = game.id(FUYURI);
    game.assert_card_identity(fuyuri, FUYURI);
    fill_decks(&mut game);

    let read_all = |game: &TestGame| {
        (
            heart_mod(game, fuyuri, HeartColor::Heart02),
            heart_mod(game, fuyuri, HeartColor::Heart03),
            heart_mod(game, fuyuri, HeartColor::Heart05),
        )
    };

    // LEFT: heart02 only.
    game.state.player1.stage.stage = [fuyuri, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        read_all(&game),
        (3, 0, 0),
        "in the LEFT slot she grants 3× heart02 and nothing else"
    );

    // CENTRE: heart03 only — the heart02 grant must be GONE, not accumulated.
    game.state.player1.stage.stage = [-1, fuyuri, -1];
    game.state.recalculate_constants();
    assert_eq!(
        read_all(&game),
        (0, 3, 0),
        "moving to the CENTRE swaps the grant: heart02 is withdrawn and heart03 \
         granted — an accumulating implementation would read (3, 3, 0)"
    );

    // RIGHT: heart05 only.
    game.state.player1.stage.stage = [-1, -1, fuyuri];
    game.state.recalculate_constants();
    assert_eq!(
        read_all(&game),
        (0, 0, 3),
        "the RIGHT slot grants 3× heart05, and the centre's heart03 is withdrawn"
    );

    // Off the board entirely: all three withdrawn.
    game.state.player1.stage.stage = [-1, -1, -1];
    game.state.on_cards_left_zones(&[fuyuri]);
    game.state.recalculate_constants();
    assert_eq!(
        read_all(&game),
        (0, 0, 0),
        "with her off stage no slot gate is satisfied, so all three grants are \
         withdrawn — rule 4.1.4 makes the leaving card a new card"
    );
}
