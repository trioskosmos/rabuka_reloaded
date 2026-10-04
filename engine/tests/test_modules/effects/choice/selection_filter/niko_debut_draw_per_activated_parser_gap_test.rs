/// 矢澤にこ `PL!-pb2-018-P＋` **ab#0** — a documented PARSER GAP, pinned.
///
///   相手のステージにいるウェイト状態のメンバーを3人までアクティブにしてもよい。
///   そうしたとき、これによりアクティブにしたメンバー1人につき、カードを1枚引く。
///
/// # The gap
///
/// The sentence has two halves. 「ウェイト状態のメンバーを3人までアクティブにしてもよい」
/// is an optional player choice; 「これによりアクティブにしたメンバー1人につき、カードを
/// 1枚引く」 is a payout scoped to what that choice activated.
///
/// Only the payout survived. The parsed effect (`cards/abilities.json:14274-14293`) is
///
///     condition  state_condition { state: "wait", target: "opponent", … }
///     count      1
///     action     draw_card
///     per_unit   true, per_unit_count 1, per_unit_type "人"
///
/// and contains **no action that activates anything** — no `change_state`, no
/// `state_change: "active"`. Resolving ab#0 therefore offers no choice beyond the
/// debut-order prompt, which is exactly what a prompt trace of the engine shows.
///
/// This is asserted rather than described because it is the kind of gap that gets
/// "fixed" by someone reading only the engine: an implementation that simply pays out
/// per opponent waited member on every debut of this card looks plausible, and no test
/// would object, because none of them asked whether the choice existed.
///
/// # What is deliberately NOT asserted
///
/// The engine's actual payout for ab#0. With the activation missing, `per_unit` over
/// `per_unit_type: "人"` is ambiguous between "per member the player activated" and "per
/// waited member on the opponent's stage", and a test that picked one of those readings
/// would be asserting a decision nobody has made rather than a behaviour the card
/// specifies.
///
/// # Coverage context
///
/// The sibling ability is covered thoroughly by
/// `pb2_discard_three_distinct_bibi_waits_foe_pl_pb2_018_p_plus_test.rs`, including a
/// parse-level pin in the same spirit (`the_cost_requires_three_distinct_names`). Every
/// test in that file resolves the debut-order prompt with `select_indices(&[1])`, so
/// ab#0 has never been exercised from the test suite at all.
///
/// Read through `serde_json` rather than through typed accessors, matching
/// `the_cost_requires_three_distinct_names` in that sibling file: a missing accessor
/// would be a compile error about MY test, whereas a missing JSON key is exactly the
/// failure these assertions exist to catch.
use crate::helpers::*;
use serde_json::Value;

const ERI: &str = "PL!-pb2-018-P＋";

/// ab#0's parsed effect, as JSON plus the two per-unit values read through the
/// typed accessors.
///
/// Two routes because neither covers everything: `per_unit` is not serialised, so the
/// JSON cannot see it, while `state_change` reads cleanly there and has no accessor
/// worth guessing at. Both are read while `effect` is still borrowed, so the returned
/// values are owned.
///
/// Located by shape rather than by index: the index is exactly what a parser change
/// would renumber, and a test hard-coding `abilities()[0]` would then point at the wrong
/// ability and pass for the wrong reason.
fn debut_draw_effect(game: &TestGame) -> (Value, Option<u8>, Option<String>) {
    let card = game
        .db
        .get_card(game.id(ERI))
        .expect("矢澤にこ must be in cards.json")
        .clone();
    // `resolved_abilities()` yields `Arc<Ability>` by value. Collected into a `Vec` so
    // the abilities — and the `&AbilityEffect` borrowed from each — outlive the search.
    let abilities = card.resolved_abilities().collect::<Vec<_>>();
    for ability in &abilities {
        if ability.triggers.as_deref() != Some("登場") {
            continue;
        }
        let Some(effect) = ability.effect.as_deref() else {
            continue;
        };
        let value = serde_json::to_value(effect).expect("an effect must serialize");
        if value.get("action").and_then(Value::as_str) == Some("draw_card") {
            return (
                value,
                effect.per_unit_count_any(),
                effect.per_unit_type_any().map(str::to_string),
            );
        }
    }
    panic!("ab#0 parses to a 登場 ability whose action is draw_card")
}

/// The gap, stated as a test.
///
/// INVERT these assertions when the activation half is parsed. They fail today on
/// purpose: a green suite here would mean the gap is gone, which is the point.
#[test]
fn niko_debut_draw_ability_has_no_activation_action() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let (effect, _per_unit_count, _per_unit_type) = debut_draw_effect(&game);

    // No activation. An activation would be a `change_state` carrying
    // `state_change: "active"`; there is none, and no nested action of any kind.
    assert_ne!(
        effect.get("action").and_then(Value::as_str),
        Some("change_state"),
        "GAP: 「ウェイト状態のメンバーを3人までアクティブにしてもよい」 has no \
         representation — expected a change_state with state_change \"active\""
    );
    assert!(
        effect.get("state_change").is_none(),
        "GAP: no `state_change` key at all, so nothing in this effect can move a member \
         from wait to active; effect={effect}"
    );

    // The cap is gone too: 「3人まで」 bounds a choice, and with no choice there is
    // nothing for it to bound. `count` is 1 — that is 「カードを1枚引く」, the payout,
    // not the number that may be activated.
    assert_eq!(
        effect.get("count").and_then(Value::as_i64),
        Some(1),
        "GAP: 「1人につき」 survives as per_unit_count, but 「3人まで」 — the cap on how \
         many may be activated — is not in the parse; effect={effect}"
    );
}

/// The per-unit payout IS represented, so the gap is confined to the activation.
///
/// Separate from the test above so closing the activation gap cannot silently change
/// what is being protected: 「これにより…1人につき」 is in the parse as `per_unit` with
/// `per_unit_count: 1`, and that half is intact.
#[test]
fn niko_debut_draw_payout_per_unit_is_parsed() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let (effect, per_unit_count, per_unit_type) = debut_draw_effect(&game);

    assert_eq!(
        per_unit_count,
        Some(1),
        "「これによりアクティブにしたメンバー1人につき、カードを1枚引く」 — the per-unit \
         shape survives parsing even though the activation it counts does not"
    );
    assert_eq!(
        per_unit_type.as_deref(),
        Some("人"),
        "the unit counted is a 人, i.e. a member"
    );
    assert_eq!(
        effect.get("source").and_then(Value::as_str),
        Some("deck"),
        "自分のデッキ"
    );
    assert_eq!(
        effect.get("destination").and_then(Value::as_str),
        Some("hand"),
        "手札に加える"
    );
}