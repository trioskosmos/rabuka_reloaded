//! Corpus gate: nothing in `abilities.json` may degrade to a silent no-op.
//!
//! Two decode paths are deliberately FORWARD-COMPATIBLE rather than strict, because
//! the card pool is still being transcribed and a hard error on an unrecognised token
//! would take the whole engine down over one new card:
//!
//! * `triggers::parse_triggers` drops any token it does not recognise, so newly printed
//!   trigger text degrades to "no recognised trigger" instead of misfiring as a
//!   different kind (`triggers.rs`, audit item 11).
//! * An effect `action` the wire table does not know decodes to
//!   `ActionType::Custom`, which the VM treats as a no-op (`vm.rs`, audit item 8).
//!
//! Both are the right call at the decode site, and both are a hazard in aggregate: an
//! ability that decodes to nothing never fires, no log line says so, and no per-card
//! test fails — because there is no per-card test for an ability that never runs. The
//! suite can go 100% green on a card that is completely dead.
//!
//! `action_coverage_test.rs` closes the same gap for actions, but only over MEMBER
//! cards and only for actions that fail loudly at resolution. This file is the
//! whole-corpus, decode-side counterpart: it fails on the data the moment a token or
//! action stops being recognised, which is the moment it is cheap to fix.
//!
//! Both assertions pass on today's corpus. They are here to keep them passing.

use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use std::collections::BTreeSet;

/// Pin the premise the gate above rests on.
///
/// `parse_triggers` ignoring what it does not recognise is the whole reason the gate
/// is needed — and it is also what would make the gate pass for the wrong reason. If a
/// future change made `parse_triggers` accept everything, the corpus test would go
/// quiet while proving nothing. This asserts the detector can still tell the
/// difference.
#[test]
fn the_trigger_detector_actually_distinguishes_known_from_unknown() {
    use rabuka_engine::triggers::parse_triggers;

    assert_eq!(
        parse_triggers("登場").count(),
        1,
        "a real trigger is recognised"
    );
    assert_eq!(
        parse_triggers("ライブ開始時, 登場").count(),
        2,
        "a comma-separated pair is recognised, in order"
    );
    assert_eq!(
        parse_triggers("この能力は存在しないトリガーである").count(),
        0,
        "an unrecognised token yields nothing rather than misfiring as another kind — \
         this is the silent degradation the corpus gate exists to catch"
    );
    assert_eq!(
        parse_triggers("登場, この能力は存在しないトリガーである").count(),
        1,
        "one bad token in a pair is dropped, and the good one survives"
    );
}

/// Every `duration` in the corpus is one `parse_duration` accepts.
///
/// Both call sites that read a duration already fail loudly — `push_temporary_effect`
/// and `set_heart_override` each log at error level and return without registering the
/// effect. That is the right local behaviour, but it is not a *test*: a newly printed
/// duration phrase the parser spells as something `parse_duration` does not know would
/// make every effect carrying it silently not register, with only a log line to show
/// for it and a green suite.
///
/// This closes that gap the same way the action and trigger gates above do.
#[test]
fn every_duration_in_the_corpus_is_parseable() {
    let db = load_real_database();
    let mut unparseable: BTreeSet<String> = BTreeSet::new();
    let mut seen = 0usize;

    fn scan(
        eff: &rabuka_engine::card::AbilityEffect,
        card_no: &str,
        bad: &mut BTreeSet<String>,
        seen: &mut usize,
    ) {
        if let Some(d) = eff.duration_any() {
            *seen += 1;
            if rabuka_engine::ability::util::parse_duration(d).is_none() {
                bad.insert(format!("{card_no}: duration={d:?}"));
            }
        }
        if let Some(actions) = eff.compound.actions.as_ref() {
            for sub in actions {
                scan(sub, card_no, bad, seen);
            }
        }
    }

    for card in db.cards.values() {
        for ab in card.resolved_abilities() {
            if let Some(eff) = ab.effect.as_deref() {
                scan(eff, card.card_no.as_ref(), &mut unparseable, &mut seen);
            }
        }
    }

    assert!(
        unparseable.is_empty(),
        "{} duration values are not in parse_duration's vocabulary, so the effects \
         carrying them are never registered: push_temporary_effect and set_heart_override \
         both log an error and return, which is loud in a log and silent to the suite. \
         Add the wire form to parse_duration (ability/util/duration.rs). Offenders: \
         {unparseable:?}",
        unparseable.len()
    );
    assert!(seen > 100, "sanity: the corpus should be large, saw {seen} durations");
}

/// Every non-null ability names at least one trigger the engine recognises.
///
/// `is_null` entries are the synthetic "this card has no printed ability" stubs the
/// parser emits (PL!HS-PR-010-PR and friends: `effect: null`, empty text, no trigger).
/// They are expected to be empty, so they are excluded rather than reported.
#[test]
fn every_ability_names_a_recognized_trigger() {
    let db = load_real_database();
    let mut unrecognised: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for card in db.cards.values() {
        for ab in card.resolved_abilities() {
            if ab.is_null {
                continue;
            }
            checked += 1;
            let raw = ab.triggers.as_deref().unwrap_or("").trim();
            if rabuka_engine::triggers::parse_triggers(raw).next().is_none() {
                unrecognised.push(format!("{}: triggers={raw:?}", card.card_no));
            }
        }
    }

    unrecognised.sort();
    unrecognised.dedup();
    assert!(
        unrecognised.is_empty(),
        "{} of {checked} abilities name a trigger `parse_triggers` does not recognise, so \
         they can never fire and nothing reports it. Add the token to TriggerKind::from_token \
         (triggers.rs) or correct the parser's `triggers` value. Offenders: {unrecognised:?}",
        unrecognised.len()
    );
    assert!(checked > 100, "sanity: the corpus should be large, saw {checked}");
}

/// No effect anywhere in the corpus decodes to `ActionType::Custom`.
///
/// `Custom` is what an action string the wire table has never heard of decodes to, and
/// the VM treats it as a success with no effect. A card carrying one is silently inert:
/// it resolves, logs nothing, and changes no state. Unlike an unknown *trigger* (which
/// stops the ability firing at all), this one fails while LOOKING like it worked, which
/// is worse.
#[test]
fn no_effect_decodes_to_the_no_op_custom_action() {
    let db = load_real_database();
    let mut custom: BTreeSet<String> = BTreeSet::new();
    let mut seen = 0usize;

    for card in db.cards.values() {
        for ab in card.resolved_abilities() {
            if let Some(eff) = ab.effect.as_deref() {
                collect_custom(eff, card.card_no.as_ref(), &mut custom, &mut seen);
            }
        }
    }

    assert!(
        custom.is_empty(),
        "{} effect nodes decode to ActionType::Custom, which the VM executes as a no-op: \
         those cards resolve successfully and change nothing. Add the action to the wire \
         ActionType table (engine/src/ability/enums.rs) or fix the parser's `action`. \
         Offenders: {custom:?}",
        custom.len()
    );
    assert!(seen > 500, "sanity: the corpus should be large, saw {seen} effect nodes");
}

/// Count every effect node, and name the ones whose action decoded to `Custom`.
fn collect_custom(
    eff: &rabuka_engine::card::AbilityEffect,
    card_no: &str,
    out: &mut BTreeSet<String>,
    seen: &mut usize,
) {
    *seen += 1;
    if eff.action == ActionType::Custom {
        out.insert(format!("{card_no}: {:?}", eff.text.as_ref()));
    }
    if let Some(actions) = eff.compound.actions.as_ref() {
        for sub in actions {
            collect_custom(sub, card_no, out, seen);
        }
    }
}
