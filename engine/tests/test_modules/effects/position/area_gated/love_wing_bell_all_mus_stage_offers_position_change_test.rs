/// `PL!-bp4-020-L` / `-SRL` **Love wing bell** ab#0 — ライブ開始時.
///
///   自分のステージにいるメンバーが『μ's』のみの場合、
///   自分のステージにいるメンバー1人をポジションチェージさせてもよい。
///
/// ab#0 had no test. `PL!-bp4-020` appears in the suite only as ab#1's subject
/// (`success_zone_center_area_mus_member_constant_blade_test.rs`) and as a
/// required-hearts fixture elsewhere.
///
/// # What is under test, and what deliberately is not
///
/// The gate: `all_members: true` over 『μ's』, meaning EVERY member on my stage must
/// be μ's before the optional position change is offered. Both directions are pinned
/// — an all-μ's board offers, and one outsider withholds. A gate read as 「at least
/// one μ's」, or dropped, passes every existing test on this card.
///
/// The swap itself is NOT asserted here. Its mechanics are already covered from the
/// opponent side by `himeno_debut_opponent_member_moved_to_own_front_area_test.rs`, and
/// this ability's distinguishing feature is the SIDE — `target: "self"` with
/// `source: "stage"`, so it rearranges my own board rather than the opponent's. That is
/// what a test on this card has to say, and guessing at a prompt shape to also re-assert
/// the swap would add brittleness without adding coverage.
///
/// # Why the fixtures are chosen the way they are
///
/// μ's membership is the interesting part. Every card below has `"unit": "Printemps"`
/// — the sub-unit name — and `"series": "ラブライブ！"` with no school name in it, so
/// none of them matches a literal `unit == "μ's"`. They are recognised as μ's only
/// through the series fallback in `card_matches_group_str`. That is deliberate: a
/// fixture whose `unit` literally read "μ's" would let a group-blind gate pass, which
/// is the failure this test exists to catch.
///
/// The group is asserted from the database in every test rather than trusted from the
/// comments above. `pb2_discard_three_distinct_bibi_waits_foe_pl_pb2_018_p_plus_test.rs`
/// records what a mis-annotated card costs: `PL!-pb2-003-PP` was annotated 矢澤にこ but is
/// 南ことり, and the resulting confident, wrong diagnosis took a multi-paragraph
/// correction to write down.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const LOVE_WING_BELL: &str = "PL!-bp4-020-L";
/// Three μ's members with three different names. Verified in cards.json:
/// `type: メンバー`, `unit: "Printemps"`, `series: "ラブライブ！"`.
const MUS_ONE: &str = "PL!-PR-001-PR"; // 高坂 穂乃果
const MUS_TWO: &str = "PL!-PR-003-PR"; // 南 ことり
const MUS_THREE: &str = "PL!-PR-008-PR"; // 小泉 花陽
/// DOLLCHESTRA — and its series names 蓮ノ空, which is one of the four school names
/// the series fallback rejects. So it is outside μ's on both readings.
const NOT_MUS: &str = "PL!HS-bp5-005-AR"; // 徒町 小鈴

/// Fire ab#0 with `members` on my stage and report whether the optional position
/// change was offered.
///
/// Deliberately does NOT assert that every staged member is μ's: one of the tests
/// deliberately stages an outsider. Each test states the membership it relies on.
fn offers_position_change(members: &[&str]) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bell = game.id(LOVE_WING_BELL);

    for (slot, card_no) in members.iter().enumerate() {
        game.state.player1.stage.stage[slot] = game.id(card_no);
    }
    game.give_energy(15);
    fire_trigger(&mut game, bell, AbilityTrigger::LiveStart, "ライブ開始時");
    game.has_pending_choice()
}

/// Assert a card's group membership from the database, so the test states the
/// property it depends on instead of trusting the constant's name.
fn assert_group(game: &TestGame, card_no: &str, group: &str, expected: bool) {
    let id = game.id(card_no);
    assert_eq!(
        rabuka_engine::ability::util::card_matches_group_str(&game.db, id, Some(group)),
        expected,
        "fixture: {card_no} is {expected} expected to be {group}",
    );
}

/// All three areas are μ's → the optional position change is offered.
///
/// The cards carry `unit: "Printemps"`, so this also demonstrates that the μ's gate
/// resolves through the series fallback rather than an exact `unit` match.
#[test]
fn love_wing_bell_offers_the_change_when_every_member_is_mus() {
    {
        let game = TestGame::new(load_real_database());
        for c in [MUS_ONE, MUS_TWO, MUS_THREE] {
            assert_group(&game, c, "μ's", true);
        }
    }
    assert!(
        offers_position_change(&[MUS_ONE, MUS_TWO, MUS_THREE]),
        "「自分のステージにいるメンバーが『μ's』のみの場合」 — three μ's members, so the \
         optional position change is offered"
    );
}

/// One non-μ's member withholds the offer.
///
/// The failing direction of `all_members`. Every other test on this card stages a
/// board that satisfies it, so a gate read as 「at least one μ's」 — or ignored —
/// would pass all of them, and the ability would fire on a board the text excludes.
#[test]
fn love_wing_bell_is_silent_when_one_member_is_not_mus() {
    {
        let game = TestGame::new(load_real_database());
        assert_group(&game, NOT_MUS, "μ's", false);
        assert_group(&game, MUS_ONE, "μ's", true);
    }
    assert!(
        !offers_position_change(&[MUS_ONE, NOT_MUS, MUS_THREE]),
        "『μ's』のみの場合 — a DOLLCHESTRA member on my stage means no offer"
    );
}

/// An empty area does NOT break the gate: 「のみ」 is about members, not occupancy.
///
/// This is the counterweight to the test above, and it was the assertion I got wrong
/// first — I wrote the message saying the offer should appear and the assertion saying
/// the opposite. The engine is right and the test was not: a stage holding two μ's
/// members contains no member that fails 『μ's』のみ』, so the change IS offered.
///
/// It matters because the neighbouring 彼方 and 姫芽 abilities use `all_areas: true` as
/// well as `all_members`, and a reader could reasonably assume one card's `のみ` means
/// the same as another's. Here it does not, and this is what pins the difference.
#[test]
fn love_wing_bell_offers_the_change_with_an_empty_area() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let bell = game.id(LOVE_WING_BELL);
    let one = game.id(MUS_ONE);
    let two = game.id(MUS_TWO);
    game.state.player1.stage.stage[0] = one;
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = two;

    assert_group(&game, MUS_ONE, "μ's", true);

    game.give_energy(15);
    fire_trigger(&mut game, bell, AbilityTrigger::LiveStart, "ライブ開始時");

    assert!(
        game.has_pending_choice(),
        "two μ's members and an empty centre: the board holds no member that breaks \
         『μ's』のみ, so the optional position change is still offered. This ability \
         gates on all_members alone — unlike 乙宗 梢 and 安養寺 姫芽, whose のみ also \
         requires every area occupied."
    );
}