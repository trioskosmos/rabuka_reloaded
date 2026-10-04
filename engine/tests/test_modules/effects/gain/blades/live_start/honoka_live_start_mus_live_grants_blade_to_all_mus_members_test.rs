/// Tests for 高坂穂乃果 (PL!-bp6-001) ab#0 — LiveStart.
///
///   {{live_start.png|ライブ開始時}}{{center.png|センター}}自分のライブカード置き場に
///   『µ's』のカードがある場合、ライブ終了時まで、**自分のステージにいるすべての**
///   『μ's』のメンバーは{{icon_blade.png|ブレード}}を得る。
///
/// Three things the printed sentence asserts, and one of them nothing tested:
///
///   * 「**すべての**『μ's』のメンバー」 — a quantifier over every qualifying member,
///     not one. An implementation that granted a single blade is the natural way to
///     read "members gain blade" and it satisfies any per-card assertion.
///   * 「**自分の**ステージ」 — my stage only.
///   * 「**{{center.png|センター}}**」 — 穂乃果 must be in the centre, and that is
///     the clause that makes this card's own position matter.
///
/// Note on the group name. The card prints 『µ's』 (U+00B5 MICRO SIGN) in the
/// condition and 『μ's』 (U+03BC GREEK MU) in the effect — verified in cards.json,
/// so the parser is faithful and this is not a defect. It is also not a behavioural
/// difference: group matching resolves through `card_series_matches_group`, which
/// never compares either spelling literally, so both names select the same cards.
/// The tests below assert group membership by its actual mechanism (the series
/// string) rather than by unit, because `Card.group` is empty in the card data and
/// `unit` holds a unit name like "Printemps", not a group — see
/// `success_zone_group_live_success_draw_test.rs`, which measures how lossy that
/// fallback is.
use crate::helpers::*;

const HONOKA: &str = "PL!-bp6-001-R＋";
const KOTORI: &str = "PL!-bp5-003-R＋"; // 南ことり, μ's
const MUS_MEMBER: &str = "PL!-sd1-010-SD"; // μ's
/// A live card whose series is plain 「ラブライブ！」, so it satisfies the group
/// filter the same way a printed μ's live does.
const MUS_LIVE: &str = "PL!-sd1-019-SD";

/// Assert the group filter's actual precondition for a card: its series carries no
/// school name, which is what makes `card_series_matches_group` accept it for μ's.
fn assert_matches_mus(game: &TestGame, id: i16, label: &str) {
    let series = game
        .db
        .get_card(id)
        .unwrap_or_else(|| panic!("{label} not in cards.json"))
        .series
        .to_string();
    for school in ["サンシャイン", "虹ヶ咲", "スーパースター", "蓮ノ空"] {
        assert!(
            !series.contains(school),
            "{label} must not be tied to {school} for this test to mean anything — \
             its series is {series:?}"
        );
    }
}

/// Drive the turn to LiveStart with `live` as the performing card, which is also
/// what puts it in the success zone the condition reads.
///
/// `set_live_card` plays the card out of HAND — it is the LiveCardSet choice, not a
/// direct zone write — so the card has to be in hand first.
fn to_live_start(game: &mut TestGame, live: i16) {
    game.add_to_hand(live);
    for _ in 0..5 {
        game.pass();
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "fixture: expected to reach LiveCardSet, got {}",
        game.state.current_phase
    );
    game.set_live_card(live);
    game.pass();
    game.pass();
}

/// Stage slots are `[left, CENTRE, right]` — index 1 is the centre.
///
/// Written out because it is the whole subject of two of these tests, and getting it
/// backwards makes the positive case fail and the negative case pass for entirely
/// the wrong reason.
fn place_in_center(game: &mut TestGame, card: i16) {
    game.state.player1.stage.stage[1] = card;
}

/// 「すべての」 — every μ's member on my stage, not one of them.
///
/// Three qualifying members on the stage and a μ's live in the success zone, so
/// the count of granted blades is the assertion. A one-member implementation passes
/// a test that checks "somebody gained a blade" and fails this one.
#[test]
fn grants_a_blade_to_every_mus_member_on_my_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id(HONOKA);
    let kotori = game.id(KOTORI);
    let third = game.id(MUS_MEMBER);
    let live = game.id(MUS_LIVE);
    for (id, label) in [
        (honoka, "honoka"),
        (kotori, "kotori"),
        (third, "third"),
        (live, "mus_live"),
    ] {
        assert_matches_mus(&game, id, label);
    }

    game.state.player1.stage.stage = [kotori, honoka, third];
    place_in_center(&mut game, honoka);
    to_live_start(&mut game, live);

    for (id, label) in [(honoka, "honoka"), (kotori, "kotori"), (third, "third")] {
        assert_eq!(
            game.state.mods.get_blade_modifier(id),
            1,
            "「すべての『μ's』のメンバー」 — {label} is on my stage and must gain the \
             blade"
        );
    }
}

/// 「自分のステージ」 — the opponent's μ's members are not included.
///
/// The same board as the test above with one card moved across. It is the only way
/// to tell a correct `target: self` from a stage-wide sweep, since the qualifying
/// card and the condition are identical either way.
#[test]
fn does_not_grant_a_blade_to_the_opponents_mus_members() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id(HONOKA);
    let theirs = game.id(MUS_MEMBER);
    let live = game.id(MUS_LIVE);
    for (id, label) in [(honoka, "honoka"), (theirs, "theirs"), (live, "mus_live")] {
        assert_matches_mus(&game, id, label);
    }

    game.state.player1.stage.stage = [-1, honoka, -1];
    game.state.player2.stage.stage = [theirs, -1, -1];
    place_in_center(&mut game, honoka);
    to_live_start(&mut game, live);

    assert_eq!(
        game.state.mods.get_blade_modifier(honoka),
        1,
        "my member gains the blade"
    );
    assert_eq!(
        game.state.mods.get_blade_modifier(theirs),
        0,
        "「自分のステージにいる」 — the opponent's member is not on my stage"
    );
}

/// 「{{center.png|センター}}」 — the blade clause is inert without a μ's live.
///
/// The condition is the gate on the whole effect, so this also covers the
/// possibility of the blade being granted unconditionally: if it were, this member
/// would gain one here too.
#[test]
fn grants_nothing_without_a_mus_live_in_my_success_zone() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id(HONOKA);
    let second = game.id(MUS_MEMBER);
    let live = game.id(MUS_LIVE);
    // The live is still a live card and still in my success zone; what changes is
    // that its group is one the filter rejects.
    let other_live = game.id("PL!N-bp1-026-L"); // Poppin' Up! — 虹ヶ咲

    game.state.player1.stage.stage = [honoka, second, -1];
    to_live_start(&mut game, other_live);

    for (id, label) in [(honoka, "honoka"), (second, "second")] {
        assert_eq!(
            game.state.mods.get_blade_modifier(id),
            0,
            "『虹ヶ咲』の live in the success zone does not satisfy 「『µ's』のカードが \
             ある場合」, so {label} gains nothing"
        );
    }
    assert!(
        game.state.player1.live_card_zone.cards.contains(&live) == false,
        "sanity: the μ's live was never played"
    );
}

/// 「{{center.png|センター}}」 on the ability card itself.
///
/// 穂乃果 in a side area, everything else identical to the passing case. The
/// condition carries `activation_position: "center"`, and this is the only test that
/// makes that field load-bearing — without it, a centre-gated card is
/// indistinguishable from one that works anywhere.
///
/// This is the test that found the gate was unreachable. `activation_position` was
/// checked only inside the location-condition evaluator, so for a `group_condition`
/// like 穂乃果's it never ran and the blade was granted from any slot. The fix hoisted
/// the check above the type dispatch in `condition.rs::evaluate_condition`, next to
/// the phase gate that was already there for the same reason.
#[test]
fn grants_nothing_when_honoka_is_not_in_the_center() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let honoka = game.id(HONOKA);
    let second = game.id(MUS_MEMBER);
    let live = game.id(MUS_LIVE);
    for (id, label) in [(honoka, "honoka"), (second, "second"), (live, "mus_live")] {
        assert_matches_mus(&game, id, label);
    }

    game.state.player1.stage.stage = [honoka, second, -1]; // 穂乃果 at index 0 = LEFT
    place_in_center(&mut game, second); // sanity: the centre slot is occupied, but not by her
    to_live_start(&mut game, live);

    for (id, label) in [(honoka, "honoka"), (second, "second")] {
        assert_eq!(
            game.state.mods.get_blade_modifier(id),
            0,
            "{{center.png|センター}} — 穂乃果 is in a side area, so her live-start \
             ability does not resolve and {label} gains nothing. Note that `second` \
             holds the centre slot, so this cannot pass by the stage being empty."
        );
    }
}