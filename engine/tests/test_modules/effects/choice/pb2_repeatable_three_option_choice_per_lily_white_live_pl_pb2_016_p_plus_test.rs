//! 東條 希 `PL!-pb2-016-P＋` (ライブ開始時)
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   ライブ開始時 — 自分の成功ライブカード置き場にある『lily white』のカード1枚につき、
//!   以下から1つを選ぶ。**同じ選択肢を2回以上選んでもよい。**
//!     ・自分のセンターエリアにいるメンバーは、ライブ終了時まで、ブレードを得る。
//!     ・自分のステージにいるメンバー1人をアクティブにする。
//!     ・カードを1枚引き、手札を1枚控え室に置く。
//!
//! # The parser fix this file pins
//!
//! All three options were collapsed into ONE mashed-together `draw_card` carrying the
//! centre position and the activate step from two of its siblings. `_try_choice`
//! builds the three options correctly when called directly, but the per-unit reading
//! claimed the text first — and the narrowing is deliberate: the guard in
//! `_try_per_unit` defers only for the REPEATABLE choice pattern
//! (「同じ選択肢を2回以上選んでもよい」), not for every choice.
//!
//! That distinction is load-bearing and has its own guard: on a plain one-shot choice
//! the per-unit reading of an option is the BETTER shape, and taking over universally
//! cost `PL!S-bp3-024-L` (Deep Resonance) option 0's member-selection target. The
//! suite caught it; the fix was to narrow, not to revert.
//!
//! # A second parser fix this file pins
//!
//! Option 1 used to parse as `target: "self"`, which the engine resolves to the
//! ACTIVATING card; `change_state` then auto-selected it through its `is_self_target`
//! shortcut instead of prompting, so the option silently did nothing.
//! 「自分のステージにいるメンバー1人をアクティブにする」 names a member the player PICKS.
//! `_fix_member_selection_target` now drops `target` for a counted member on a stage
//! unless the text says 「このメンバー」.

use crate::helpers::*;
use rabuka_engine::ability::enums::ActionType;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const RESONANCE: &str = "PL!-pb2-016-P＋";
const FILLER: &str = "PL!-sd1-010-SD";
/// A `lily white` live for the success live zone — the choice is taken per card.
const MUS_LIVE: &str = "PL!-pb1-029-SRL";
/// A real stage member for option 1 to activate.
const WAITED_MEMBER: &str = "PL!-pb2-002-PP"; // 絢瀬絵里, a BiBi member

/// Stand RESONANCE, load the success zone with `zone`, and fire ライブ開始時.
fn fired(zone: &[&str]) -> TestGame {
    fired_with_member(zone, None)
}

/// As `fired`, but with a WAITED member already on the centre stage, so a choice
/// option that acts on a stage member has something to act on when it resolves.
fn fired_with_member(zone: &[&str], member: Option<&str>) -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for c in zone {
        game.state
            .player1
            .success_live_card_zone
            .add_card(game.id(c));
    }
    let r = game.id(RESONANCE);
    game.assert_card_identity(r, RESONANCE);
    game.state.player1.live_card_zone.cards.push(r);

    if let Some(m) = member {
        let cid = game.id(m);
        game.state.player1.stage.stage[MemberArea::Center as usize] = cid;
        game.state.mods.add_orientation_modifier(cid, "wait");
    }

    let ability_id = {
        let card = game.db.get_card(r).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap_or_else(|| panic!("{RESONANCE} lacks a ライブ開始時 ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(r).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(r);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(r),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    game
}

/// The choice is PRESENTED, and picking option 1 activates a stage member.
///
/// This is the test the file used to record as an OPEN GAP. It now passes, and getting
/// it to pass involved two real defects and one fixture error of my own:
///
/// 1. The `target: "self"` parse described in the header.
/// 2. The same clause shape is broken on `PL!-pb2-015-P＋`, whose member-activating
///    option had never been exercised — its test only covered the energy option, and
///    even asserted the member stayed waited when the energy option was chosen. The
///    parser fix is shared, so both work now; the 015 file gained the missing branch.
/// 3. Not a bug: my first fixture here staged `PL!-pb1-010-N`, which exists in the
///    DATABASE (so `game.id()` correctly did not panic) but carries no unit, so the
///    effect found zero candidates. From inside the test that looks identical to an
///    engine bug, and I spent a cycle on it. `assert_card_exists` plus a real stage
///    member is the lesson.
#[test]
fn the_choice_is_presented_and_selecting_an_option_runs_it() {
    let mut game = fired_with_member(&[MUS_LIVE], Some(WAITED_MEMBER));
    let waited = game.state.player1.stage.stage[MemberArea::Center as usize];
    assert_ne!(waited, -1, "the fixture must put a member on the centre stage");

    assert!(
        game.has_pending_choice(),
        "以下から1つを選ぶ must present a choice"
    );
    // Option 1 = 「自分のステージにいるメンバー1人をアクティブにする」
    game.select_option(1);
    let mut g = 0;
    while game.has_pending_choice() && g < 5 {
        game.select_indices(&[0]);
        g += 1;
    }
    assert_ne!(
        game.state.mods.get_orientation_modifier(waited),
        Some("wait"),
        "selecting option 1 (自分のステージにいるメンバー1人をアクティブにする) must activate \
         the waited member — the option is parsed, presented and dispatched, and the \
         member it names is the object of the effect"
    );
}

/// The prompt does not enumerate its options. True of every choice in this engine, so
/// it is a property of the shape, not a defect in this card.
#[test]
fn the_prompt_does_not_enumerate_its_options() {
    let mut game = fired(&[MUS_LIVE]);
    assert!(game.has_pending_choice(), "the choice is offered");
    assert_eq!(
        game.pending_choice_count(),
        0,
        "the prompt selects by index and does not enumerate its options"
    );
}

/// The parse: a choice with three options, each with its own action.
#[test]
fn the_parse_is_a_three_option_choice() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let id = game.id(RESONANCE);
    let eff = {
        let owned = game
            .db
            .get_card(id)
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| (**e).clone()))
            .expect("ライブ開始時 ability has an effect")
    };
    assert_eq!(
        eff.action,
        ActionType::Choice,
        "以下から1つを選ぶ must parse as a choice, not a single action"
    );

    // `options` lives on the typed `AbilityEffect` through the flattened `compound`
    // box, so it IS readable here. Assert the option ACTIONS, not just the bullet count
    // on the text: the bullets survive a parse that mashes three options into one,
    // which is the exact regression this file exists to prevent.
    let opts = eff
        .options_any()
        .expect("以下から1つを選ぶ keeps its three options");
    assert_eq!(opts.len(), 3, "three printed options");
    assert_eq!(
        opts.iter().map(|o| o.action).collect::<Vec<_>>(),
        vec![
            ActionType::GainResource,
            ActionType::ChangeState,
            ActionType::Sequential,
        ],
        "ブレードを得る / メンバー1人をアクティブにする / 1枚引き手札1枚控え室に置く"
    );
    assert_eq!(
        opts[1].state_change_any(),
        Some("active"),
        "アクティブにする — without this the option is a no-op, which is what the \
         target fix addressed"
    );
    assert_eq!(
        opts[1].target, None,
        "メンバー1人 is CHOSEN by the player; target: self would resolve to the \
         activating card instead of prompting"
    );

    let bullets = eff.text.matches('\u{30fb}').count();
    assert_eq!(
        bullets, 3,
        "the three bulleted options are all on the effect: {}\n  text was {:?}",
        bullets,
        eff.text
    );
    assert!(
        eff.text.contains("同じ選択肢を2回以上選んでもよい"),
        "同じ選択肢を2回以上選んでもよい — the repeatable clause the per-unit guard keys on"
    );
}
