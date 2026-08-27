/// Q280 — 米女メイ PL!SP-bp7-007-R＋ (ライブ成功時 energy-placement).
///
/// ab#1: 自分のエネルギーデッキから、エネルギーカードを2枚ウェイト状態で置く。
///       それらのエネルギーカードは、次のターンのアクティブフェイズにアクティブしない。
///
/// Official QA Q280:
///   「アクティブしないエネルギーをこのメンバーのライブ成功時能力でアクティブにしました。
///    その後、コストの支払いでそのエネルギーをウェイトにしたとき、次のターンどうなりますか？」
///   → 「アクティブしない効果は継続しているため、アクティブフェイズにアクティブしません。」
///
/// rule: the "doesn't activate next active phase" flag is keyed to the placed **energy**
/// card (not the member), survives that energy being activated again and re-waited, and is
/// honoured at the owner's next active phase even though an opponent's turn comes in between.
use crate::helpers::*;
use rabuka_engine::core::types::{AbilityTrigger, Phase, TurnPhase};
use rabuka_engine::turn::TurnEngine;

const MEI: &str = "PL!SP-bp7-007-R＋"; // 米女メイ
const ENERGY: &str = "LL-E-001-SD";

fn seed_energy_deck(game: &mut TestGame, count: usize) {
    for _ in 0..count {
        game.state.player1.energy_deck.cards.push(game.id(ENERGY));
    }
}

fn fill_main_deck(game: &mut TestGame) {
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
}

fn wait_energy_count(game: &TestGame) -> usize {
    let z = &game.state.player1.energy_zone;
    z.cards.len().saturating_sub(z.active_count() as usize)
}

/// Fire 米女メイ's ab#1 (ライブ成功時 → place 2 energy in WAIT, flag them).
fn trigger_mei_live_success(game: &mut TestGame, mei: i16) {
    let card = game.db.get_card(mei).unwrap();
    let ab = card
        .resolved_abilities()
        .find(|a| a.triggers.as_deref() == Some("ライブ成功時"))
        .unwrap();
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        AbilityTrigger::LiveSuccess,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(mei),
        None,
        None,
    );
    game.state.activating_card = Some(mei);
    game.state.process_pending_auto_abilities(&pid);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
}

// ====================================================================
// Fix #1: the "does not activate next phase" flag is keyed to the placed
// ENERGY cards, NOT to the member whose ability placed them.
// ====================================================================

#[test]
fn q280_live_success_flags_placed_energy_not_member() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let mei = game.id(MEI);
    game.state.player1.stage.stage = [-1, mei, -1];
    seed_energy_deck(&mut game, 6);
    fill_main_deck(&mut game);

    let wait_before = wait_energy_count(&game);
    let zone_before = game.state.player1.energy_zone.cards.len();

    trigger_mei_live_success(&mut game, mei);

    // 2 energy moved: deck → zone, placed in WAIT.
    assert_eq!(
        game.state.player1.energy_zone.cards.len(),
        zone_before + 2,
        "ab#1 should place 2 energy from the energy deck into the energy zone"
    );
    assert_eq!(
        wait_energy_count(&game),
        wait_before + 2,
        "the 2 placed energy must be in WAIT state"
    );

    // Both placed energy cards carry the do-not-active flag; the member does not.
    let zone_cards = game.state.player1.energy_zone.cards.clone();
    let placed: Vec<i16> = zone_cards[zone_cards.len() - 2..].to_vec();
    for c in &placed {
        assert!(
            game.state.mods.is_delayed_cannot_active(*c),
            "Q280: placed energy {} should carry the next-phase do-not-activate flag",
            *c
        );
    }
    assert!(
        !game.state.mods.is_delayed_cannot_active(mei),
        "Q280: the flag must be keyed to the placed ENERGY, not the ability's member"
    );
}

// ====================================================================
// Fix #2 + #3: the owner's active phase activates all energy EXCEPT the
// flagged ones, and an opponent's active phase does not clear the flag.
// ====================================================================

/// Set up an energy zone where exactly `flags` cards carry the do-not-active flag.
fn flag_last_n_energy(game: &mut TestGame, total: usize, flags: usize) -> Vec<i16> {
    let ids: Vec<i16> = (0..total).map(|_| game.id(ENERGY)).collect();
    let z = &mut game.state.player1.energy_zone;
    z.cards.clear();
    z.cards.extend(ids.iter().copied());
    z.set_active_count(0);
    let flagged: Vec<i16> = ids[total - flags..].to_vec();
    for &c in &flagged {
        game.state.mods.add_delayed_cannot_active(c, 1);
    }
    flagged
}

fn advance_to_phase_active(game: &mut TestGame) {
    game.state.current_turn_phase = TurnPhase::FirstAttackerNormal;
    game.state.current_phase = Phase::Active;
    TurnEngine::advance_phase(&mut game.state);
}

#[test]
fn q280_active_phase_excludes_flagged_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // 3 energy in zone, 2 of them flagged "do not active next phase".
    let _ = flag_last_n_energy(&mut game, 3, 2);

    advance_to_phase_active(&mut game);

    // A single advance_phase runs the Active arm; no Energy draw happened yet.
    let z = &game.state.player1.energy_zone;
    assert_eq!(z.cards.len(), 3, "3 energy cards in the zone");
    // The 2 flagged cards stayed INACTIVE, so active = 3 - 2 = 1. Without the fix
    // the Active arm would activate all 3.
    assert_eq!(
        z.active_count(),
        1,
        "Q280: the 2 do-not-activate energy must not become active; active={}",
        z.active_count()
    );
}

#[test]
fn q280_opponents_active_phase_does_not_clear_owner_flag() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    // Place 2 flagged energy owned by p1.
    let flagged = flag_last_n_energy(&mut game, 2, 2);

    // Simulate the OPPONENT's active phase ticking; since p1 owns the cards, the
    // opponent's turn must NOT consume p1's flag (it is the opponent's own active).
    let opponent_owned: std::collections::HashSet<i16> =
        game.state.player2.all_card_ids().into_iter().collect();
    game.state
        .mods
        .tick_delayed_cannot_active_for(&opponent_owned);
    for &c in &flagged {
        assert!(
            game.state.mods.is_delayed_cannot_active(c),
            "Q280: an opponent's active phase must NOT clear the owner's flag"
        );
    }

    // The owner's own active phase DOES consume the flag for the flagged energy.
    let owner_owned: std::collections::HashSet<i16> =
        game.state.player1.all_card_ids().into_iter().collect();
    game.state.mods.tick_delayed_cannot_active_for(&owner_owned);
    for &c in &flagged {
        assert!(
            !game.state.mods.is_delayed_cannot_active(c),
            "Q280: the owner's active phase consumes the do-not-activate flag"
        );
    }
}