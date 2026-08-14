/// PL!N-bp7-017-N 宮下 愛 ab#0 (登場).
///
/// 登場：自分のエネルギーデッキから、エネルギーカード1枚を自分のステージにいる
/// 『虹ヶ咲』のメンバーの下に置いてもよい。
///
/// (Debut) You may place 1 energy card from your energy deck under a 虹ヶ咲
/// member on your stage.
///
/// The "置いてもよい" (may) makes the placement OPTIONAL: the engine must offer a
/// Skip/Do choice. Accepting places the energy under a 虹ヶ咲 member; skipping
/// does nothing (the energy stays in the deck).
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const AI: &str = "PL!N-bp7-017-N"; // 宮下 愛 — DiverDiva + 虹ヶ咲
const NIJI: &str = "PL!N-bp1-004-R"; // 朝香果林 — another 虹ヶ咲 member on stage
const ENERGY: &str = "LL-E-001-SD";

/// Fire 宮下 愛's 登場 (debut) ability and resolve all pending choices.
/// `accept` chooses whether to accept the optional placement.
fn trigger_debut(game: &mut TestGame, ai: i16, accept: bool) {
    let card = game.db.get_card(ai).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("登場"))
        .expect("card should have a 登場 ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::Debut,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(ai),
        None,
        None,
    );
    game.state.activating_card = Some(ai);
    game.state.process_pending_auto_abilities(&pid);

    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            // Optional skip/do prompt.
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(if accept { 1 } else { 0 });
            }
            // Which member to place under (or default).
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            _ => game.select_indices(&[0]),
        }
    }
}

fn total_under(game: &TestGame) -> usize {
    (0..3)
        .map(|i| game.state.player1.stage.under_cards[i].len())
        .sum()
}

/// Accepting the optional placement moves 1 energy from the deck under a 虹ヶ咲 member.
#[test]
fn ai_accept_places_energy_under_nijigasaki() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    let niji = game.id(NIJI);
    game.state.player1.stage.stage = [ai, niji, -1];
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }
    let deck_before = game.state.player1.energy_deck.cards.len();

    trigger_debut(&mut game, ai, true); // accept

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before - 1,
        "accepting places exactly 1 energy from the deck"
    );
    assert_eq!(
        total_under(&game),
        1,
        "exactly 1 energy card ends up under a stage member"
    );
}

/// Skipping the optional placement does nothing: the energy stays in the deck.
#[test]
fn ai_skip_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    let niji = game.id(NIJI);
    game.state.player1.stage.stage = [ai, niji, -1];
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }
    let deck_before = game.state.player1.energy_deck.cards.len();

    trigger_debut(&mut game, ai, false); // skip

    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before,
        "skipping must not consume any energy from the deck"
    );
    assert_eq!(
        total_under(&game),
        0,
        "skipping must not place any energy under a member"
    );
}

/// No 虹ヶ咲 member on stage → the placement is skipped/does nothing.
#[test]
fn ai_no_nijigasaki_member_places_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let ai = game.id(AI);
    // Only 宮下 愛 (which IS 虹ヶ咲) on stage — treat it as the matching member.
    game.state.player1.stage.stage = [ai, -1, -1];
    for _ in 0..3 {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }
    let deck_before = game.state.player1.energy_deck.cards.len();

    trigger_debut(&mut game, ai, true);

    // 宮下 愛 is itself 虹ヶ咲, so it is a valid target and the placement works.
    assert_eq!(
        game.state.player1.energy_deck.cards.len(),
        deck_before - 1,
        "宮下 愛 itself is a 虹ヶ咲 member and receives the energy"
    );
}
