//! Five abilities whose effect is decided by what sits in the 成功ライブカード置き場 —
//! the sharpest form of "this trigger's outcome is set by other cards" in the deck.
//!
//! Three of them had NO direct test at all, and every one of the five had its
//! boundary untested:
//!
//!   * 星空 凛 `PL!-bp6-014-N` / 西木野真姫 `PL!-bp6-015-N` — a 常時 whose condition is
//!     "a 『lilywhite』 / 『BiBi』 card is in my SUCCESS zone". Presence, not a
//!     count, so a card moving in and out has to move the grant with it.
//!   * 矢澤にこ `PL!-bp6-009-R` — a 常時 センター that reads BOTH SIDE SLOTS' printed
//!     blade counts. Two slots, two members, one threshold.
//!   * A song for You! You? You!! `PL!-bp5-022-L` — ライブ開始時, scaling PER CARD in
//!     the success zone (+2 score and one of each of four hearts per card).
//!   * ?←HEARTBEAT `PL!-bp4-021-L` — ライブ開始時 with TWO thresholds on the same
//!     quantity (the success zone's score total): ≥6 relaxes the requirement, ≥9
//!     additionally raises the score.
//!
//! The two scaling cards are the ones worth writing first: a per-card multiplier
//! has several distinguishable wrong answers (flat +2 regardless of count,
//! counting the wrong zone, ignoring the second clause) and a single positive
//! assertion cannot tell them apart. Everything here is an exact number.
//!
//! 成功ライブカード置き場 is `Player::success_live_card_zone`, and the 常時 cards are
//! re-derived on every `recalculate_constants`, so the presence cases drive real
//! scans rather than a hand-set flag.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const RIN: &str = "PL!-bp6-014-N"; // 星空 凛 — lilywhite presence → heart01
const MIKI: &str = "PL!-bp6-015-N"; // 西木野真姫 — BiBi presence → heart06
const NIKO_ALT: &str = "PL!-bp6-009-R"; // 矢澤にこ — side slots' printed blades → +1 score
const SONG: &str = "PL!-bp5-022-L"; // A song for You! You? You!!
const HEARTBEAT: &str = "PL!-bp4-021-L"; // ?←HEARTBEAT
const FILLER: &str = "PL!-sd1-010-SD";
/// What goes INTO the 成功ライブカード置き場 when a test is counting cards there.
/// A real ライブ, because that is what the zone holds: seeding it with a member
/// would build a board the rules forbid, and 「カード1枚につき」 would then be
/// measured against a position that cannot occur.
const COUNT_CARD: &str = "PL!-sd1-001-SD";
/// A 『lilywhite』 ライブ — the card that satisfies 星空 凛's condition.
///
/// The extracted condition pins `card_type: "live_card"` with
/// `location: "success_live_card_zone"`, so a lilywhite MEMBER in that zone
/// satisfies nothing: it is the wrong card type. And `lilywhite` is not a
/// KNOWN_GROUPS name — `card_series_matches_group` covers only the five
/// series-level groups, so `lilywhite` matches through a card's UNIT field. That
/// makes the qualifying set small and worth naming: exactly three of the 291 live
/// cards carry a `lilywhite` unit, and they are joint 蓮ノ空-series prints.
const LILYWHITE_LIVE: &str = "PL!-bp4-025-L";
/// A 『BiBi』 ライブ — the card that satisfies 西木野真姫's condition, for the same
/// reason. Exactly three live cards carry a `BiBi` unit, and NONE of them is also
/// a `lilywhite` one, so the two conditions ARE separable by card after all.
const BIBI_LIVE: &str = "PL!-bp4-026-L";

/// A lilywhite member, for 凛's condition. Asserted by the caller.
/// A lilywhite MEMBER. Used as the wrong-CARD-TYPE fixture: it matches the
/// `lilywhite` group, so the only thing that can keep it out of 星空 凛's condition
/// is the condition's own `card_type: "live_card"` clause.
const LILYWHITE: &str = "PL!-sd1-004-SD";

fn heart_mod(game: &TestGame, card: i16, colour: HeartColor) -> i32 {
    game.state.mods.get_heart_modifier(card, colour)
}

fn score_mod(game: &TestGame, card: i16) -> i32 {
    game.state.mods.get_score_modifier(card)
}

/// p1's ライブの合計スコア constant bonus.
///
/// NOT `get_score_modifier(host)`: this ability's effect carries
/// `target: "live_total"`, which the constant scan deliberately routes into the
/// per-player accumulator instead of a per-card modifier — keying it under the
/// member's own id could never match a live card and silently no-op'd. Reading the
/// per-card modifier here therefore reads a number this card can never move, and
/// the test would fail with a correct engine.
fn total_score_bonus(game: &TestGame) -> i32 {
    i32::from(game.state.mods.p1_constant_total_score_bonus)
}

fn need_heart_mod(game: &TestGame, card: i16, colour: HeartColor) -> i32 {
    game.state.mods.get_need_heart_modifier(card, colour)
}

// ====================================================================
// 星空 凛 / 西木野真姫 — success-zone PRESENCE, granted and withdrawn
// ====================================================================

/// A 常時 that reads a zone's CONTENTS is re-derived on every scan, so a card
/// leaving the success zone must WITHDRAW the grant. Not a latched ライブ開始時 —
/// the opposite, and the pair is what makes the distinction enforceable.
#[test]
fn rin_grants_heart01_while_a_lilywhite_live_is_there_and_not_for_a_member() {
    let mut game = TestGame::new(load_real_database());
    let rin = game.id(RIN);
    let lilywhite_live = game.id(LILYWHITE_LIVE);
    game.assert_card_identity(rin, RIN);
    game.assert_card_identity(lilywhite_live, LILYWHITE_LIVE);
    assert!(
        rabuka_engine::ability::util::card_matches_group_str(
            &game.db,
            lilywhite_live,
            Some("lilywhite")
        ),
        "precondition: the qualifying card must match the lilywhite unit"
    );
    // The lilywhite MEMBER also matches the `lilywhite` group — the group matcher
    // reads the unit field and knows nothing about card types. What excludes it is
    // the condition's own `card_type: "live_card"` clause, so this is a claim about
    // the CONDITION, not the group, and it is asserted behaviourally below rather
    // than through the group matcher.
    let lilywhite_member = game.id(LILYWHITE);
    assert_eq!(
        game.state
            .card_database
            .get_card(lilywhite_member)
            .unwrap()
            .card_type,
        rabuka_engine::card::CardType::Member,
        "precondition: the lilywhite member really is a メンバー, so putting it in a \
         ライブカード zone is the wrong-card-type case"
    );

    game.state.player1.stage.stage = [rin, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, rin, HeartColor::Heart01),
        0,
        "precondition: an EMPTY success zone must not grant — 『かぎり』 is a \
         condition, not a flat bonus"
    );

    // The WRONG card type first: a member in the success zone grants nothing.
    game.state
        .player1
        .success_live_card_zone
        .add_card(lilywhite_member);
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, rin, HeartColor::Heart01),
        0,
        "a lilywhite MEMBER in the success zone does not satisfy a condition that \
         requires a ライブカード there"
    );

    // The qualifying card arrives.
    game.state
        .player1
        .success_live_card_zone
        .add_card(lilywhite_live);
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, rin, HeartColor::Heart01),
        1,
        "a 『lilywhite』 ライブカード in the success zone grants heart01"
    );

    // And it must be withdrawn when the card leaves — the transition a presence
    // grant exists for, and the one a latched grant would get wrong. Removing only
    // the QUALIFYING card is the sharp version: the zone is still non-empty, so a
    // grant that only checked emptiness would survive.
    game.state
        .player1
        .success_live_card_zone
        .cards
        .retain(|c| *c != lilywhite_live);
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, rin, HeartColor::Heart01),
        0,
        "the qualifying card left, so 『かぎり』 no longer holds and the grant must be \
         WITHDRAWN even though the zone is still non-empty — a surviving heart01 \
         would apply to every later live"
    );
    assert!(
        !game.state.player1.success_live_card_zone.cards.is_empty(),
        "precondition: the zone is still holding the non-qualifying member, so this \
         is a membership withdrawal and not an empty-zone case"
    );
}

/// 真姫 is the same shape on a different colour, which is what makes it a control
/// rather than a duplicate: if the group or colour were wired to the wrong thing,
/// 凛's test alone would not notice. The two groups are separable by card — the
/// `BiBi` live below is not also a `lilywhite` one — and the premises state that,
/// so this test would catch a swapped group rather than passing either way.
#[test]
fn miki_grants_heart06_while_a_bibi_live_is_there_and_not_for_a_lilywhite_live() {
    let mut game = TestGame::new(load_real_database());
    let miki = game.id(MIKI);
    let bibi_live = game.id(BIBI_LIVE);
    game.assert_card_identity(miki, MIKI);
    game.assert_card_identity(bibi_live, BIBI_LIVE);
    for (id, group, who) in [
        (bibi_live, "BiBi", "the 『BiBi』 live"),
        (game.id(LILYWHITE_LIVE), "lilywhite", "the lilywhite live"),
    ] {
        assert!(
            rabuka_engine::ability::util::card_matches_group_str(&game.db, id, Some(group)),
            "precondition: {who} must match group {group}"
        );
        assert!(
            !rabuka_engine::ability::util::card_matches_group_str(
                &game.db,
                id,
                Some(if group == "BiBi" { "lilywhite" } else { "BiBi" })
            ),
            "precondition: {who} must NOT match the other unit, or the two \
             conditions are not separable and this file proves nothing about which \
             group is checked"
        );
    }

    game.state.player1.stage.stage = [miki, -1, -1];
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, miki, HeartColor::Heart06),
        0,
        "an empty success zone grants nothing"
    );

    // A card of the right TYPE but the wrong unit.
    let lilywhite_live = game.id(LILYWHITE_LIVE);
    game.state
        .player1
        .success_live_card_zone
        .add_card(lilywhite_live);
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, miki, HeartColor::Heart06),
        0,
        "a 『lilywhite』 ライブカード does not satisfy 『『BiBi』のカードがあるかぎり』 — \
         the unit filter is half the condition"
    );

    game.state.player1.success_live_card_zone.add_card(bibi_live);
    game.state.recalculate_constants();
    assert_eq!(
        heart_mod(&game, miki, HeartColor::Heart06),
        1,
        "adding the 『BiBi』 ライブカード satisfies the condition and grants heart06"
    );
    assert_eq!(
        heart_mod(&game, miki, HeartColor::Heart01),
        0,
        "and only heart06: 真姫's print does not grant heart01, which is 凛's"
    );
}

// ====================================================================
// 矢澤にこ — BOTH side slots' printed blade counts
// ====================================================================

/// 「自分のステージの右サイドエリアと左サイドエリアに、元々持つブレードの数が2つの
/// メンバーがいるかぎり、ライブの合計スコアを＋１する」.
///
/// THE NEGATIVE for the `==` reading, which is the one clause the positive test
/// cannot reach.
///
/// `blade_limit_operator: "=="` with `blade_limit: 2` means 「元々持つブレードの数が2つ」
/// is an EXACT figure, not 「2つ以上」. A member printing 3 blades therefore does
/// NOT qualify — and that is exactly the case an `>=` implementation would pass,
/// so it needs its own test rather than a sentence inside the positive one.
#[test]
fn niko_alt_no_score_when_a_side_slot_member_does_not_print_exactly_two_blades() {
    let mut game = TestGame::new(load_real_database());
    let niko = game.id(NIKO_ALT);
    let exact = game.id("PL!-bp3-014-PR"); // 星空 凛 — exactly 2 blades
    let three = game.id("PL!-sd1-004-SD"); // 園田海未 — 3 blades
    let filler = game.id(FILLER);
    game.assert_card_identity(niko, NIKO_ALT);
    fill_decks(&mut game, filler);
    assert_eq!(
        game.state.card_database.get_card(exact).unwrap().blade,
        2,
        "precondition: the qualifying member prints exactly 2"
    );
    assert_eq!(
        game.state.card_database.get_card(three).unwrap().blade,
        3,
        "precondition: the non-qualifying member prints 3 — one blade MORE, so a \
         `>=` reading would accept it and this test is the only thing that can tell \
         the two readings apart"
    );

    game.state.player1.stage.stage = [exact, niko, three];
    game.state.recalculate_constants();
    assert_eq!(
        total_score_bonus(&game),
        0,
        "a side-slot member printing 3 blades does NOT satisfy 『元々持つブレードの数が\
         2つ』 — the operator is an equality, so more blades is not better"
    );

    // And swapping the 3-blade member out for the 2-blade one flips it, which is
    // what makes this a condition test rather than a permanently-false one.
    game.state.player1.stage.stage[2] = game.id("PL!-sd1-006-SD"); // exactly 2
    game.state.recalculate_constants();
    assert_eq!(
        total_score_bonus(&game),
        1,
        "replacing it with a 2-blade member grants the +1, so the zero above was the \
         blade figure and not some unrelated reason the grant never applies"
    );
}

/// TWO slots, and 「元々持つ」 — the PRINTED blade count, not a blade count modified
/// by anything. Both are separately breakable: a condition reading only one slot, or
/// reading the current blade total rather than the printed one.
#[test]
fn niko_alt_grants_score_only_while_both_side_slots_hold_two_blade_members() {
    let mut game = TestGame::new(load_real_database());
    let niko = game.id(NIKO_ALT);
    let filler = game.id(FILLER);
    game.assert_card_identity(niko, NIKO_ALT);
    fill_decks(&mut game, filler);

    // Two DIFFERENT members that each print EXACTLY 2 blades. The extracted
    // condition carries `blade_limit: 2, blade_limit_operator: "=="` — 「数が2つ」 is
    // an exact count, not 「2つ以上」, so a 3-blade member FAILS it. An `>=` reading
    // would pass this test only with a 3-blade fixture, which is why the premise
    // below asserts the exact figure rather than a floor.
    let left = game.id("PL!-bp3-014-PR"); // 星空 凛 — prints exactly 2
    let right = game.id("PL!-sd1-006-SD"); // 西木野真姫 — prints exactly 2
    for member in [left, right] {
        let printed = game.state.card_database.get_card(member).unwrap().blade;
        assert_eq!(
            printed, 2,
            "precondition: each side-slot member must print EXACTLY 2 blades — \
             『元々持つブレードの数が2つ』 is an equality, and a member with 3 blades \
             does not qualify"
        );
    }

    // She is a センター ability, so she occupies the middle slot.
    game.state.player1.stage.stage = [left, niko, right];
    game.state.recalculate_constants();
    assert_eq!(
        total_score_bonus(&game),
        1,
        "both side slots hold 2-blade members, so the +1 applies"
    );

    // One slot empties: the 「両側」 condition fails and the grant must go.
    game.state.player1.stage.stage[0] = -1;
    game.state.on_cards_left_zones(&[left]);
    game.state.recalculate_constants();
    assert_eq!(
        total_score_bonus(&game),
        0,
        "one side slot empty is not 『右サイドエリアと左サイドエリアに…いるかぎり』 — \
         the condition is over BOTH slots, so the +1 is withdrawn"
    );
}

// ====================================================================
// A song for You! You? You!! — a per-card SCALING ライブ開始時
// ====================================================================

/// 「自分の成功ライブカード置き場にあるカード1枚につき、このカードのスコアを＋２し、
/// 必要ハートをheart01/heart03/heart06/heart0増やす」.
///
/// This is the strongest available claim in the deck: a PER-CARD multiplier with
/// two simultaneous effects. Every wrong answer is distinguishable — a flat +2, a
/// count of the wrong zone, one clause applied without the other — and a single
/// positive assertion catches none of them.
#[test]
fn song_boost_scales_with_the_number_of_cards_in_the_success_zone() {
    let mut game = TestGame::new(load_real_database());
    let song = game.id(SONG);
    let filler = game.id(FILLER);
    game.assert_card_identity(song, SONG);

    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(song);

    // Zero cards: no boost at all.
    fire_trigger(&mut game, song, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        score_mod(&game, song),
        0,
        "an EMPTY success zone grants nothing — カード1枚につき has no card to count"
    );
    assert_eq!(
        need_heart_mod(&game, song, HeartColor::Heart00),
        0,
        "and no requirement relaxation either"
    );

    // Rebuild from scratch per count so the readings cannot accumulate.
    for (cards, want_score) in [(1usize, 2), (2, 4), (3, 6)] {
        let mut g = TestGame::new(load_real_database());
        let song = g.id(SONG);
        let filler = g.id(FILLER);
        fill_decks(&mut g, filler);
        g.state.player1.live_card_zone.cards.push(song);
        for _ in 0..cards {
            g.state
                .player1
                .success_live_card_zone
                .add_card(g.new_id(COUNT_CARD));
        }
        fire_trigger(&mut g, song, AbilityTrigger::LiveStart, "ライブ開始時");

        assert_eq!(
            score_mod(&g, song),
            want_score,
            "{cards} card(s) in the success zone → +2 score each = {want_score}; a flat \
             +2 or an off-by-one in the count lands here"
        );
        for colour in [
            HeartColor::Heart00,
            HeartColor::Heart01,
            HeartColor::Heart03,
            HeartColor::Heart06,
        ] {
            assert_eq!(
                need_heart_mod(&g, song, colour),
                i32::from(cards as u8),
                "{cards} card(s) → {cards} added to each of heart01/heart03/heart06/heart0; \
                 the second clause scales with the first"
            );
        }
    }
}

/// OPONENT SIDE: 「自分の」 is the only qualifier, so p2's success zone must not
/// count toward p1's boost. Reading the wrong player's zone is the easiest way to
/// get a scaling multiplier wrong and the only way this can catch it.
#[test]
fn song_boost_does_not_count_opponent_success_zone_cards() {
    let mut game = TestGame::new(load_real_database());
    let song = game.id(SONG);
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.state.player1.live_card_zone.cards.push(song);

    for _ in 0..3 {
        game.state
            .player2
            .success_live_card_zone
            .add_card(game.new_id(COUNT_CARD));
    }
    fire_trigger(&mut game, song, AbilityTrigger::LiveStart, "ライブ開始時");

    assert_eq!(
        score_mod(&game, song),
        0,
        "p2's success zone is not 『自分の』 — three cards there must not give +6"
    );
    assert_eq!(
        need_heart_mod(&game, song, HeartColor::Heart00),
        0,
        "and no requirement relaxation from the opponent's cards either"
    );
}

// ====================================================================
// ?←HEARTBEAT — TWO thresholds on one quantity
// ====================================================================

/// 「自分の成功ライブカード置き場にあるカードのスコアの合計が６以上の場合、必要ハートを
/// heart0減らす。スコアの合計が９以上の場合、さらにこのカードのスコアを＋１する」.
///
/// One quantity, two thresholds, three distinguishable regions. The boundaries at
/// exactly 6 and exactly 9 are where an off-by-one lives, and no single reading
/// distinguishes 「6 relaxes but does not raise」 from either neighbour.
#[test]
fn heartbeat_two_thresholds_partition_the_success_zone_score_total() {
    let cheap = "PL!-sd1-001-SD";
    let mid = "PL!SP-bp1-027-L"; // score 6
    let dear = "PL!N-bp1-028-L"; // score 5 — 6 + 5 clears the 9 threshold

    // Region 1: total below 6 — nothing at all.
    let mut below = TestGame::new(load_real_database());
    let song = below.id(HEARTBEAT);
    below.state.player1.live_card_zone.cards.push(song);
    below.state.player1.success_live_card_zone.add_card(below.id(cheap));
    fire_trigger(&mut below, song, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        score_mod(&below, song),
        0,
        "success-zone total below 6 → no score, and the requirement is untouched"
    );
    assert_eq!(
        need_heart_mod(&below, song, HeartColor::Heart00),
        0,
        "below the 6 threshold nothing is relaxed"
    );

    // Region 2: total of exactly 6 — relaxation but NOT the extra score.
    let mut at_six = TestGame::new(load_real_database());
    let song = at_six.id(HEARTBEAT);
    at_six.state.player1.live_card_zone.cards.push(song);
    at_six.state
        .player1
        .success_live_card_zone
        .add_card(at_six.id(mid)); // score 6
    fire_trigger(&mut at_six, song, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        need_heart_mod(&at_six, song, HeartColor::Heart00),
        -1,
        "total of exactly 6 satisfies 『６以上』 and REDUCES heart0 by 1 — 6 is the \
         first value that does. The sign is negative because the card says 減らす: \
         the modifier lowers the requirement, so reading +1 here would be a test \
         that passes only if the effect were applied backwards"
    );
    assert_eq!(
        score_mod(&at_six, song),
        0,
        "but 6 is BELOW 『９以上』, so the additional +1 score must NOT apply — the \
         two thresholds are independent"
    );

    // Region 3: total at or above 9 — both clauses. The total is COMPUTED from the
    // seeded cards rather than assumed: `PL!-sd1-001-SD` carries no printed score
    // at all, so a fixture that counted it toward the total would read 6 and never
    // reach the 『９以上』 branch — a silently wrong fixture that still looks like a
    // failed threshold.
    let mut above = TestGame::new(load_real_database());
    let song = above.id(HEARTBEAT);
    above.state.player1.live_card_zone.cards.push(song);
    for card_no in [mid, dear] {
        above
            .state
            .player1
            .success_live_card_zone
            .add_card(above.id(card_no));
    }
    let total: u32 = above
        .state
        .player1
        .success_live_card_zone
        .cards
        .iter()
        .map(|c| {
            above
                .state
                .card_database
                .get_card(*c)
                .unwrap()
                .score
                .unwrap_or(0) as u32
        })
        .sum();
    assert!(
        total >= 9,
        "precondition: this region must actually reach 『９以上』 (computed total \
         {total}) — otherwise it is the same case as the region above and proves \
         nothing about the second threshold"
    );
    fire_trigger(&mut above, song, AbilityTrigger::LiveStart, "ライブ開始時");
    assert_eq!(
        score_mod(&above, song),
        1,
        "a total of 9+ satisfies 『９以上』 and adds +1 score"
    );
    assert_eq!(
        need_heart_mod(&above, song, HeartColor::Heart00),
        -1,
        "and the 6-clause reduction still applies — the thresholds stack rather \
         than being exclusive"
    );
}
