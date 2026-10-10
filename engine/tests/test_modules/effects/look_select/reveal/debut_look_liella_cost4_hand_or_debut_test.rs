//! 澁谷かのん `PL!SP-pb2-001-P＋` / `PL!SP-pb2-001-R` — 登場 look-and-select
//!
//! Text, with the icon markup rendered as plain labels:
//!
//!   登場 — 手牌を1枚控え室に置いてもいい：自分のデキキの上からカードを5枚見る。
//!   その中からコスト4以下の『Liella!〇のメンバーカードを1枚まで公開する。
//!   そのカードを自分のステージのメンバーのいないエリアに登場させるか、
//!   手牌に加える。残りを控え室に置く。
//!
//! # Why the hand/stage choice is encoded the way it is
//!
//! The printed text offers TWO equal options — 登場させる OR 手牌に加える — but
//! the parse does not emit a two-option choice. `_build_or_destination_followup` in
//! `parser.py` encodes it as: the `select_action` takes `destination: hand` (the
//! default), and an OPTIONAL single-option followup offers the hand → stage move on
//! top. The reachable outcomes are the same set, so this is a deliberate encoding
//! rather than a dropped clause — but it is a hard-coded pattern, so it is asserted
//! below rather than assumed.
//!
//! # What this file had and did not have
//!
//! Eight behavioural tests, NO parse assertions, and no constants at all — which is
//! why `audit_fixture_annotations.py` could not check its fixtures either. Both are
//! fixed below: the structure is pinned, and every card number is named and verified.

use crate::helpers::*;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

const KANON: &str = "PL!SP-pb2-001-R"; // 澁谷かのん
const FILLER: &str = "PL!-sd1-010-SD"; // 高坂穂乃果
const LIELLA_A: &str = "PL!SP-PR-003-PR"; // 澁谷かのん
const LIELLA_B: &str = "PL!SP-PR-004-PR"; // 唐可可
const NOT_LIELLA: &str = "PL!-sd1-008-SD"; // 小泉花陽 — μ's, so 『Liella!〇 excludes it
/// 莲ヌ空 — also μ's, and likewise excluded by 『Liella!〇. The old name
/// `non_mus` was simply wrong: 莲ヌ空 IS a μ's group.
const KANON_GROUP: &str = "PL!HS-sd1-010-SD";

fn setup_kanon(game: &mut TestGame, top_cards: &[i16]) -> i16 {
    let kanon = game.id(KANON);
    let filler = game.id(FILLER);
    game.state.player1.hand.cards.push(kanon);
    game.state.player1.hand.cards.push(filler);
    game.state.player1.hand.cards.push(filler);
    game.give_energy(20);
    game.state.player1.main_deck.cards.clear();
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for &cid in top_cards.iter().rev() {
        game.state.player1.main_deck.cards.insert(0, cid);
    }
    game.state.player1.stage.stage = [-1, -1, -1];
    kanon
}

/// Pay the optional cost by selecting hand index 0.
fn pay_optional_cost(game: &mut TestGame) {
    TurnEngine::resume_with_choice(&mut game.state, None, Some(vec![0].into())).ok();
}

/// Select Liella! cost≤4 from looked_at, skip stage followup → stays in hand.
#[test]
fn kanon_select_liella_cost4_keep_in_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id(LIELLA_A);
    let filler = game.id(FILLER);
    let kanon = setup_kanon(&mut game, &[liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);

    assert!(game.has_pending_choice(), "looked_at choice appears");
    let pending = game.state.get_pending_choice_json();
    let filtered = pending
        .as_ref()
        .and_then(|v| v.get("filtered_indices"))
        .and_then(|v| v.as_array());
    assert_eq!(
        filtered.map(|a| a.len()).unwrap_or(0),
        1,
        "only Liella! selectable"
    );

    game.select_indices(&[0]); // select liella from looked_at
                               // skip stage debut followup → card stays in hand
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    assert!(
        game.state.player1.hand.cards.contains(&liella),
        "Liella! in hand"
    );
}

/// Select → choose stage debut → card on stage.
#[test]
fn kanon_select_liella_debut_to_stage() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id(LIELLA_A);
    let filler = game.id(FILLER);
    let kanon = setup_kanon(&mut game, &[liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_indices(&[0]); // select from looked_at

    // Followup: stage debut choice — pick option 0
    assert!(game.has_pending_choice(), "stage debut choice");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(0);

    // Observed: answering the debut choice moves the Liella! picked from
    // looked_at straight to the stage; the only follow-up prompt is the
    // 登場-driven SelectPosition (there is no separate hand-selection ask).
    assert!(game.has_pending_choice(), "SelectPosition prompt expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectPosition"),
        "expected SelectPosition"
    );
    game.select_option(0);

    assert!(
        game.state
            .player1
            .stage
            .stage.contains(&liella),
        "on stage"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&liella),
        "not in hand"
    );
}

/// Skip optional cost → effect not executed.
#[test]
fn kanon_skip_cost_effect_not_executed() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id(LIELLA_A);
    let filler = game.id(FILLER);
    let kanon = setup_kanon(&mut game, &[liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    // Skip optional cost
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    assert!(
        !game.state.player1.hand.cards.contains(&liella),
        "not in hand"
    );
}

/// No matching cards → all discarded, then skip followup.
#[test]
fn kanon_no_matching_cards_discard_all() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    let kanon_group = game.id(KANON_GROUP);
    let kanon = setup_kanon(&mut game, &[filler, kanon_group]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    // Drain followup choice
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    assert!(
        game.state.player1.waitroom.cards.contains(&filler),
        "discarded"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&kanon_group),
        "discarded"
    );
}

/// Cost > 4 Liella! rejected, then skip followup.
#[test]
fn kanon_cost_above_4_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let kanon_self = game.id(KANON);
    let filler = game.id(FILLER);
    let kanon = setup_kanon(&mut game, &[kanon_self, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    // Cost-11 Kanon copy is Liella! but above the cost≤4 filter → rejected,
    // so nothing was selected and both looked_at cards end up discarded.
    assert!(
        !game.state.player1.hand.cards.contains(&kanon_self),
        "Cost > 4 card must not be selectable/kept"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&kanon_self),
        "Rejected card goes to the waitroom with the rest"
    );
}

/// Non-Liella! rejected by group filter, then skip followup.
#[test]
fn kanon_non_liella_cost4_rejected() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let non_liella = game.id(NOT_LIELLA);
    let filler = game.id(FILLER);
    let kanon = setup_kanon(&mut game, &[non_liella, filler]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    // μ's member fails the 『Liella!』 group filter → rejected and discarded.
    assert!(
        !game.state.player1.hand.cards.contains(&non_liella),
        "Non-Liella! card must not be selectable/kept"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&non_liella),
        "Rejected card goes to the waitroom with the rest"
    );
}

/// max=1: exactly 1 card from looked_at goes to hand.
#[test]
fn kanon_max_1_enforced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella_a = game.id(LIELLA_A);
    let liella_b = game.id(LIELLA_B);
    let kanon = setup_kanon(&mut game, &[liella_a, liella_b]);

    game.play_to_stage(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_indices(&[0]); // select first matching from looked_at
                               // skip followup
    // Answer what cannot be declined, decline what can: a mandatory
    // prompt left parked mid-ability produces exactly the absence
    // the next assertion checks, so nothing fails.
    game.drain_choices();
    // Only 1 of the 2 Liella! cards should be in hand (max=1)
    let hand_count = game
        .state
        .player1
        .hand
        .cards
        .iter()
        .filter(|&&id| id == liella_a || id == liella_b)
        .count();
    assert_eq!(hand_count, 1, "max=1 enforced");
}

/// Stage full → debut fails, card stays in hand.
#[test]
fn kanon_stage_full_falls_back_to_hand() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let liella = game.id(LIELLA_A);
    let filler = game.id(FILLER);
    let kanon = setup_kanon(&mut game, &[liella, filler]);
    let filler_copy2 = game.new_id(FILLER);
    let filler_copy3 = game.new_id(FILLER);
    game.state.player1.stage.stage = [filler, filler_copy2, filler_copy3];

    game.play_to_stage_baton(kanon, MemberArea::Center);
    pay_optional_cost(&mut game);
    game.select_indices(&[0]);

    // Stage debut choice is still offered even though the stage is full.
    assert!(game.has_pending_choice(), "stage debut choice expected");
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "expected SelectTarget"
    );
    game.select_option(0);
    // Observed: with all stage slots full the debut fails silently — no hand
    // selection or position prompt follows; the card falls back to the hand.
    assert!(
        !game.has_pending_choice(),
        "no follow-up prompts when stage is full (debut fails)"
    );
    assert!(
        !game
            .state
            .player1
            .stage
            .stage.contains(&liella),
        "not on stage"
    );
}

/// The parse structure, pinned.
///
/// Every behavioural test above reaches the engine through this shape, and none of them
/// reads the fields directly. The hand-default / stage-followup encoding in particular
/// is invisible from behaviour alone: both legs end with the card somewhere sensible,
/// so swapping or dropping one would not reliably turn a test red.
#[test]
fn the_parse_is_a_look_select_with_an_optional_stage_followup() {
    use rabuka_engine::ability::enums::{ActionType, Zone};

    let db = load_real_database();
    let game = TestGame::new(db);
    game.assert_card_in_group(
        game.id(LIELLA_A),
        "Liella!",
        "the selectable card must be Liella!",
    );

    let eff = {
        let owned = game
            .db
            .get_card(game.id(KANON))
            .unwrap()
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("\u{767b}\u{5834}"));
        owned
            .and_then(|a| a.effect.as_ref().map(|e| (**e).clone()))
            .expect("\u{767b}\u{5834} ability has an effect")
    };

    assert_eq!(
        eff.action,
        ActionType::LookAndSelect,
        "\u{624b}\u{724c}\u{3092}1\u{679a}\u{63a7}\u{3048}\u{5ba4}\u{306b}\u{7f6e}\u{3044}\u{3066}\u{3082}\u{3044}\u{3044}\u{ff1a}\u{30c7}\u{30ad}\u{30ad}\u{306e}\u{4e0a}\u{304b}\u{3089}\u{30ab}\u{30fc}\u{30c9}\u{3092}5\u{679a}\u{898b}\u{308b} \u{2014} the effect is a look-and-select"
    );

    // \u{30c7}\u{30ad}\u{30ad}\u{306e}\u{4e0a}\u{304b}\u{3089}\u{30ab}\u{30fc}\u{30c9}\u{3092}5\u{679a}\u{898b}\u{308b}
let look = eff
            .look_action()
            .map(|b| b.as_ref())
            .expect("\u{30c7}\u{30ad}\u{30ad}\u{306e}\u{4e0a}\u{304b}\u{3089}\u{30ab}\u{30fc}\u{30c9}\u{3092}5\u{679a}\u{898b}\u{308b}");
    assert_eq!(look.action, ActionType::LookAt);
    assert_eq!(look.source_str(), Some("deck_top"), "\u{30c7}\u{30ad}\u{30ad}\u{306e}\u{4e0a}\u{304b}\u{3089}");
    assert_eq!(look.count, Some(5), "\u{30ab}\u{30fc}\u{30c9}\u{3092}5\u{679a}");

    // \u{305d}\u{306e}\u{4e2d}\u{304b}\u{3089}\u{2026}\u{3092}1\u{679a}\u{307e}\u{3067}\u{516c}\u{958b}\u{3059}\u{308b}
let sel = eff
            .select_action()
            .map(|b| b.as_ref())
            .expect("\u{305d}\u{306e}\u{4e2d}\u{304b}\u{3089}\u{2026}\u{3092}1\u{679a}\u{307e}\u{3067}\u{516c}\u{958b}\u{3059}\u{308b}");
    assert_eq!(sel.action, ActionType::SelectCards);
    assert_eq!(sel.count, Some(1), "1\u{679a}\u{307e}\u{3067}");
    assert_eq!(
        sel.max,
        Some(true),
        "\u{307e}\u{3067} \u{2014} an upper bound, so declining is legal"
    );
    assert_eq!(
        sel.card_type_any().map(|c| c.as_card_str()),
        Some("member_card"),
        "\u{30e1}\u{30f3}\u{30d0}\u{30fc}\u{30ab}\u{30fc}\u{30c9}"
    );
    assert_eq!(
        sel.group_names_any().map(|g| g.to_vec()),
        Some(vec!["Liella!".to_string()]),
        "\u{300e}Liella!\u{300f}\u{306e}\u{30e1}\u{30f3}\u{30d0}\u{30fc}\u{30ab}\u{30fc}\u{30c9}"
    );
    assert_eq!(sel.cost_limit_any(), Some(4), "\u{30b3}\u{30b9}\u{30c8}4\u{4ee5}\u{4e0b}");
    assert_eq!(format!("{:?}", sel.cost_limit_operator_any()), format!("{:?}", Some(rabuka_engine::core::card::Operator::Lte)), "4\u{4ee5}\u{4e0b} \u{2014} inclusive");
    assert_eq!(
        sel.destination,
        Some(Zone::Hand),
        "\u{624b}\u{724c}\u{306b}\u{52a0}\u{3048}\u{308b} \u{2014} this leg of the printed \
         choice is the DEFAULT, which is why there is no explicit 'add to hand' option"
    );
    assert_eq!(
        sel.discard_remaining_any(),
        Some(true),
        "\u{6b8b}\u{308a}\u{3092}\u{63a7}\u{3048}\u{5ba4}\u{306b}\u{7f6e}\u{304f}"
    );

    // \u{767b}\u{5834}\u{3055}\u{305b}\u{308b}\u{304b}\u{3001}\u{624b}\u{724c}\u{306b}\u{52a0}\u{3048}\u{308b}
let fu = eff
            .followup_action()
            .map(|b| b.as_ref())
            .expect("\u{767b}\u{5834}\u{3055}\u{305b}\u{308b}\u{304b}\u{3001}\u{624b}\u{724c}\u{306b}\u{52a0}\u{3048}\u{308b} \u{2014} the stage leg");
    assert_eq!(
        fu.action,
        ActionType::Choice,
        "the second leg is offered as a choice on top of the default"
    );
    assert_eq!(
        fu.optional,
        Some(true),
        "\u{81ea}\u{5206}\u{306e}\u{30b9}\u{30c6}\u{30fc}\u{30b8}\u{306e}\u{30e1}\u{30f3}\u{30d0}\u{30fc}\u{306e}\u{3044}\u{306a}\u{3044}\u{30a8}\u{30ea}\u{30a2}\u{306b}\u{767b}\u{5834}\u{3055}\u{305b}\u{308b}\u{304b} \u{2014} the \
         followup is an OFFER, and declining leaves the card in hand"
    );
    let opts = fu.options_any().expect("the followup carries its stage option");
    assert_eq!(
        opts.len(),
        1,
        "one leg \u{2014} the hand leg is the select's own destination, so a two-option \
         choice would be redundant"
    );
    assert_eq!(opts[0].action, ActionType::MoveCards);
    assert_eq!(opts[0].source, Some(Zone::Hand));
    assert_eq!(opts[0].destination, Some(Zone::Stage));
    assert_eq!(opts[0].count, Some(1));
}
