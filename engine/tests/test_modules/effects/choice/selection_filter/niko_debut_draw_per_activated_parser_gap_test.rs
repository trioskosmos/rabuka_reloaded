/// 矢澤にこ `PL!-pb2-018-P＋` **ab#0** — both halves of an optional offer + payout.
///
///   相手のステージにいるウェイト状態のメンバーを3人までアクティブにしてもよい。
///   そうしたとき、これによりアクティブにしたメンバー1人につき、カードを1枚引く。
///
/// # This file used to pin a PARSER GAP. It is now inverted (2026-10-04)
///
/// The sentence has two halves. 「ウェイト状態のメンバーを3人までアクティブにしてもよい」
/// is an optional player choice; 「これによりアクティブにしたメンバー1人につき、カードを
/// 1枚引く」 is a payout scoped to what that choice activated.
///
/// Only the payout used to survive. The parse was a bare
///
///     condition  state_condition { state: "wait", target: "opponent", … }
///     count      1
///     action     draw_card
///     per_unit   true, per_unit_count 1, per_unit_type "人"
///
/// with **no action that activates anything** — no `change_state`, no
/// `state_change: "active"` — so resolving ab#0 offered no choice at all. Two things were
/// lost with it: the 「3人まで」 cap, and the 「てもよい」 optionality. The card drew 1 for
/// merely *having* a waited member on the opponent's stage.
///
/// The gap was tracked here rather than in `PARSER_NOTES.md` alone, which is what made it
/// findable: this file's own header said to invert the assertions when the activation half
/// parsed. That is now done, and the shape is pinned below.
///
/// # Why `sequential` + `conditional`, not `conditional_on_optional`
///
/// The natural-looking shape is the one the parser uses for every 「〜してもよい。そうした
/// とき、〜」 (ミア, 花帆, 澁谷かのん): `conditional_on_optional` with an `optional_action` and
/// a `conditional_action`. **That is wrong here.** `execute_conditional_on_optional` treats
/// the offer as a COST — it presents a Skip/Pay choice and runs the consequence on Pay.
/// Activating the opponent's members is an effect, not a payment; filing it as a cost
/// would let a player "pay" by waking nobody. Action-shaped optionals go through
/// `sequential` + `conditional: true`, the same route DIVE! takes (see the NOTE in
/// `parser.py` above `_try_conditional`'s caller).
///
/// # What is deliberately NOT asserted
///
/// The engine's actual payout for ab#0. With the offer parsed, `per_unit` over
/// `per_unit_type: "人"` should mean "per member the player activated" — that is the
/// printed reading — but no test here resolves the ability end to end, because the sibling
/// file's tests all resolve the debut-order prompt with `select_indices(&[1])` and this
/// ability's own prompt is a separate question. Asserting a payout nobody has watched
/// resolve would be pinning a guess; what is pinned here is the PARSE, which is what was
/// actually broken.
///
/// # Coverage context
///
/// The sibling ability is covered thoroughly by
/// `pb2_discard_three_distinct_bibi_waits_foe_pl_pb2_018_p_plus_test.rs`, including a
/// parse-level pin in the same spirit (`the_cost_requires_three_distinct_names`).
///
/// Read through `serde_json` rather than through typed accessors, matching
/// `the_cost_requires_three_distinct_names` in that sibling file: a missing accessor
/// would be a compile error about MY test, whereas a missing JSON key is exactly the
/// failure these assertions exist to catch.
use crate::helpers::*;

const ERI: &str = "PL!-pb2-018-P＋";

/// ab#0's ability, located by shape: a 登場 `sequential` of exactly two legs whose second
/// is a `draw_card`.
///
/// Located by shape rather than by index or by root action: the index is exactly what a
/// parser change would renumber, and hard-coding the old root action (`draw_card`) would
/// make this finder stop matching the moment the gap was closed — which is what happened
/// on the first attempt at inverting these tests.
///
/// `conditional` is read here because it decides the ORDER of operations: the payout must
/// not run unless the offer was taken.
fn debut_draw_ability(game: &TestGame) -> std::sync::Arc<rabuka_engine::card::Ability> {
    let card = game
        .db
        .get_card(game.id(ERI))
        .expect("矢澤にこ must be in cards.json")
        .clone();
    for ability in card.resolved_abilities() {
        if ability.triggers.as_deref() != Some("登場") {
            continue;
        }
        let Some(effect) = ability.effect.as_deref() else {
            continue;
        };
        if effect.action != rabuka_engine::ability::enums::ActionType::Sequential {
            continue;
        }
        let legs = effect
            .compound_branch()
            .and_then(|c| c.actions.as_ref())
            .expect("a sequential carries its legs");
        if legs.len() == 2
            && legs[1].action == rabuka_engine::ability::enums::ActionType::DrawCard
        {
            return ability;
        }
    }
    panic!("ab#0 does not parse to a two-leg sequential ending in a draw_card")
}

/// The activation half IS represented — the gap this file was written to pin is closed.
///
/// The sentence has two halves. 「ウェイト状態のメンバーを3人までアクティブにしてもよい」
/// is an optional player choice; 「これによりアクティブにしたメンバー1人につき、カードを
/// 1枚引く」 is a payout scoped to what that choice activated. Only the payout used to
/// survive; the parse now carries both, wired so the payout depends on the offer.
///
/// The shape is `sequential` + `conditional`, NOT `conditional_on_optional`:
/// `execute_conditional_on_optional` presents the offer as a Skip/Pay CHOICE and runs the
/// consequence on Pay, which is a cost-shaped contract. Activating the opponent's members
/// is an effect, not a payment — misfiling it as a cost would let a player "pay" by waking
/// nobody. Action-shaped optionals go through the sequential pipeline, the same route
/// DIVE! takes (see the note in `parser.py` above `_try_conditional`'s caller).
#[test]
fn niko_debut_draw_ability_carries_the_activation_half() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let ability = debut_draw_ability(&game);
    let effect = ability.effect.as_deref().expect("has an effect");

    assert_eq!(
        effect.conditional,
        Some(true),
        "the payout depends on the offer, so the container is a CONDITIONAL sequential"
    );

    let legs = effect.compound_branch().and_then(|c| c.actions.as_ref()).expect("two legs");
    let offer = legs[0].as_ref();

    assert_eq!(
        offer.action,
        rabuka_engine::ability::enums::ActionType::ChangeState,
        "「ウェイト状態のメンバーを3人までアクティブに」 — expected the activation leg"
    );
    assert_eq!(
        offer.state_change_any(),
        Some("active"),
        "ウェイト状態 → アクティブ"
    );
    assert_eq!(
        offer.target_any(),
        Some("opponent"),
        "相手のステージにいる…ウェイト状態のメンバー"
    );
    assert_eq!(
        offer.card_type_any(),
        Some(&rabuka_engine::card::CardType::Member),
        "the offer acts on メンバー"
    );
    assert_eq!(
        offer.count,
        Some(3),
        "「3人まで」 is the CAP on how many may be activated — the field the old parse lost"
    );
    assert_eq!(
        offer.max,
        Some(true),
        "「まで」 makes 3 a ceiling, not an exact count: waking one member is allowed"
    );
    assert_eq!(
        offer.optional,
        Some(true),
        "「…にしてもよい」 — the offer is the player's may-I choice, which is what the old \
         parse dropped along with the cap"
    );
}

/// The per-unit payout is represented AND now hangs off the activation.
///
/// Separate from the test above so a future change to either half is visible on its own.
/// 「これにより…1人につき」 is `per_unit` with `per_unit_count: 1`, and 「自分のデッキ」 → 手札
/// is the draw.
#[test]
fn niko_debut_draw_payout_per_unit_is_parsed() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let ability = debut_draw_ability(&game);
    let effect = ability.effect.as_deref().expect("has an effect");
    let legs = effect.compound_branch().and_then(|c| c.actions.as_ref()).expect("two legs");
    let payout = legs[1].as_ref();

    assert_eq!(
        payout.action,
        rabuka_engine::ability::enums::ActionType::DrawCard,
        "カードを1枚引く"
    );
    assert_eq!(payout.per_unit_any(), Some(true), "1人につき");
    assert_eq!(
        payout.per_unit_count_any(),
        Some(1),
        "「これによりアクティブにしたメンバー1人につき、カードを1枚引く」"
    );
    assert_eq!(
        payout.per_unit_type_any(),
        Some("人"),
        "the unit counted is a 人, i.e. a member"
    );
    assert_eq!(payout.source_any().as_deref(), Some("deck"), "自分のデッキ");
    assert_eq!(
        payout.destination_any().as_deref(),
        Some("hand"),
        "手札に加える"
    );
}