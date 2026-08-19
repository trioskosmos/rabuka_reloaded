/// Tests for PL!HS-bp2-018-N (日下あすか):
///
/// 登場 自分のメインフェイズの場合、EE支払ってもよい：
/// 自分の控え室からライブカードを1枚、表向きでライブカード置き場に置く。
/// 次のライブカードセットフェイズで自分がライブカード置き場に置けるカード枚数の上限が1枚減る。
///
/// Key mechanics:
/// - Trigger: 登場 (debut) — only fires when played from hand to stage
/// - Gate: temporal_condition — only during own main phase
/// - Cost: EE optional payment
/// - Effect: sequential — (1) move live_card from discard → live_card_zone,
///   (2) reduce_live_card_set_limit by 1
/// - Similar to DIVE! ab#0 but source is discard (not hand)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const HS_BP2_018: &str = "PL!HS-bp2-018-N";
const FILLER: &str = "PL!-sd1-010-SD";
const LIVE_A: &str = "PL!-sd1-019-SD";

fn fill_deck(game: &mut TestGame, player: &str, count: usize) {
    let ids: Vec<i16> = (0..count).map(|_| game.id(FILLER)).collect();
    let deck = if player == "p1" {
        &mut game.state.player1.main_deck.cards
    } else {
        &mut game.state.player2.main_deck.cards
    };
    for f in ids {
        deck.push(f);
    }
}

/// Helper: set up a game where HS-bp2-018 is in hand, a live card is in discard,
/// and we're ready to play it.
fn setup() -> (TestGame, i16, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(HS_BP2_018);
    let live = game.id(LIVE_A);
    let filler = game.id(FILLER);

    // Member in hand
    game.state.player1.hand.cards.push(member);

    // Live card in discard (the card we want to move to live_card_zone)
    game.state.player1.waitroom.cards.push(live);

    // Filler cards for decks
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);

    // Energy for cost
    game.give_energy(10);

    (game, member, live)
}

/// 登場時にメインフェイズでEEを支払い、控え室のライブカードをライブカード置き場に置き、
/// ライブカードセット上限が1減る。
#[test]
fn debut_main_phase_pay_energy_moves_live_card_and_reduces_limit() {
    let (mut game, member, live) = setup();

    game.play_to_stage(member, MemberArea::Center);

    // Debut ability fires — should present a choice for the live card in discard
    // (optional cost → then move card selection)
    // Handle optional energy cost
    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_option(1); // Yes, pay the cost
            }
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "discard" =>
            {
                // Select the live card from discard
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    // Live card should now be in live_card_zone
    assert!(
        game.state.player1.live_card_zone.cards.contains(&live),
        "Live card should be in live_card_zone after ability resolves"
    );

    // Live card should no longer be in discard
    assert!(
        !game.state.player1.waitroom.cards.contains(&live),
        "Live card should be removed from discard"
    );

    // live_card_set_limit_reduction should be 1
    assert_eq!(
        game.state.player1.live_card_set_limit_reduction, 1,
        "live_card_set_limit_reduction should be 1 after ability"
    );
}

/// 登場時にメインフェイズでEE支払いをスキップ → 効果なし。
#[test]
fn debut_main_phase_skip_energy_no_effect() {
    let (mut game, member, live) = setup();

    game.play_to_stage(member, MemberArea::Center);

    // Ability fires — decline the optional cost
    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_option(0); // No, skip the cost
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    // Live card should still be in discard
    assert!(
        game.state.player1.waitroom.cards.contains(&live),
        "Live card should remain in discard when cost is skipped"
    );

    // live_card_set_limit_reduction should be 0
    assert_eq!(
        game.state.player1.live_card_set_limit_reduction, 0,
        "live_card_set_limit_reduction should be 0 when cost is skipped"
    );
}

/// 控え室にライブカードがない場合、コストを払ってもmove_cardsは無効だが、
/// reduce_live_card_set_limitはsequential blockとして依然実行される。
/// これはengineのsequential実装の動作：各actionは独立に実行され、
/// 前のactionの成否に関わらず実行される。
#[test]
fn debut_main_phase_no_live_cards_in_discard_still_reduces_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(HS_BP2_018);
    let filler = game.id(FILLER);

    game.state.player1.hand.cards.push(member);
    // No live cards in discard — only filler members
    game.state.player1.waitroom.cards.push(filler);
    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    game.give_energy(10);

    game.play_to_stage(member, MemberArea::Center);

    // Handle all pending choices
    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_option(1); // Pay cost
            }
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "discard" =>
            {
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    // Filler should still be in discard (not moved — it's not a live card)
    assert!(
        !game.state.player1.live_card_zone.cards.contains(&filler),
        "Filler member should NOT be placed in live_card_zone (only live cards)"
    );

    // reduce_live_card_set_limit fires as part of the sequential block
    assert_eq!(
        game.state.player1.live_card_set_limit_reduction, 1,
        "limit reduction fires even when move_cards found no valid target (sequential)"
    );
}

/// 控え室にライブカードが複数枚ある場合、どれか1枚を選択する。
#[test]
fn debut_main_phase_multiple_live_cards_in_discard_select_one() {
    let (mut game, member, _live) = setup();
    let live2 = game.id(LIVE_A);

    // Add a second live card to discard
    game.state.player1.waitroom.cards.push(live2);

    game.play_to_stage(member, MemberArea::Center);

    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_option(1); // Pay
            }
            rabuka_engine::ability::types::Choice::SelectCard { count, .. } if *count == 1 => {
                // Select first live card from discard
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    // Exactly 1 live card should be in live_card_zone
    assert_eq!(
        game.state.player1.live_card_zone.cards.len(),
        1,
        "Exactly 1 live card should be in live_card_zone, got {}",
        game.state.player1.live_card_zone.cards.len()
    );

    // Limit reduction should be exactly 1
    assert_eq!(
        game.state.player1.live_card_set_limit_reduction, 1,
        "limit reduction should be 1"
    );
}

/// ライブカードセット上限の減少が正しいことを確認:
/// 元々の上限は3。能力使用後は2になる。
#[test]
fn limit_reduction_affects_live_card_set_count() {
    let (mut game, member, _live) = setup();

    game.play_to_stage(member, MemberArea::Center);

    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match &choice {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_option(1);
            }
            rabuka_engine::ability::types::Choice::SelectCard { zone, .. }
                if zone == "discard" =>
            {
                game.select_indices(&[0]);
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    // Verify: default limit 3, reduction 1, effective limit 2
    let reduction = game.state.player1.live_card_set_limit_reduction;
    let effective_limit = 3i32 - reduction as i32;
    assert_eq!(
        effective_limit, 2,
        "Effective live card set limit should be 2 (3 - 1), got {}",
        effective_limit
    );
}
