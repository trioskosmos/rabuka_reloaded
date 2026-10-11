//! PSYCHIC FIRE `PL!-pb2-042-L` (自動, ターン1回)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   自動／ターン1回 — 自分がエールしたとき、エールにより公開された自分のカードの中に
//!   「矢澤にこ」と「西木野真姫」と「絢瀬絵里」のメンバーカードがあり、**かつ**
//!   自分のセンターエリアにコスト11以上の『BiBi』のメンバーがいる場合、相手のステージに
//!   いる元々持つハートの数が4つ以下のメンバー1人をウェイトにする。
//!
//! **The four negative tests below are LIVE evidence, not vacuous.** This header used
//! to say the opposite — that they asserted "no wait" and passed only because the
//! positive did not fire, with an instruction to "treat the fixture pins as the real
//! content until `all_clauses_met_waits_the_foe` passes". That warning went stale in
//! the dangerous direction the moment the positive was fixed (see its doc comment:
//! CLOSED 2026-10-02), and it had exactly the effect
//! `characterization/strict_engine_bug_expected_fail_test.rs` warns about in its own
//! header: a reader told these tests prove nothing will dismiss a real regression as
//! "the known vacuous one" instead of reading the failure. The positive fires
//! (`foe waited = true`), so each negative below is the positive with exactly one
//! clause broken — which is what makes it evidence that THAT clause is load-bearing.
//!
//! What is NOT a decode problem: `bytecode_no_silent_decode_fallbacks` reports no
//! fallback for this ability, so all four clauses reach the engine intact.
//!
//! The four clauses, and the two the parser had been dropping (docs §8.3 item 15):
//!   * 「「矢澤にこ」と「西木野真姫」と「絢瀬絵里」」 — the と-joined name list. The
//!     parser's name-list regex did not allow `」と「`, so it matched only the LAST
//!     entry and this clause was really "絢瀬絵里 was revealed". FIXED.
//!   * 「自分のセンターエリアに」 — the centre clause was inheriting
//!     `location: revealed_cards` from the clause before it, so it looked for an
//!     11-cost BiBi member among the cards the yell revealed rather than on the
//!     stage. FIXED.
//!
//! The remaining clauses are the centre cost bound (11以上) and the target's printed
//! heart bound (4以下), the latter the same mechanism as `PL!-pb2-006` (≤1) and
//! `PL!-pb2-033` (≤3).

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const PSYCHIC: &str = "PL!-pb2-042-L";
/// The three named characters, all BiBi, all revealed.
const NICO: &str = "PL!-PR-017-PR"; // 矢澤にこ, cost 2
const MAKI: &str = "PL!-pb2-006-PP"; // 西木野真姫, cost 2
const ERI: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, cost 4
/// A 17-cost BiBi member — clears コスト11以上 in the CENTRE.
const CENTRE_DEAR: &str = "PL!-PR-015-PR"; // 西木野真姫, cost 17
/// A 2-cost BiBi member — under the コスト11以上 bound.
///
/// Read together with `open_gap_bibi_group_matching_is_by_unit_string_not_by_game_group`,
/// this case is pinned correctly; the bound itself is not tightly pinned, see the note
/// on that test.
const CENTRE_CHEAP: &str = "PL!-pb2-006-PP"; // cost 2
/// An 11+-cost member that is NOT BiBi — clears the cost bound, fails the group.
const CENTRE_NON_BIBI: &str = "PL!-pb1-026-N";
/// Opponent members by printed heart total.
const OPP_2: &str = "PL!-bp6-017-N"; // 2 printed hearts
const OPP_5: &str = "PL!N-bp4-019-N"; // 5 printed hearts — over the bound
const FILLER: &str = "PL!-sd1-010-SD";

/// Stage `centre` in the CENTRE, reveal `revealed`, put `foe` on the opponent's
/// stage, fire the 自動, and report whether the foe was waited.
fn run(centre: &str, revealed: &[&str], foe: &str) -> bool {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let foe_id = game.id(foe);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;
    let centre_id = game.id(centre);
    game.state.player1.stage.stage[MemberArea::Center as usize] = centre_id;
    for c in revealed {
        game.state.revealed_cards.push(game.id(c));
    }
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = -1;

    let psychic = game.id(PSYCHIC);
    game.assert_card_identity(psychic, PSYCHIC);
    game.state.player1.live_card_zone.cards.push(psychic);

    let ability_id = {
        let card = game.db.get_card(psychic).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"))
            .unwrap_or_else(|| panic!("{PSYCHIC} lacks a 自動 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(psychic).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(psychic);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::Auto,
        pid.clone(),
        Some(card_no),
        Some(psychic),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    game.state.mods.get_orientation_modifier(foe_id) == Some("wait")
}

/// Fixtures the clauses rest on, pinned so a data change cannot make them vacuous.
#[test]
fn fixtures_are_what_the_clauses_assume() {
    let db = load_real_database();
    let game = TestGame::new(db);

    for card_no in [NICO, MAKI, ERI] {
        let id = game.id(card_no);
        game.assert_card_in_group(id, "BiBi", "one of the three named characters");
    }
    assert!(
        game.db.get_card(game.id(CENTRE_DEAR)).and_then(|c| c.cost).unwrap_or(0) >= 11,
        "the centre fixture must clear コスト11以上"
    );
    assert!(
        game.db.get_card(game.id(CENTRE_CHEAP)).and_then(|c| c.cost).unwrap_or(99) < 11,
        "the centre control must be under コスト11以上"
    );
    for (card_no, want) in [(OPP_2, 2u8), (OPP_5, 5)] {
        let total = game
            .db
            .get_card(game.id(card_no))
            .and_then(|c| c.base_heart.as_ref().map(|b| b.hearts.values_sum()))
            .unwrap_or(0);
        assert_eq!(
            total, want,
            "{card_no} must print {want} hearts — the 4つ以下 bound is the point"
        );
    }
}

/// The positive. **CLOSED 2026-10-02.** This used to be a known non-firing bug, and
/// while it was open the test asserted `!fired` — pinning the broken behaviour with a
/// failure message that said the opposite ("the effect should fire and does not").
///
/// The cause was parser defect D4 (`docs/PARSER_DEFECTS.md`): the effect node
/// inherited `position: "center"` and the three quoted character names from the gating
/// condition, so the action was re-gated on its own condition's terms and the
/// conjunction could never be satisfied from the printed text. With the condition
/// context kept on the condition and off the effect, all four clauses are met and the
/// foe is waited.
///
/// The parse is now: a `location_condition` on `revealed_cards` naming
/// 矢澤にこ/西木野真姫/絢瀬絵里, ANDed with a `comparison_condition` carrying
/// `position: "center"`, `cost_limit: 11` (`>=`), `group_names: ["BiBi"]`, plus a
/// top-level `yell_trigger: true` — the compound the printed text describes.
#[test]
fn all_clauses_met_waits_the_foe() {
    let fired = run(CENTRE_DEAR, &[NICO, MAKI, ERI], OPP_2);
    println!(
        "all four clauses met -> foe waited = {fired} (characters revealed, 17-cost \
         BiBi in centre, foe at 2 printed hearts)"
    );
    assert!(
        fired,
        "OPEN GAP: …「矢澤にこ」と「西木野真姫」と「絢瀬絵里」…かつ センターエリアに\
         コスト11以上の『BiBi』のメンバー…かつ 元々持つハートの数が4つ以下のメンバー1人を\
         ウェイトにする — the effect should fire and does not"
    );
}

/// THE test for the と-joined name list. Dropping any one of the three must stop the
/// effect — with only 絢瀬絵里 required (the pre-fix parse) the first two would not
/// have mattered at all.
#[test]
fn dropping_any_one_of_the_three_named_characters_stops_it() {
    for missing in [NICO, MAKI, ERI] {
        let revealed: Vec<&str> = [NICO, MAKI, ERI]
            .iter()
            .copied()
            .filter(|c| *c != missing)
            .collect();
        assert!(
            !run(CENTRE_DEAR, &revealed, OPP_2),
            "「矢澤にこ」と「西木野真姫」と「絢瀬絵里」 — with {} not revealed the \
             condition is not met",
            missing
        );
    }
}

/// THE test for the centre-clause location leak: the cost-qualified BiBi must be found
/// on the STAGE. While the clause carried `revealed_cards`, putting a cost-17 BiBi
/// among the revealed cards satisfied it and a centre member of the right cost did not.
///
/// Renamed from `the_eleven_cost_bibi_…`: there is no eleven-cost BiBi in this test. It
/// stages a cost-2 member in the centre and puts the cost-17 in the reveal, so the
/// "eleven" was doing no work and naming a bound this test never exercises.
#[test]
fn the_cost_qualified_bibi_must_be_in_the_centre_not_the_reveal() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let foe_id = game.id(OPP_2);
    game.state.player2.stage.stage[MemberArea::Center as usize] = foe_id;
    // A cost-2 BiBi in the CENTRE, and a cost-17 BiBi among the REVEALED cards.
    let cheap = game.id(CENTRE_CHEAP);
    game.state.player1.stage.stage[MemberArea::Center as usize] = cheap;
    for c in [NICO, MAKI, ERI, CENTRE_DEAR] {
        game.state.revealed_cards.push(game.id(c));
    }
    let psychic = game.id(PSYCHIC);
    game.state.player1.live_card_zone.cards.push(psychic);

    let ability_id = {
        let card = game.db.get_card(psychic).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("自動"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(psychic).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(psychic);
    game.state.trigger_auto_ability(
        ability_id, AbilityTrigger::Auto, pid.clone(), Some(card_no), Some(psychic), None, None,
    );
    game.state.process_pending_auto_abilities(&pid);
    let _ = game.drain_auto_ability_choices();

    assert_ne!(
        game.state.mods.get_orientation_modifier(foe_id),
        Some("wait"),
        "自分のセンターエリアにコスト11以上の『BiBi』のメンバー — the 17-cost BiBi is in \
         the REVEALED set, not the centre, so the condition must not be met"
    );
}

/// Extra copies of a named character do not change anything.
///
/// Each of the three name clauses parses as its own `location_condition` with
/// `count: 1` and `operator: ">="`, and the text says 「…のメンバーカードがあり」 —
/// each name must be PRESENT, not accounted for. Two readings of the same three
/// clauses would both satisfy the tests above and are distinguished only here:
///
///   * present-at-least-once, which is what the parse says; and
///   * an exact multiset, i.e. exactly one にこ, exactly one 真姫, exactly one 絵里,
///     which would fail on a duplicate and which the `>=` operator rules out.
///
/// `dropping_any_one_of_the_three_named_characters_stops_it` cannot catch that
/// difference: it removes a name entirely, and both readings agree a missing name
/// fails. Two にこ revealed alongside one 真姫 and one 絵里 must still wait the foe.
#[test]
fn extra_copies_of_a_named_character_still_satisfy_its_clause() {
    assert!(
        run(CENTRE_DEAR, &[NICO, NICO, MAKI, ERI], OPP_2),
        "count: 1 with operator >= per name — 「矢澤にこ」と「西木野真姫」と「絢瀬絵里」の\
         メンバーカードがあり is a presence test, so a second にこ is redundant, not fatal"
    );
    // The control, and the reason the pair above is worth having. THREE cards, two
    // distinct names, no 絵里: the same card count as a passing reveal, so a rule
    // that read 「3 member cards among the reveal」 would pass this and is wrong.
    // `dropping_any_one_of_the_three_named_characters_stops_it` cannot catch that:
    // it drops to TWO cards, where every candidate rule agrees.
    assert!(
        !run(CENTRE_DEAR, &[NICO, NICO, MAKI], OPP_2),
        "three cards is not the clause — all THREE NAMES must be present. A duplicate \
         of one name does not make up for a missing one"
    );
}

/// The cost bound on the centre clause.
#[test]
fn a_cheap_centre_bibi_does_not_satisfy_the_cost_bound() {
    assert!(
        !run(CENTRE_CHEAP, &[NICO, MAKI, ERI], OPP_2),
        "コスト11以上 — a cost-2 BiBi in the centre is under the bound"
    );
}

/// Remaining coverage gap, recorded rather than filled: the コスト11以上 bound is pinned
/// by cost 17 (passes) and cost 2 (fails), which admits any threshold in `3..=17`.
///
/// Closing it needs a `unit: "BiBi"` member costing exactly 11 (or 10, for the nearest
/// miss). Note what this is NOT: a group-matching defect.
///
/// `PL!-bp3-003-P` (南ことり, cost 11 — exactly the bound) looks like the card this
/// needs, and it was used here to claim an off-by-one on the bound and then a
/// group-matching bug. Both were wrong. 南ことり is a μ's member whose `unit` field
/// says `"Printemps"`; `PL!-pb2-002-PP` (絢瀬絵里) is a genuinely different character
/// whose `unit` says `"BiBi"`. Group membership is matched on that string, so the
/// engine is CORRECT to exclude 南ことり from a 『BiBi』 clause. Two characters in two
/// groups, not one character in two printings.
///
/// So the bound stays loosely pinned until a `unit: "BiBi"` card at cost 11 turns up.
///

/// The group on the centre clause.
#[test]
fn a_cheap_centre_non_bibi_does_not_satisfy_the_group() {
    assert!(
        !run(CENTRE_NON_BIBI, &[NICO, MAKI, ERI], OPP_2),
        "『BiBi』のメンバー — a non-BiBi centre member must not satisfy the clause"
    );
}

/// The heart bound on the target, the same mechanism as 006 (≤1) and 033 (≤3).
#[test]
fn an_opponent_member_with_five_printed_hearts_is_not_waited() {
    assert!(
        !run(CENTRE_DEAR, &[NICO, MAKI, ERI], OPP_5),
        "元々持つハートの数が4つ以下 — 5 printed hearts is over the bound"
    );
}
