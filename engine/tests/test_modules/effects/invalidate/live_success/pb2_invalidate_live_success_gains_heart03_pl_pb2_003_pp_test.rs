//! 南ことり `PL!-pb2-003-PP` (ライブ開始時)
//!
//! 相手のステージにいるメンバー1人のすべての{{live_success.png|ライブ成功時}}能力を、
//! ライブ終了時まで、無効にする。これにより無効にした場合、ライブ終了時まで、
//! {{heart_03.png|heart03}}を得る。
//!
//! The heart is CONDITIONAL on the invalidation having actually done something:
//! 「これにより無効にした場合」. So a target with no ライブ成功時 ability must grant
//! nothing, which is the case most likely to be missed.

use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;
use rabuka_engine::zones::MemberArea;

const NICO: &str = "PL!-pb2-003-PP";
/// An opponent member that HAS a ライブ成功時 ability.
const OPP_LIVE_SUCCESS: &str = "PL!-bp6-001-P"; // 高坂穂乃果, Printemps
/// An opponent member with no ライブ成功時 ability.
const OPP_PLAIN: &str = "PL!-bp6-017-N"; // 小泉花陽, Printemps
const FILLER: &str = "PL!-sd1-010-SD";

fn fill_decks(game: &mut TestGame) {
    let f = game.id(FILLER);
    for _ in 0..12 {
        game.state.player1.main_deck.cards.push(f);
        game.state.player2.main_deck.cards.push(f);
    }
}

/// Stand the opponent member in their centre, fire ライブ開始時, answer the target
/// prompt with the first option, and return (game, nico).
fn fired_against(opponent: &str) -> (TestGame, i16) {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let opp = game.id(opponent);
    game.state.player2.stage.stage[MemberArea::Center as usize] = opp;

    let nico = game.id(NICO);
    game.assert_card_identity(nico, NICO);
    game.state.player1.hand.cards.push(nico);
    game.give_energy(20);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nico);

    let ability_id = {
        let card = game.db.get_card(nico).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(nico).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(nico);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(nico),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    // The prompt here is the invalidate TARGET, not the heart: 「相手のステージに
    // いるメンバー1人のライブ成功時能力を無効にし、こうなった場合」 chooses an
    // opponent member whether or not it actually carries that ability, so both
    // tests reach this prompt. The old `if has_pending_choice()` guard made a
    // missing prompt invisible, which is the failure that mattered here — a
    // target that never got selected cannot invalidate anything.
    game.drain_choices_strict(&["SelectCard"], &[0]);
    (game, nico)
}

/// Pin the fixture difference the whole test rests on: one target HAS a
/// ライブ成功時 ability, the other does not.
#[test]
fn fixture_live_success_abilities_differ() {
    let db = load_real_database();
    let game = TestGame::new(db);
    let has = game
        .db
        .get_card(game.id(OPP_LIVE_SUCCESS))
        .unwrap()
        .resolved_abilities()
        .any(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains("ライブ成功時"))
        });
    let has_not = game
        .db
        .get_card(game.id(OPP_PLAIN))
        .unwrap()
        .resolved_abilities()
        .any(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains("ライブ成功時"))
        });
    assert!(has, "{OPP_LIVE_SUCCESS} must have a ライブ成功時 ability to invalidate");
    assert!(
        !has_not,
        "{OPP_PLAIN} must have none, or the conditional heart could not be tested"
    );
}

#[test]
fn invalidating_a_live_success_ability_gains_heart03() {
    let (game, nico) = fired_against(OPP_LIVE_SUCCESS);
    assert_eq!(
        game.state.mods.get_heart_modifier(nico, HeartColor::Heart03),
        1,
        "これにより無効にした場合 — a target that HAS a ライブ成功時 ability is \
         invalidated, so the conditional heart is granted"
    );
}

#[test]
fn a_target_with_no_live_success_ability_gains_nothing() {
    let (game, nico) = fired_against(OPP_PLAIN);
    assert_eq!(
        game.state.mods.get_heart_modifier(nico, HeartColor::Heart03),
        0,
        "これにより無効にした場合 — nothing was invalidated, so the conditional \
         heart must NOT be granted. This is the case the card text singles out."
    );
}

/// 相手のステージ — our own ライブ成功時 abilities are not targets.
#[test]
fn our_own_live_success_abilities_are_not_invalidated() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    fill_decks(&mut game);
    let ally = game.id(OPP_LIVE_SUCCESS);
    game.state.player1.stage.stage[MemberArea::Center as usize] = ally;

    let nico = game.id(NICO);
    game.state.player1.hand.cards.push(nico);
    game.give_energy(20);
    for _ in 0..5 {
        game.pass();
    }
    game.set_live_card(nico);

    let ability_id = {
        let card = game.db.get_card(nico).unwrap();
        let ab = card
            .resolved_abilities()
            .find(|a| a.triggers.as_deref() == Some("ライブ開始時"))
            .unwrap();
        format!("{}_{}", card.card_no, ab.full_text)
    };
    let card_no = game.db.get_card(nico).unwrap().card_no.to_string();
    let pid = game.state.player1.id.clone();
    game.state.activating_card = Some(nico);
    game.state.trigger_auto_ability(
        ability_id,
        AbilityTrigger::LiveStart,
        pid.clone(),
        Some(card_no),
        Some(nico),
        None,
        None,
    );
    game.state.process_pending_auto_abilities(&pid);
    // With no opponent member there is nothing to target, so no prompt and no
    // heart. An empty `allowed` list makes any prompt a failure, which is the
    // assertion this negative was missing: it only checked the heart.
    game.drain_choices_strict(&[], &[]);
    assert_eq!(
        game.state.mods.get_heart_modifier(nico, HeartColor::Heart03),
        0,
        "相手のステージにいるメンバー1人 — with no opponent member there is nothing to \
         invalidate, so the conditional heart is not granted"
    );
}
