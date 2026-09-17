/// BP07 C9: PL!HS-PR-035-PR 百生吟子 ab#0 (登場).
///
/// 登場：相手の控え室にあるメンバーカードを3枚選び、相手のデッキの下に好きな順番で
/// 置いてもよい。そうした場合、相手のステージにいる元々持つブレードの数が3つ以下の
/// メンバー1人をウェイトにする。
///
/// (Debut) Choose 3 member cards in the OPPONENT's discard and may place them under
/// the OPPONENT's deck in any order. If you do, wait 1 member on the opponent's
/// stage whose ORIGINAL blade count is <= 3.
use crate::helpers::*;
use rabuka_engine::core::types::AbilityTrigger;

const GINKO: &str = "PL!HS-PR-035-PR";
const M1: &str = "PL!-sd1-001-SD";
const M2: &str = "PL!-sd1-003-SD";
const M3: &str = "PL!-sd1-004-SD";

fn trigger_debut(game: &mut TestGame, card_id: i16) {
    let card = game.db.get_card(card_id).unwrap();
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
        Some(card_id),
        None,
        None,
    );
    game.state.activating_card = Some(card_id);
    game.state.process_pending_auto_abilities(&pid);
    game.drain_auto_ability_choices();
}

/// Seed the opponent's deck so its bottom 3 are identifiable, then verify the
/// 3 chosen opponent-discard cards land on the opponent's deck bottom.
#[test]
fn gin_ko_places_opponent_discard_under_opponent_deck() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Opponent discard: 3 member cards to be chosen.
    let a = game.id(M1);
    let b = game.id(M2);
    let c = game.id(M3);
    game.state.player2.waitroom.cards.push(a);
    game.state.player2.waitroom.cards.push(b);
    game.state.player2.waitroom.cards.push(c);

    // Opponent deck with fillers (so we can see the bottom after placement).
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..5 {
        game.state.player2.main_deck.cards.push(filler);
    }
    let p2_deck_before = game.state.player2.main_deck.cards.len();

    // 百生吟子 on P1's stage.
    let gin = game.id(GINKO);
    game.state.player1.stage.stage[1] = gin;
    trigger_debut(&mut game, gin);

    // Accept the optional "置いてもよい" and select the 3 opponent-discard cards.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 12 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // Pay / accept
            }
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                let cnt = game.pending_choice_count();
                let idxs: Vec<usize> = (0..cnt).collect();
                game.select_indices(&idxs);
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }

    let p2_deck = &game.state.player2.main_deck.cards;
    // The 3 chosen cards moved from opponent discard to opponent deck bottom.
    for id in [a, b, c] {
        assert!(
            !game.state.player2.waitroom.cards.contains(&id),
            "chosen card {} should leave the opponent's discard",
            id
        );
        assert!(
            p2_deck.contains(&id),
            "chosen card {} should be on the opponent's deck",
            id
        );
    }
    assert_eq!(
        p2_deck.len(),
        p2_deck_before + 3,
        "3 cards added to the opponent's deck bottom"
    );
}
