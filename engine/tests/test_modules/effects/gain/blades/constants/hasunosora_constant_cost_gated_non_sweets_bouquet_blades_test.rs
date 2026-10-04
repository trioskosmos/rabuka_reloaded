/// Tests for 百生 吟子 (PL!HS-bp5-004) ab#0 — 常時.
///
///   {{jyouji.png|常時}}自分のステージにいるコスト4以上の『スリーズブーケ』以外の
///   メンバー1人につき、{{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
///
/// Nothing tested this ability. `PL!HS-bp5-004-*` appears in seven files, always as a
/// convenient "cost 15" member borrowed by someone else's test — never as the subject.
/// So a per-unit blade clause with a cost bound, a group EXCLUSION and a self-reference
/// had no coverage at all.
///
/// Three things make it worth writing, in increasing order of how easily they go wrong:
///
///   * 「**コスト4以上**」 — a bound, with 桜内梨子 at exactly 4 and 西木野真姫 at 2.
///   * 「『スリーズブーケ』**以外**」 — 乙宗梢 is cost 4 as well, so she and 桜内梨子
///     differ ONLY by unit. That pair is what separates "honours the exclusion" from
///     "counts every member over the cost".
///   * **百生 吟子 is herself a 『スリーズブケット』 member** (`PL!HS-PR-007-PR`,
///     `"unit": "スリーズブokeh"`). Her own ability excludes her from her own count, at
///     cost 15 — comfortably over the bound. An implementation that forgot to exclude
///     would grant her blades for her own presence, every turn, forever.
///
/// The parse carries a hazard worth naming: the same effect object holds BOTH
/// `exclude_group_names: ["スリーズブーケ"]` and `group_names: ["スリーズブーケ"]`
/// (`abilities.json:8144-8153`). The second is extractor hoisting from the clause it
/// excludes. Read literally the object demands a card be スリーズブケット AND not
/// スリーズブケット — nothing qualifies, and the ability never fires. Test 1 is
/// therefore also the check that the hoisted field is inert.
use crate::helpers::*;

const ME: &str = "PL!HS-bp5-004-R"; // 百生 吟子, cost 15
/// 桜内梨子 — cost exactly 4, GuiltyKiss. Qualifies on both counts.
const AT_BOUND: &str = "PL!S-bp2-002-R";
/// 西木野真姫 — cost 2, BiBi. Qualifies by group but is under the bound.
const UNDER_BOUND: &str = "PL!-pb2-006-PP";
/// 乙宗梢 — cost exactly 4 AND スリーズブケット. Same cost as AT_BOUND, opposite group.
const SWEETS_AT_BOUND: &str = "PL!HS-PR-003-PR";
/// 百生 吟子 as a PR print — cost 9, スリーズブケット. The same character as ME.
const ME_AS_PR: &str = "PL!HS-PR-007-PR";

/// Stage 吟子 alongside the given members and report the blades she gained.
///
/// 吟子 is placed AFTER the members, in the next free slot. Placing her first at slot 1
/// and then writing members from slot 0 let the loop overwrite her — and a stage with no
/// 吟子 on it has no ability to fire, which looks exactly like "the ability grants
/// nothing". The stage has three slots, so at most two members fit beside her.
fn blades_with(stage_members: &[&str]) -> i16 {
    assert!(
        stage_members.len() <= 2,
        "the stage has 3 slots and one is 吟子's, so at most 2 members fit beside her"
    );
    let db = load_real_database();
    let mut game = TestGame::new(db);
    for (i, card_no) in stage_members.iter().enumerate() {
        game.state.player1.stage.stage[i] = game.id(card_no);
    }
    let me = game.id(ME);
    game.state.player1.stage.stage[stage_members.len()] = me;
    game.state.recalculate_constants();
    game.state.mods.get_blade_modifier(me)
}

#[test]
fn a_cost_4_member_that_is_not_sweets_bouquet_gives_two_blades() {
    assert_eq!(
        blades_with(&[AT_BOUND]),
        2,
        "コスト4以上 — 桜内梨子 is exactly 4 and not 『スリーズブケット』, so she counts \
         once at +2. Also proves the hoisted `group_names` in the parse is inert: read \
         literally the clause would demand スリーズブケット AND not スリーズブケット and \
         nothing would ever qualify."
    );
}

#[test]
fn a_cost_2_member_is_under_the_bound() {
    assert_eq!(
        blades_with(&[UNDER_BOUND]),
        0,
        "西木野真姫 is BiBi, so the group filter admits her — but she is cost 2, one \
         step under 「コスト4以上」. This is the nearest miss, and it is what separates \
         `>= 4` from `>= 2`."
    );
}

#[test]
fn a_sweets_bouquet_member_at_the_cost_bound_is_excluded() {
    assert_eq!(
        blades_with(&[SWEETS_AT_BOUND]),
        0,
        "乙宗梢 costs exactly 4 — the same as 桜内梨子 — and is the only difference \
         between the two, so the count here is decided entirely by 『スリーズブケット以外』. \
         An implementation that applied only the cost bound would grant 2 here."
    );
}

#[test]
fn hasunosora_does_not_count_herself() {
    assert_eq!(
        blades_with(&[ME_AS_PR]),
        0,
        "「自分のステージにいるコスト4以上の『スリーズブーケ』以外のメンバー」 — \
         百生 吟子 is herself a 『スリーズブケット』 member at cost 15, so she is \
         excluded from her own count. Granting here would mean she pays herself blades \
         for being on the board, permanently."
    );
}

#[test]
fn one_qualifying_member_of_two_gives_two_blades() {
    assert_eq!(
        blades_with(&[AT_BOUND, UNDER_BOUND]),
        2,
        "1人につき — two members beside 吟子, of which ONE qualifies: 桜内梨子 is cost 4 \
         and not 『スリーズブケット』; 西木野真姫 is BiBi but cost 2, under the bound. So the \
         count is 1 unit at +2, not 2 units at +4 — the bound is evaluated per member, not \
         against the stage."
    );
}

#[test]
fn two_qualifying_members_give_four_blades() {
    assert_eq!(
        blades_with(&[AT_BOUND, AT_BOUND]),
        4,
        "1人につき is per qualifying MEMBER: two of them are two units at +2 each. Two \
         separate instances of 桜内梨子, so this counts cards rather than distinct names \
         — the clause carries no `distinct`, so it must not dedupe."
    );
}

/// The fixtures are only meaningful at the units and costs they stand for, and the two
/// discriminating pairs each differ on exactly one field.
///
/// Without this, a card-data edit could quietly make the "excluded" and the "qualifying"
/// fixtures identical — at which point every test above still passes and the file asserts
/// nothing. `assert_same_cost_different_unit` is the guard for the pair that matters.
#[test]
fn the_fixtures_straddle_the_bound_and_the_group() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let card = |c: &str| {
        game.db
            .get_card(game.id(c))
            .unwrap_or_else(|| panic!("{c} not in cards.json"))
    };

    for c in [AT_BOUND, UNDER_BOUND, SWEETS_AT_BOUND, ME_AS_PR] {
        assert_eq!(
            card(c).card_type,
            rabuka_engine::card::CardType::Member,
            "{c} must be a メンバー — the clause counts stage members"
        );
    }

    // The pair that decides the group test: same cost, opposite unit.
    assert_eq!(
        card(AT_BOUND).cost,
        card(SWEETS_AT_BOUND).cost,
        "桜内梨子 and 乙宗梢 must cost the SAME amount, or that test is deciding on \
         cost rather than on 『スリーズブケット以外』"
    );
    assert_eq!(card(AT_BOUND).cost, Some(4), "both are cost exactly 4");
    assert_eq!(
        card(SWEETS_AT_BOUND)
            .unit
            .as_deref()
            .map(|s| s.to_string())
            .as_deref(),
        Some("スリーズブーケ"),
        "乙宗梢 must be 『スリーズブケット』 — she is the excluded half of the pair"
    );
    assert_ne!(
        card(AT_BOUND).unit.as_deref(),
        card(SWEETS_AT_BOUND).unit.as_deref(),
        "and the qualifying half must NOT be スリーズブケット"
    );

    // The bound itself, on both sides of it.
    assert_eq!(card(UNDER_BOUND).cost, Some(2), "one step under the bound");
    assert!(
        card(ME).cost.unwrap_or(0) >= 4,
        "吟子 herself must clear the cost bound, or the self-exclusion test is decided \
         by cost rather than by group"
    );
}