//! 南ことり `PL!-pb2-012-P＋` / `-R` — 起動 (ab#1), the additional-cost CHOICE
//!
//! {{kidou.png|起動}}{{turn1.png|ターン1回}}
//! このメンバーをウェイトにする：この能力を起動するための**追加コストとして、手札を2枚控え室に
//! 置くか**、**『Printemps』のメンバーを2人ウェイトにする**。
//! 自分の控え室から『Printemps』のライブカードを1枚手札に加える。
//!
//! In English: once per turn — wait this member: as an ADDITIONAL cost for
//! activating, either put 2 cards from hand into the discard OR wait 2 Printemps
//! members. Recover 1 Printemps live card from the discard to your hand.
//!
//! # Why this file exists: the 「か」 is a CHOICE (parser defect D3)
//!
//! 「この能力を起動するための追加コストとして」 was not recognised as a cost at all. The
//! 「：」 convention puts everything after it in the effect, so the whole clause landed in
//! the effect as the first leg of a `sequential`, which is wrong twice over:
//!
//! * it was not a cost, so it ran as an EFFECT STEP — after the card's own cost, and
//!   outside the activation's payment rules, rather than as part of paying for it;
//! * its 「か」 became a two-step SEQUENCE, so the player had to discard 2 cards AND wait
//!   2 members to activate, instead of either one.
//!
//! The fix routes the clause into `cost` and gives the 「か」 the corpus's own
//! multi-cost shape: a `sequential_cost` whose second leg is a `choice_condition`. The
//! engine already prompts for that via `ChoiceRoute::ChoiceCost`.
//!
//! # The recover step cannot be exercised with this card pool
//!
//! 「自分の控え室から『Printemps』のライブカードを1枚手札に加える」 asks for a Printemps
//! LIVE card, and the pool has none: every `-SRL` live card is μ's or Liella! (verified
//! against the card database). So this file pins the payment — which is what D3 was
//! about — and deliberately asserts nothing about the recovered card. Asserting it
//! would mean staging a card that does not exist.

use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::zones::MemberArea;

const KOTORI: &str = "PL!-pb2-012-P＋";
const FILLER: &str = "PL!-sd1-010-SD";
/// Two differently-named Printemps members, so the wait branch has two real targets.
const P_HANAYOME: &str = "PL!-bp6-017-N"; // 小泉花陽
const P_KOTORI2: &str = "PL!-bp5-012-N"; // 南 ことり (a second copy)

/// Board: Kotori in the centre, two Printemps allies in the side areas, and `hand`
/// filler cards. Returns the game plus the ids the assertions read.
///
/// Kotori is staged by hand rather than played, so no energy is spent and no other
/// trigger fires. The 2 hand cards are not incidental: rule 9.2 restricts an
/// activation that pays its own cost to having 2+ cards in hand.
fn board(hand: usize) -> (TestGame, i16, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let filler = game.new_id(FILLER);
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }
    let kotori = game.id(KOTORI);
    game.assert_card_identity(kotori, KOTORI);
    let hanayome = game.id(P_HANAYOME);
    let kotori2 = game.id(P_KOTORI2);
    game.assert_card_in_group(hanayome, "Printemps", "ally fixture");
    game.assert_card_in_group(kotori2, "Printemps", "ally fixture");
    game.state.player1.stage.stage[MemberArea::LeftSide as usize] = hanayome;
    game.state.player1.stage.stage[MemberArea::RightSide as usize] = kotori2;
    game.state.player1.stage.stage[MemberArea::Center as usize] = kotori;
    for _ in 0..hand {
        game.state.player1.hand.cards.push(filler);
    }
    game.state.recalculate_constants();
    (game, kotori, hanayome, kotori2)
}

/// The two payment options the engine is offering, in printed order.
///
/// `pending_choice_summary` renders a `SelectTarget` as only `target=…`, and
/// `drain_choices_strict` has no auto-answer for one, so both the reading and
/// the answering of this prompt are explicit here.
fn cost_option_texts(game: &TestGame) -> Vec<String> {
    match game.get_pending_choice() {
        Choice::SelectTarget {
            target, options, ..
        } => {
            assert_eq!(
                target, "choice_condition",
                "the prompt must be the cost-option route, not some other SelectTarget"
            );
            options.clone().unwrap_or_default()
        }
        other => panic!("expected the additional-cost choice, got {other:?}"),
    }
}

/// Activate, assert the payment is a choice, and answer it with `option`.
/// Returns the option texts that were offered.
fn pay_with(game: &mut TestGame, kotori: i16, option: i16) -> Vec<String> {
    game.activate_ability(kotori);
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "「手札を2枚控え室に置く**か**、『Printemps』のメンバーを2人ウェイトにする」 — the \
         additional cost is a choice between two payments, so it must be put to the player \
         as an option prompt. Before the fix the 「か」 was a `sequential` inside the effect, \
         so no prompt was raised and the player had to pay BOTH."
    );
    let options = cost_option_texts(game);
    assert_eq!(
        options.len(),
        2,
        "exactly the two printed payment options: {options:?}"
    );
    assert!(
        options[0].contains("控え室") && options[1].contains("ウェイト"),
        "the prompt must name both payment options so the choice is legible: {options:?}"
    );
    game.select_option(option);
    // The discard branch pays by asking WHICH cards leave the hand, so it raises a
    // SelectCard of its own. The wait branch raises nothing here, and
    // `drain_choices_strict` no-ops on an empty queue — so this covers both without
    // a guard, and still fails on an unexpected prompt.
    game.drain_choices_strict(&["SelectCard"], &[0, 1]);
    game.drain_auto_ability_choices();
    options
}

/// Activate her 起動 and assert the payment is OFFERED AS A CHOICE.
///
/// This is the assertion D3 needed and did not have. Before the fix there was no
/// prompt at all: the two payment options were effect steps, so activation either
/// silently did both or did nothing, and nothing in the old shape said which.
#[test]
fn the_additional_cost_is_offered_as_a_choice_not_a_sequence() {
    let (mut game, kotori, _, _) = board(2);
    let options = pay_with(&mut game, kotori, 0);
    assert!(
        options.iter().any(|o| o.contains("控え室")) && options.iter().any(|o| o.contains("ウェイト")),
        "both printed payments are on the table, in order: {options:?}"
    );
}

/// Choose the HAND branch: 2 cards leave the hand for the discard, and no ally is
/// waited.
///
/// 「not both」 is the load-bearing half. A `sequential` payment would have emptied
/// two hand cards AND waited the two Printemps allies; only the ally orientations
/// distinguish the two readings, because Kotori herself is waited either way — that
/// is the DECLARED cost (「このメンバーをウェイトにする」), not part of the choice.
#[test]
fn paying_with_the_hand_branch_waits_nobody() {
    let (mut game, kotori, hanayome, kotori2) = board(2);
    let hand_before = game.state.player1.hand.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    pay_with(&mut game, kotori, 0);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before - 2,
        "「手札を2枚控え室に置く」 — paying by hand removes exactly 2 cards from hand"
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before + 2,
        "…they go to the 控え室, and nowhere else"
    );
    for (who, name) in [(hanayome, "小泉花陽"), (kotori2, "南 ことり")] {
        assert_eq!(
            game.state.mods.get_orientation_modifier(who),
            None,
            "{name} must not be waited when the HAND branch is paid — 「か」 is a choice, so \
             choosing the discard obliges nothing else"
        );
    }
    assert_eq!(
        game.state.mods.get_orientation_modifier(kotori),
        Some("wait"),
        "Kotori herself is waited either way: that is the DECLARED cost \
         「このメンバーをウェイトにする」, which is a separate step from the additional-cost \
         choice. This is what a `sequential` reading of the additional cost would also \
         produce, which is why the ally orientations above carry the real check."
    );
}

/// Choose the WAIT branch: the 2 Printemps allies are waited, and no card leaves hand.
///
/// The mirror image, and the other half of "either". Only the hand assertions
/// distinguish it from the discard branch.
#[test]
fn paying_with_the_wait_branch_discards_nothing() {
    let (mut game, kotori, hanayome, kotori2) = board(2);
    let hand_before = game.state.player1.hand.cards.len();
    let discard_before = game.state.player1.waitroom.cards.len();

    pay_with(&mut game, kotori, 1);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "『Printemps』のメンバーを2人ウェイトにする — the wait branch costs no cards, so hand \
         is untouched. A `sequential` payment would have emptied two cards here."
    );
    assert_eq!(
        game.state.player1.waitroom.cards.len(),
        discard_before,
        "…and the 控え室 is untouched for the same reason"
    );
    assert_eq!(
        (
            game.state.mods.get_orientation_modifier(hanayome),
            game.state.mods.get_orientation_modifier(kotori2),
        ),
        (Some("wait"), Some("wait")),
        "「2人ウェイト」 — both Printemps allies are waited, and both are real targets because \
         they are differently named members"
    );
}

/// The parse must keep the choice in `cost`, as a `choice_condition`.
///
/// Pinned structurally so D3 cannot return as effect legs. Asserting the shape rather
/// than only the behaviour keeps a silent regression — a re-parse that moves the
/// choice back into the effect while still happening to look right at runtime — from
/// passing on the behaviour tests alone.
#[test]
fn the_additional_cost_parses_as_a_choice_condition_inside_cost() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let cost = {
        let card = game.db.get_card(kotori).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("起動"))
            .expect("Kotori has a 起動");
        ab.cost.as_ref().expect("起動 has a cost").clone()
    };

    let root = cost.compound.actions.as_ref().expect("costs are a list");
    assert_eq!(
        root.len(),
        2,
        "the declared cost 「このメンバーをウェイトにする」 and the additional cost are two \
         separate payments: {cost:?}"
    );
    assert_eq!(
        root[0].state_change_any(),
        Some("wait"),
        "step 1 is the declared self-wait"
    );

    let additional = &root[1];
    let options = additional.compound.actions.as_ref().expect("the choice has options");
    assert_eq!(
        options.len(),
        2,
        "step 2 is the additional cost, and it has TWO options — 「…置く**か**、…ウェイトに\
         する」. It must not be a single cost, nor two sequential steps: {additional:?}"
    );
    let first_text = options[0].text.as_ref();
    let second_text = options[1].text.as_ref();
    assert!(
        first_text.contains("控え室") && second_text.contains("ウェイト"),
        "the two options are the discard-2 and wait-2 payments, in printed order: \
         {first_text:?} / {second_text:?}"
    );
    assert_eq!(options[0].count_any(), Some(2), "「手札を2枚」");
    assert_eq!(options[1].count_any(), Some(2), "「2人ウェイト」");
}

/// The ability's own effect must be only the recover step.
///
/// The two payment options used to BE the effect, as the first two legs of a
/// `sequential`. If they ever reappear here the choice has been demoted back to an
/// effect step, which is the original defect.
#[test]
fn the_effect_is_only_the_recover_step() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let kotori = game.id(KOTORI);
    let eff = {
        let card = game.db.get_card(kotori).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("起動"))
            .expect("Kotori has a 起動");
        ab.effect.as_ref().expect("起動 has an effect").clone()
    };
    assert!(
        eff.compound.actions.is_none(),
        "「自分の控え室から『Printemps』のライブカードを1枚手札に加える」 is a single step, so \
         the effect must not be a sequence. It was a `sequential` whose first legs were the \
         two payment options — the payment had leaked into the effect: {eff:?}"
    );
    assert_eq!(eff.count_any(), Some(1), "「ライブカードを1枚」");
}
