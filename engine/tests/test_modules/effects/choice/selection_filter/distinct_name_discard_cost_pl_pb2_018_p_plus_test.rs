//! 矢澤にこ `PL!-pb2-018-P＋` — 「名前の異なる『BiBi』のメンバーカードを3枚」 cost.
//!
//! Text (ab#1, 登場/ライブ開始時):
//!
//!   手札の**名前の異なる**『BiBi』のメンバーカードを3枚控え室に置いてもよい：
//!   自分のステージにいるメンバーが『BiBi』のみの場合、相手のステージにいるメンバー1人を
//!   ウェイトにする。
//!
//! # The mechanic under test
//!
//! 「名前の異なる」 constrains the COST. It is not 「異なるカード」 (distinct card numbers) —
//! the game compares the printed character NAME, so 絢瀬絵里 `PL!-bp3-002-P` and 絢瀬絵里
//! `PL!-bp3-002-R` are the same name and cannot BOTH be paid. A hand holding three BiBi
//! members across only two names can therefore never pay three, because the third card has
//! a name already taken.
//!
//! # What is observed
//!
//! Which cards the cost prompt actually offers — `filtered_indices`, the legally selectable
//! set. That is the rule's whole effect: a duplicate-named card is not offered, so the
//! player cannot select it and cannot reach a 3-card payment they are not entitled to.
//!
//! `pending_choice_option_count()` is used rather than `pending_choice_count()`: the engine
//! sets the latter to `max_select = min(requested, matching)`, so it reads 1 here whether one
//! card or five qualify and could not tell these two boards apart.

use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const NIKO: &str = "PL!-pb2-018-P＋";
const FILLER: &str = "PL!-sd1-010-SD";
/// 絢瀬絵里, two distinct prints that share one NAME.
const ERI_P: &str = "PL!-bp3-002-P";
const ERI_R: &str = "PL!-bp3-002-R";
/// 西木野真姫 and 矢澤にこ — two further names.
const MAKI_P: &str = "PL!-bp3-006-P";
const NIKO_OTHER: &str = "PL!-bp3-009-P";

/// Fire ab#1 (登場/ライブ開始時) with `hand` in p1's hand.
fn fired_with_hand(hand: &[&str]) -> TestGame {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.id(FILLER);
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
    }

    for no in hand {
        let id = game.id(no);
        // Every hand card must really be a BiBi member — the cost filters on both.
        let c = game.db.get_card(id).unwrap();
        assert_eq!(
            c.unit.as_deref(),
            Some("BiBi"),
            "precondition: {no} must be a BiBi member for 「『BiBi』のメンバーカード」"
        );
        game.state.player1.hand.cards.push(id);
    }

    let n = game.id(NIKO);
    game.assert_card_identity(n, NIKO);
    // play_to_stage draws from hand. 矢澤にこ is itself a BiBi member, so it must LEAVE
    // hand before the cost prompt is read — otherwise it would be counted as a candidate.
    game.state.player1.hand.cards.push(n);
    game.give_energy(17); // 矢澤にこ costs 17 to play
    game.play_to_stage(n, MemberArea::Center);
    assert!(
        !game.state.player1.hand.cards.contains(&n),
        "precondition: 矢澤にこ must be on stage, not in hand, or the cost prompt would offer it \
         as one of its own 『BiBi』のメンバーカード"
    );

    let ability_id = {
        let card = game.db.get_card(n).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| {
                let t = a.triggers.as_deref().unwrap_or("");
                t.contains("ライブ開始時") && a.cost.is_some()
            })
            .unwrap_or_else(|| panic!("{NIKO} lacks the 登場/ライブ開始時 discard cost ability"));
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(n).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(n);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(n),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    game
}

/// Advance to the discard-cost card prompt.
///
/// 矢澤にこ has two 登場 abilities, so the appearance first raises a
/// `SelectAutoAbility` ordering prompt (the cost ability is option 1). That ability's cost is
/// 「…置いてもよい」, so a pay/skip gate follows. Only then is the card-selection prompt shown.
fn advance_to_cost_selection(game: &mut TestGame) {
    let mut guard = 0;
    while guard < 12 {
        // A cost that cannot be paid in full is skipped without any prompt, so "nothing
        // pending" is a normal stopping point here, not an error.
        if !game.has_pending_choice() {
            return;
        }
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectAutoAbility { options, .. } => {
                let pick = options
                    .iter()
                    .position(|o| o.ability_text.contains("名前の異なる"))
                    .unwrap_or_else(|| {
                        panic!("expected the 名前の異なる cost ability among the auto options, got {options:?}")
                    });
                game.select_choice_option(pick);
                guard += 1;
            }
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target.contains("pay_optional_cost") =>
            {
                game.select_choice_option(1); // pay the optional cost
                guard += 1;
            }
            _ => return,
        }
    }
}

/// THE claim: a cost that says 「名前の異なる」 cannot be paid with two cards of one name.
///
/// 「名前の異なる」 constrains the COST. It is not 「異なるカード」 (distinct card numbers) —
/// the game compares the printed character NAME, so 絢瀬絵里 `PL!-bp3-002-P` and 絢瀬絵里
/// `PL!-bp3-002-R` are the same name and cannot BOTH be paid. A hand holding three BiBi
/// members across only two names can therefore never satisfy a 3-card 「名前の異なる」 cost.
/// Pay whatever the cost prompt offers (all candidates at once) and report how many cards
/// reached the 控え室.
///
/// A 「…を3枚控え室に置いてもよい」 cost that cannot be paid IN FULL is skipped outright
/// rather than partially paid (Rule 9.4.2.3), so with fewer than three distinct names in
/// hand there may be no prompt at all. That skip is a correct outcome, not a missing one.
fn pay_whole_cost(game: &mut TestGame, expect_prompt: bool) -> usize {
    // Three outcomes, all named. The old `if has_pending_choice()` collapsed the
    // first two, so a cost skipped for being unpayable and a cost left unpaid
    // after a prompt both reported "nothing reached the 控え室" — which is
    // exactly the number the 「名前の異なる」 negatives assert on.
    match game.pending_choice_type() {
        None => {
            assert!(
                !expect_prompt,
                "the cost is payable here, so a prompt must be raised; nothing pending"
            );
            game.state.player1.waitroom.cards.len()
        }
        Some(ty) => {
            assert_eq!(
                ty,
                "SelectCard",
                "this cost is a hand-card discard, so the prompt must be a card selection"
            );
            // The selectable set is `filtered_indices` when the prompt is filtered, and every
            // hand card when it is not. Either way, take all of them: the question is how many
            // cards the engine lets reach the 控え室, not which ones it prefers.
            let offered = game
                .pending_choice_option_count()
                .unwrap_or_else(|| game.state.player1.hand.cards.len());
            let all: Vec<usize> = (0..offered).collect();
            if !all.is_empty() {
                game.select_indices(&all);
            }
            game.state.player1.waitroom.cards.len()
        }
    }
}

/// THE claim: a cost that says 「名前の異なる」 cannot be paid with two cards of one name.
///
/// 「名前の異なる」 constrains the COST. It is not 「異なるカード」 (distinct card numbers) —
/// the game compares the printed character NAME, so 絢瀬絵里 `PL!-bp3-002-P` and 絢瀬絵里
/// `PL!-pb3-002-R` are the same name and cannot BOTH be paid. A hand holding three BiBi
/// members across only two names can therefore never satisfy a 3-card 「名前の異なる」 cost.
#[test]
fn a_duplicate_named_member_cannot_be_paid_for_the_cost() {
    let mut game = fired_with_hand(&[ERI_P, ERI_R, MAKI_P]);
    advance_to_cost_selection(&mut game);

    // Precondition on the claim itself: the two 絵里 prints really are one NAME.
    let (p, r) = (game.id(ERI_P), game.id(ERI_R));
    assert_ne!(p, r, "the two 絢瀬絵里 prints are different card numbers");
    assert_eq!(
        game.db.get_card(p).unwrap().name,
        game.db.get_card(r).unwrap().name,
        "precondition: {ERI_P} and {ERI_R} must share one printed NAME, since 「名前の異なる」 \
         compares names and not card numbers"
    );

    // Two names for a 3-card 「名前の異なる」 cost, so the cost cannot be paid in
    // full: it is either skipped without a prompt or offered and left unpaid. Both
    // are correct, and both must land below 3.
    let paid = pay_whole_cost(&mut game, false);

    assert!(
        paid < 3,
        "名前の異なる『BiBi』のメンバーカードを3枚控え室に置く — the hand holds 絢瀬絵里 twice \
         (ONE name) plus 西木野真姫 (a second name), so a 3-card payment is NOT legal and \
         3 cards must not reach the 控え室. {paid} were paid: the cost carries \
         `distinct: card_name` but the candidate set is not filtered by it, so two prints of \
         the same character satisfy 「名前の異なる」."
    );
}

/// The control: three DIFFERENT names DO pay the full three-card cost.
///
/// Without this, the test above could pass on an engine that simply refuses to ever pay a
/// 3-card cost, or one that never runs this ability at all.
#[test]
fn three_distinct_names_pay_the_full_three_card_cost() {
    let mut game = fired_with_hand(&[ERI_P, MAKI_P, NIKO_OTHER]);
    advance_to_cost_selection(&mut game);
    assert!(
        game.has_pending_choice(),
        "precondition: three distinct names must make the 3-card cost payable, so a prompt \
         is raised"
    );
    let paid = pay_whole_cost(&mut game, true);

    assert_eq!(
        paid,
        3,
        "絢瀬絵里 / 西木野真姫 / 矢澤にこ are three different names, so 手札の名前の異なる\
         『BiBi』のメンバーカードを3枚控え室に置く is fully satisfiable and all 3 must be \
         paid. {paid} were paid instead — capping this hand would be as wrong as allowing \
         the duplicate."
    );
}

/// Two copies of one name alone: the cost is unpayable, so nothing may be discarded.
#[test]
fn two_copies_of_one_name_pay_nothing() {
    let mut game = fired_with_hand(&[ERI_P, ERI_R]);
    advance_to_cost_selection(&mut game);

    // One name for a 3-card 「名前の異なる」 cost, so the cost is skipped outright
    // and no prompt is raised (Rule 9.4.2.3). That absence is the result.
    let paid = pay_whole_cost(&mut game, false);
    assert_eq!(
        paid,
        0,
        "the hand is 絢瀬絵里 twice — a single name — so 手札の名前の異なる…を3枚控え室に\
         置いてもよい cannot be paid in full and is skipped. {paid} cards were discarded, so \
         a 1-name hand satisfied a 「名前の異なる」 cost."
    );
}

/// The constraint is the NAME, not the card number. If 「名前の異なる」 were read as
/// 「異なるカード」, the -P and -R prints would count as two names and all 3 of
/// {ERI_P, ERI_R, MAKI_P} would be payable.
#[test]
fn the_constraint_is_the_name_not_the_card_number() {
    let mut dup = fired_with_hand(&[ERI_P, ERI_R, MAKI_P]);
    advance_to_cost_selection(&mut dup);
    let with_dup_name = pay_whole_cost(&mut dup, false);

    let mut distinct = fired_with_hand(&[ERI_P, MAKI_P, NIKO_OTHER]);
    advance_to_cost_selection(&mut distinct);
    let with_distinct_names = pay_whole_cost(&mut distinct, true);

    assert!(
        with_dup_name < with_distinct_names,
        "名前の異なる compares NAMES. Both hands hold three BiBi members, but the first repeats \
         絢瀬絵里 so it must pay fewer ({with_dup_name}) than the second, which holds three \
         distinct names ({with_distinct_names}). Equal counts mean the rule is comparing card \
         numbers or is not applied at all."
    );
}
