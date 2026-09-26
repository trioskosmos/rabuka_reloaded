/// 鬼塚夏美 PL!SP-bp5-020-N ab#0 (ライブ成功時):
///
/// {{live_success.png/ライブ成功時}}{{icon_energy.png/E}}支払ってもよい：
/// カードを1枚引く。
///
/// And her sibling case, 平安名すみれ PL!SP-pb1-004-R ab#1, the same effect at
/// E E E.
///
/// These tests used to assert only that the deck got smaller after seven blind
/// `pass()` calls. A turn draws on its own, so that assertion held whether or
/// not the printed effect ever fired — it could not fail. Every test below
/// fires ライブ成功時 explicitly and measures the ability's OWN delta: the
/// exact number of cards drawn, and the exact energy paid.
use crate::helpers::*;
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;

/// Stage three members so the live can succeed, put `live_no` in the live zone,
/// and hand out `energy` active energy. The deck is filled and its length
/// returned so the caller can diff it.
fn setup_live(live_no: &str, energy: usize) -> (TestGame, i16, usize) {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    let stage_member = game.new_id("PL!-sd1-001-SD");
    let stage_member_copy2 = game.new_id("PL!-sd1-001-SD");
    let stage_member_copy3 = game.new_id("PL!-sd1-001-SD");
    game.state.player1.stage.stage = [stage_member, stage_member_copy2, stage_member_copy3];
    let live = game.id(live_no);
    game.assert_card_identity(live, live_no);
    game.state.player1.hand.cards.push(live);
    game.give_energy(energy);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(live);
    let deck_len = game.state.player1.main_deck.cards.len();
    (game, live, deck_len)
}

/// Fire ライブ成功時 and answer the E 支払ってもよい prompt. `pay` picks the
/// payment option; declining is index 0. The prompt is a bare pay/skip toggle
/// (no options list), so the assertion is on `allow_skip` — the printed てもよい
/// — and on the description naming the draw it is buying.
fn fire_live_success_and_answer(game: &mut TestGame, live: i16, pay: bool) {
    fire_trigger(game, live, AbilityTrigger::LiveSuccess, "ライブ成功時");
    if !game.has_pending_choice() {
        return;
    }
    match game.get_pending_choice() {
        Choice::SelectTarget {
            allow_skip,
            description,
            ..
        } => {
            assert!(
                *allow_skip,
                "E 支払ってもよい — the cost must be skippable, got {description:?}"
            );
            assert!(
                !description.is_empty(),
                "the pay/skip prompt must say what it is paying for"
            );
        }
        other => panic!("expected a pay/skip SelectTarget, got {other:?}"),
    }
    game.select_option(if pay { 1 } else { 0 });
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

/// EEE live: paying draws exactly 1 and spends exactly 3 energy.
#[test]
fn three_energy_live_success_draw_pays_three_and_draws_one() {
    let (mut game, live, deck_before) = setup_live("PL!SP-pb1-004-R", 6);
    let energy_before = game.state.player1.energy_zone.active_count();

    fire_live_success_and_answer(&mut game, live, true);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before - 3,
        "E E E 支払ってもよい — exactly three energy were spent"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "カードを1枚引く — exactly one card left the deck"
    );
    assert!(!game.has_pending_choice(), "the effect must resolve cleanly");
}

/// E live: paying draws exactly 1 and spends exactly 1 energy.
#[test]
fn one_energy_live_success_draw_pays_one_and_draws_one() {
    let (mut game, live, deck_before) = setup_live("PL!SP-bp5-020-N", 5);
    let energy_before = game.state.player1.energy_zone.active_count();

    fire_live_success_and_answer(&mut game, live, true);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before - 1,
        "E 支払ってもよい — exactly one energy was spent"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 1,
        "カードを1枚引く — exactly one card left the deck"
    );
}

/// 支払ってもよい: declining spends nothing and draws nothing. The deck length
/// must be UNCHANGED here — that is the assertion the old version could not
/// make, because a plain turn draw also shrinks the deck.
#[test]
fn one_energy_live_success_declined_spends_nothing_and_draws_nothing() {
    let (mut game, live, deck_before) = setup_live("PL!SP-bp5-020-N", 5);
    let energy_before = game.state.player1.energy_zone.active_count();

    fire_live_success_and_answer(&mut game, live, false);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        energy_before,
        "declining 支払ってもよい must not spend energy"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declining 支払ってもよい must not draw — nothing left the deck at all"
    );
    assert!(!game.has_pending_choice(), "no prompt may be left dangling");
}

/// With 0 active energy the cost cannot be paid: the effect must resolve
/// without spending, without drawing, and without offering an unpayable option.
#[test]
fn one_energy_live_success_with_zero_energy_neither_pays_nor_draws() {
    let (mut game, live, deck_before) = setup_live("PL!SP-bp5-020-N", 0);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "setup guard: the zone really is empty"
    );

    // Try to pay anyway: whatever the engine does, energy must not go negative
    // and the deck must not shrink by the effect.
    fire_trigger(
        &mut game,
        live,
        AbilityTrigger::LiveSuccess,
        "ライブ成功時",
    );
    while game.has_pending_choice() {
        game.select_option(1);
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "no energy exists, so none can be spent and the count cannot go negative"
    );
    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "with no energy to pay, カードを1枚引く must not happen"
    );
}
