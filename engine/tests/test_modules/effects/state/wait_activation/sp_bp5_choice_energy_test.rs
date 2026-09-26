/// Tests for PL!SP-bp5-001-R+ — Choice behind an optional pay_energy cost.
///
/// 登場/ライブ開始時: {{icon_energy.png|E}}支払ってもよい：
///   以下から1つを選ぶ。
///   • 相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
///   • カードを1枚引く。
///
/// Covers: choice + pay_energy (0% coverage)
///
/// Prompt discipline for this card. The real sequence is two prompts, and
/// answering them by position (`select_option(1)` twice) is what let a swapped
/// pair pass unnoticed, so every test below names the prompt it is answering:
///
/// 1. `pay_optional_cost:skip_optional_cost` — option 1 pays, option 0 skips.
/// 2. `choice` — option 0 is the wait line, option 1 is the draw line. This
///    choice carries `options: None`; the index picks the printed line.
///
/// The wait line then resolves by ITSELF when exactly one opponent member is
/// inside the cost filter (one match for a count-1 effect auto-selects), and
/// prompts only when there are two or more. So the filter is proved by the
/// OUTCOME in the single-target fixture and by the OFFERED LIST in the
/// two-target one.
use crate::helpers::*;
use crate::test_modules::support::bp7_wait_immunity_helpers::is_waited;
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

const KANON: &str = "PL!SP-bp5-001-R＋";

/// Pay the optional E and pick `effect_option` (0 = wait an opponent member,
/// 1 = draw a card), asserting the identity of both prompts on the way.
fn pay_and_choose(game: &mut TestGame, effect_option: i16) {
    use rabuka_engine::ability::types::Choice;
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "prompt 1 of 2: the optional cost is offered first"
    );
    match game.get_pending_choice() {
        Choice::SelectTarget {
            target, allow_skip, ..
        } => {
            assert_eq!(
                target, "pay_optional_cost:skip_optional_cost",
                "prompt 1 is the pay-or-skip cost gate"
            );
            assert!(
                *allow_skip,
                "the optional cost must allow skipping"
            );
        }
        other => panic!("prompt 1: expected the cost gate, got {other:?}"),
    }
    game.select_option(1);

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "prompt 2 of 2: the two-option choice comes after the cost is settled"
    );
    match game.get_pending_choice() {
        Choice::SelectTarget {
            target, description, ..
        } => {
            assert_eq!(target, "choice", "prompt 2 is the two-option choice");
            assert!(
                description.contains("ウェイト") && description.contains("引く"),
                "the choice must offer exactly the two printed lines, got {description:?}"
            );
        }
        other => panic!("prompt 2: expected the choice, got {other:?}"),
    }
    game.select_option(effect_option);
}

/// Play the card to the centre with `energy` active energy available.
fn deploy_kanon(game: &mut TestGame, card: i16, energy: usize) {
    game.add_to_hand(card);
    game.give_energy(energy);
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(card),
        None,
        Some(MemberArea::Center),
        Some(false),
    )
    .expect("play to stage");
}

/// Paying the E and taking the draw line puts the DECK CARD in hand.
///
/// Was `assert!(hand.len() >= 1)`, which holds for the played card having
/// bounced back, for the draw having silently not happened at all, and for any
/// other card — the hand was already non-empty from the fixture. The printed
/// effect names a draw, so the card that was on the deck is the thing to look
/// for, and the deck must no longer hold it.
///
/// The opponent's stage is empty here, so the OTHER line has no target at all:
/// this is the case where the draw is the only thing the effect can do.
#[test]
fn sp_bp5_choice_energy_pay_and_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id(KANON);
    game.assert_card_identity(card, KANON);
    let deck_card = game.id("PL!-sd1-010-SD");
    game.assert_card_identity(deck_card, "PL!-sd1-010-SD");
    // One known card on the deck, so "the draw happened" is observable by
    // identity rather than by a count.
    game.state.player1.main_deck.cards.push(deck_card);
    let deck_before = game.state.player1.main_deck.cards.len();

    deploy_kanon(&mut game, card, 15);

    assert!(
        game.state.player2.stage.stage.iter().all(|&c| c == -1),
        "precondition: the opponent's stage is empty, so only the draw is selectable"
    );
    pay_and_choose(&mut game, 1);

    assert!(
        game.player().hand.cards.contains(&deck_card),
        "the draw line must bring the card that was on the deck into hand"
    );
    assert!(
        !game.state.player1.main_deck.cards.contains(&deck_card),
        "the drawn card must have LEFT the deck"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "the draw line is exactly one card"
    );
    assert!(
        !game.has_pending_choice(),
        "the draw resolves on its own; nothing is left to answer"
    );
}

/// Paying the E and taking the wait line waits the OPPONENT member that is
/// inside the cost filter, and leaves the cost-5 member standing next to it.
///
/// One eligible member, so the effect auto-selects and no target prompt appears
/// — the filter is therefore proved by the outcome, which is the stronger claim:
/// a "the first opponent member" implementation would also have waited the
/// cost-4 card in this arrangement, so the cost-5 neighbour is what makes the
/// filter observable at all.
#[test]
fn sp_bp5_choice_energy_pay_and_wait_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id(KANON);
    let cheap = game.id("PL!-sd1-010-SD"); // cost 4 — the filter boundary
    let pricey = game.id("PL!-pb1-021-PR"); // cost 5 — over the cap
    game.assert_card_cost(cheap, 4);
    game.assert_card_cost(pricey, 5);
    game.state.player2.stage.stage = [cheap, pricey, -1];

    let d1 = game.id("PL!-sd1-010-SD");
    game.state.player1.main_deck.cards.push(d1);

    deploy_kanon(&mut game, card, 15);

    pay_and_choose(&mut game, 0);
    assert!(
        !game.has_pending_choice(),
        "one eligible member and a count-1 effect resolve without a prompt"
    );

    assert_eq!(
        game.state.mods.get_orientation_modifier(cheap),
        Some("wait"),
        "the cost-4 opponent member must be waited"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(pricey),
        None,
        "the cost-5 member is out of the filter's range and must be untouched"
    );
    assert!(
        !game.player().hand.cards.contains(&d1),
        "the wait line was chosen, so no card was drawn"
    );
    assert_eq!(
        game.state.player2.stage.stage,
        [cheap, pricey, -1],
        "a wait never moves a member between areas"
    );
}

/// With two members inside the filter the effect must ask which one, and the
/// cost-5 member must not be on offer at all.
///
/// This is the fixture where the filter is observable as a LIST: "3 options
/// offered" would hold if the cost-5 card were included, so the count is
/// asserted against the printed filter and then the RESULT is pinned — exactly
/// one of the two eligible members ends up waited, never the third.
#[test]
fn sp_bp5_choice_energy_waits_only_members_at_or_below_cost_four() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id(KANON);
    let cheap4 = game.id("PL!-sd1-010-SD"); // cost 4 — the boundary, inside
    let cheap2 = game.id("PL!-sd1-002-SD"); // cost 2 — inside
    let pricey5 = game.id("PL!-pb1-021-PR"); // cost 5 — outside
    game.assert_card_cost(cheap4, 4);
    game.assert_card_cost(cheap2, 2);
    game.assert_card_cost(pricey5, 5);
    game.state.player2.stage.stage = [cheap4, cheap2, pricey5];

    deploy_kanon(&mut game, card, 15);
    pay_and_choose(&mut game, 0);

    // Two eligible members, so a count-1 wait has to ask which one. With more
    // than one candidate the engine offers a CARD selection, not a target list.
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "with two members inside the cost filter, the wait must ask which one"
    );
    game.select_indices(&[0]);

    assert!(
        is_waited(&game, cheap4) || is_waited(&game, cheap2),
        "one of the two eligible members must be waited"
    );
    assert_eq!(
        (is_waited(&game, cheap4) as u8) + (is_waited(&game, cheap2) as u8),
        1,
        "the wait line is exactly one member, not all eligible ones"
    );
    assert!(
        !is_waited(&game, pricey5),
        "the cost-5 member must never be waited, whatever the player picked"
    );
}

/// Declining the optional cost with a valid target present changes nothing: the
/// cost is optional in full, so the skip must not fall through into either line.
/// Energy, hand and the opponent's board are all asserted, because "nothing
/// happened" is exactly those three.
#[test]
fn sp_bp5_choice_energy_decline_cost_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id(KANON);
    let opponent = game.id("PL!-sd1-010-SD");
    let deck_card = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage[1] = opponent;
    game.state.player1.main_deck.cards.push(deck_card);

    // Enough to play the card AND to pay the E, so the only reason nothing
    // happens is declining — not a shortage of energy.
    deploy_kanon(&mut game, card, 15);
    let energy_after_play = game.state.player1.energy_zone.active_count();

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "even with the energy available the cost is optional, so it is offered"
    );
    game.select_option(0); // skip
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_after_play,
        "declining the optional cost must not spend the E"
    );
    assert!(
        !game.player().hand.cards.contains(&deck_card),
        "declining the cost resolves the whole ability: no draw"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent),
        None,
        "declining the cost resolves the whole ability: no wait"
    );
    assert!(
        !game.state.player1.waitroom.cards.contains(&deck_card),
        "declining the cost must not discard anything either — the card on top \
         leaves the deck through the turn's own Draw, not through this ability"
    );
}

/// Taking the DRAW line while an eligible opponent member sits on stage must
/// not also wait it: the two lines are exclusive.
///
/// The sibling test covers the wait branch; this one exists because a "choose
/// draw" implementation that also applied the wait would satisfy every
/// count-based assertion in this file.
#[test]
fn sp_bp5_choice_energy_pay_and_choose_draw() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id(KANON);
    let opponent = game.id("PL!-sd1-010-SD");
    let deck_card = game.id("PL!-sd1-010-SD");
    game.assert_card_cost(opponent, 4);
    game.state.player2.stage.stage[1] = opponent;
    game.state.player1.main_deck.cards.push(deck_card);

    deploy_kanon(&mut game, card, 15);

    pay_and_choose(&mut game, 1);

    assert!(
        game.player().hand.cards.contains(&deck_card),
        "the draw line must bring the deck card to hand"
    );
    assert!(!game.state.player1.main_deck.cards.contains(&deck_card));
    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent),
        None,
        "the two lines are exclusive: taking the draw must not also wait the \
         member, even though a cost-4 target was available"
    );
    assert!(
        !game.has_pending_choice(),
        "the draw line opens no further prompt"
    );
}

/// The same exclusion from the other side: declining the payment must not draw
/// either, and the deck card must still be in the deck afterwards.
#[test]
fn sp_bp5_choice_energy_decline_payment_has_no_follow_up_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let card = game.id(KANON);
    let opponent = game.id("PL!-sd1-010-SD");
    let deck_card = game.id("PL!-sd1-010-SD");
    game.state.player2.stage.stage[1] = opponent;
    game.state.player1.main_deck.cards.push(deck_card);

    deploy_kanon(&mut game, card, 15);
    let energy_after_play = game.state.player1.energy_zone.active_count();
    let hand_before = game.player().hand.cards.len();

    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectTarget"),
        "the pay-or-skip gate comes first"
    );
    game.select_option(0); // skip the cost
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent),
        None,
        "declining the cost must not wait the opponent member"
    );
    assert_eq!(
        game.player().hand.cards.len(),
        hand_before,
        "declining the cost must not draw"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_after_play,
        "declining the optional cost must not spend the E"
    );
    assert!(
        game.state.player1.main_deck.cards.contains(&deck_card),
        "the deck card is still on the deck, not consumed by the skip"
    );
    assert!(!game.player().hand.cards.contains(&deck_card));
}
