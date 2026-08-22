/// Untested abilities — batch 6.
///
///   - PL!HS-pb1-010-R 村野さやか 登場/ライブ開始時: my cost≥10 member →
///     wait ONE opponent member with cost≤4. Both sides of the gate probed.
///   - PL!SP-bp7-009-R 鬼塚夏美 ab#0 常時 (left/right heart02) + ab#1
///     ライブ開始時 センター: wait an OPPONENT member with original blades ≤2;
///     center-only activation position honored.
///   - PL!-pb1-017-R 小泉花陽 登場: optional self-wait → draw 1, then discard
///     1 UNLESS this turn had a baton touch.
///   - PL!-bp3-007-R 東條希 ライブ開始時: optional 2-discard → look top 3,
///     split 1 to hand / 1 to deck top / 1 to waitroom.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;
use rabuka_engine::core::types::AbilityTrigger;

const FILLER: &str = "PL!-sd1-010-SD"; // cost 4, blade 1
const CLEAN_KOTORI: &str = "PL!-pb1-021-PR"; // cost 5, blade 1
const BIG_RUBY: &str = "PL!S-bp5-009-R"; // cost 15, blade 5

fn trigger_auto(game: &mut TestGame, cid: i16, trigger: AbilityTrigger, trigger_str: &str) {
    let card = game.db.get_card(cid).unwrap();
    // Dual-trigger texts like 「登場/ライブ開始時」 are stored as one combined
    // string — match by containment.
    let ab = card
        .resolved_abilities()
        .find(|a| {
            a.triggers
                .as_deref()
                .is_some_and(|t| t.contains(trigger_str))
        })
        .expect("card should have the requested trigger ability");
    let pid = game.state.player1.id.clone();
    game.state.trigger_auto_ability(
        format!("{}_{}", card.card_no, ab.full_text),
        trigger,
        pid.clone(),
        Some(card.card_no.to_string()),
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

// ====================================================================
// PL!HS-pb1-010-R — condition (my cost≥10) and target filter (opp cost≤4)
// are independent gates.
// ====================================================================
#[test]
fn sayaka_hspb1010_waits_cheap_opponent_when_condition_met() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-010-R"); // cost 2
    let big = game.id(BIG_RUBY); // cost 15
    let cheap = game.id(FILLER); // cost 4

    game.state.player1.stage.stage[0] = big;
    game.state.player1.stage.stage[1] = sayaka;
    game.state.player2.stage.stage[0] = cheap;

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_waited(&game, cheap),
        "my stage has a cost-15 member → opponent's cost-4 member is waited"
    );
}

#[test]
fn sayaka_hspb1010_no_big_member_no_effect() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-010-R"); // herself: cost 2
    let cheap = game.id(FILLER);

    game.state.player1.stage.stage[1] = sayaka;
    game.state.player2.stage.stage[0] = cheap;

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_waited(&game, cheap),
        "no cost≥10 member on MY stage → condition unmet, opponent untouched"
    );
}

#[test]
fn sayaka_hspb1010_expensive_opponent_not_eligible() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sayaka = game.id("PL!HS-pb1-010-R");
    let big = game.id(BIG_RUBY);
    let pricey = game.id(CLEAN_KOTORI); // cost 5 > 4

    game.state.player1.stage.stage[0] = big;
    game.state.player1.stage.stage[1] = sayaka;
    game.state.player2.stage.stage[0] = pricey;

    trigger_auto(&mut game, sayaka, AbilityTrigger::Debut, "登場");
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        !is_waited(&game, pricey),
        "condition met but opponent's cheapest member costs 5 > 4 → nobody waited"
    );
}

fn is_waited(game: &TestGame, cid: i16) -> bool {
    game.state.mods.get_orientation_modifier(cid) == Some("wait")
}

// ====================================================================
// PL!SP-bp7-009-R 鬼塚夏美 — position-gated pair of abilities.
// ====================================================================
#[test]
fn natsumi_bpb7009_sides_grant_heart02_center_does_not() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp7-009-R");

    // Left side → heart02.
    game.state.player1.stage.stage[0] = natsumi;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(natsumi, HeartColor::Heart02),
        1,
        "左サイド: heart02 granted"
    );

    // Center → gone.
    game.state.player1.stage.stage[0] = -1;
    game.state.player1.stage.stage[1] = natsumi;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(natsumi, HeartColor::Heart02),
        0,
        "センター: neither side ability applies"
    );

    // Right side → back.
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[2] = natsumi;
    game.state.recalculate_constants();
    assert_eq!(
        game.state.mods.get_heart_modifier(natsumi, HeartColor::Heart02),
        1,
        "右サイド: heart02 granted again"
    );
}

#[test]
fn natsumi_bpb7009_center_waits_low_blade_opponent_only() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let natsumi = game.id("PL!SP-bp7-009-R");
    let low = game.id(FILLER); // original blade 1
    let high = game.id(BIG_RUBY); // original blade 5

    game.state.player1.stage.stage[1] = natsumi; // CENTER required
    game.state.player2.stage.stage = [low, high, -1];

    trigger_auto(
        &mut game,
        natsumi,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert!(
        is_waited(&game, low),
        "opponent member with 元々ブレード1 ≤ 2 is waited"
    );
    assert!(
        !is_waited(&game, high),
        "original blade 5 exceeds the limit → untouched"
    );

    // Position gate: from a SIDE the live-start does nothing at all.
    game.state.mods.add_orientation_modifier(low, "active");
    game.state.player1.stage.stage[1] = -1;
    game.state.player1.stage.stage[0] = natsumi;
    trigger_auto(
        &mut game,
        natsumi,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        !is_waited(&game, low),
        "（センター限定）: not in center → no effect even with eligible targets"
    );
}

// ====================================================================
// PL!-pb1-017-R 小泉花陽 — draw, then discard unless a baton touch happened
// this turn.
// ====================================================================
#[test]
fn hanayo_pb1017_discards_without_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-pb1-017-R");

    game.state.player1.stage.stage[1] = hanayo;
    let spare = game.id(FILLER);
    game.add_to_hand(spare);
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    let deck_card = game.id(FILLER);
    put_on_deck_top(&mut game, 0, deck_card);

    trigger_auto(&mut game, hanayo, AbilityTrigger::Debut, "登場");
    game.select_option(1); // accept self-wait cost

    assert!(is_waited(&game, hanayo), "accepted cost waits her");
    // Draw +1 then mandatory discard −1 → net zero. The discard asks which
    // hand card to drop — resolve it.
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => game.select_indices(&[0]),
            _ => break,
        }
    }
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "draw 1 then discard 1 without baton touch → hand back to 1"
    );
}

#[test]
fn hanayo_pb1017_baton_touch_skips_the_discard() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let hanayo = game.id("PL!-pb1-017-R");

    game.state.player1.stage.stage[1] = hanayo;
    let spare = game.id(FILLER);
    game.add_to_hand(spare);
    let stock = game.new_id(FILLER);
    fill_decks(&mut game, stock);
    let deck_card = game.id(FILLER);
    put_on_deck_top(&mut game, 0, deck_card);

    // This turn already had a baton touch.
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    game.state.baton_touch_count_p1 += 1;

    trigger_auto(&mut game, hanayo, AbilityTrigger::Debut, "登場");
    game.select_option(1);

    assert_eq!(
        game.state.player1.hand.cards.len(),
        2,
        "バトンタッチ済み → conditional discard skipped → net +1 card"
    );
}

// ====================================================================
// PL!-bp3-007-R 東條希 — optional 2-discard → look 3, distribute 1/1/1.
// ====================================================================
#[test]
fn nozomi_bp3007_skipped_cost_means_no_look() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let nozomi = game.id("PL!-bp3-007-R");

    game.state.player1.stage.stage[1] = nozomi;
    let filler = game.id(FILLER);
    fill_decks(&mut game, filler);
    let deck_before = game.state.player1.main_deck.cards.len();
    let hand_before = game.state.player1.hand.cards.len();

    trigger_auto(
        &mut game,
        nozomi,
        AbilityTrigger::LiveStart,
        "ライブ開始時",
    );
    if game.has_pending_choice() {
        game.select_option(0); // decline the optional cost
    }
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before,
        "declined cost → no look, deck untouched"
    );
    assert_eq!(
        game.state.player1.hand.cards.len(),
        hand_before,
        "declined cost → no card joins the hand"
    );
}
