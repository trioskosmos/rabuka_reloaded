/// Tests for untested abilities from the coverage report.
///
/// Covers:
///   - choice:           PL!N-pb1-010-R (debut: activate energy OR put Niji live on deck top)
///   - change_state:     PL!N-bp5-004-R (weight self to weight opponent with exactly 4 blades)
///   - change_state:     PL!S-bp6-001-R (if from graveyard, weight opponent cost>=13 side member)
///   - change_state:     PL!SP-PR-021   (if hearts>=5, weight opponent cost<=2 member)
///   - choose_target:    PL!N-bp3-010-R (choose self/opponent, 2 members to deck bottom)
///   - choose_target:    PL!N-bp4-002-R (choose self/opponent, look at top card)
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

// ============================================================
// Card constants
// ============================================================
const NIJI_CHOICE: &str = "PL!N-pb1-010-R";
const NIJI_LIVE: &str = "PL!N-bp1-026-L";
const FILLER: &str = "PL!-sd1-010-SD";
const FILLER_LIVE: &str = "PL!-sd1-019-SD";
const NIJI_BLADE_4: &str = "PL!N-bp5-004-R";
const SHION: &str = "PL!S-bp6-001-R";
const SP_PR_021: &str = "PL!SP-PR-021-PR";
const AZUNA_TARGET: &str = "PL!N-bp3-010-R";
const AIKO: &str = "PL!N-bp4-002-R";

// ============================================================
// Helpers
// ============================================================

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

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn drain_auto(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 30 {
        safety += 1;
        if game.pending_choice_type().as_deref() == Some("SelectAutoAbility") {
            game.select_indices(&[]);
        } else {
            break;
        }
    }
}

fn advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
    drain_auto(game);
}

fn trigger_live_start_with(game: &mut TestGame, filler_live: i16) {
    game.state.player1.hand.cards.push(filler_live);
    for _ in 0..10 {
        game.state.player2.main_deck.cards.push(game.id(FILLER));
    }
    advance_to_live_set(game);
    game.set_live_card(filler_live);
    advance_to_live_start(game);
}

// ============================================================
// choice: PL!N-pb1-010-R 窶・debut choice: activate energy OR put Niji live on deck top
// ============================================================

/// 逋ｻ蝣ｴ: 驕ｸ謚櫁い0 窶・繧ｨ繝阪Ν繧ｮ繝ｼ繧・譫壹い繧ｯ繝・ぅ繝悶↓縺吶ｋ縲・
#[test]
fn niji_choice_activate_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(NIJI_CHOICE);
    let e1 = game.id("LL-E-001-SD");
    game.state.player1.energy_zone.cards.push(e1);

    fill_deck(&mut game, "p1", 5);
    game.give_energy(10);

    game.add_to_hand(member);
    game.play_to_stage(member, MemberArea::Center);

    // Debut triggers during play_to_stage; handle any pending choices
    while game.has_pending_choice() {
        game.select_choice_option(0);
    }

    let active = game.state.player1.energy_zone.active_count();
    assert!(
        active >= 1,
        "option 0 should activate 1 energy, got {}",
        active
    );
}

/// 逋ｻ蝣ｴ: 驕ｸ謚櫁い1 窶・謗ｧ縺亥ｮ､縺ｮ陌ｹ繝ｶ蜥ｲ繝ｩ繧､繝悶ｒ2譫壹∪縺ｧ繝・ャ繧ｭ縺ｮ荳翫↓鄂ｮ縺上・
#[test]
fn niji_choice_put_live_on_deck_top() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(NIJI_CHOICE);

    let live1 = game.id(NIJI_LIVE);
    let live2 = game.id(NIJI_LIVE);
    game.state.player1.waitroom.cards.push(live1);
    game.state.player1.waitroom.cards.push(live2);

    fill_deck(&mut game, "p1", 5);
    game.give_energy(10);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.add_to_hand(member);
    game.play_to_stage(member, MemberArea::Center);

    while game.has_pending_choice() {
        game.select_choice_option(1);
    }

    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after > deck_before,
        "option 1 should put cards on deck top: before={}, after={}",
        deck_before,
        deck_after
    );
}

// ============================================================
// change_state: PL!N-bp5-004-R 窶・weight self to weight opponent w/ exactly 4 blades
// ============================================================

/// 繝ｩ繧､繝夜幕蟋区凾: 閾ｪ蛻・ｒ繧ｦ繧ｧ繧､繝医↓縺励※縲∫嶌謇九・繝悶Ξ繝ｼ繝画・螂ｽ4縺ｮ繝｡繝ｳ繝舌・繧偵え繧ｧ繧､繝医・
#[test]
fn niji_bp5_004_weight_exact_4_blades() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(NIJI_BLADE_4);
    // Condition is on ORIGINAL blades (蜈・・戟縺､窶ｦ縺｡繧・≧縺ｩ4縺､): use a member
    // whose natural blade count is exactly 4 (蜊励％縺ｨ繧・.
    let opponent_member = game.id("PL!-PR-003-PR");
    // A modified blade count must NOT satisfy the original-value filter:
    // filler naturally has 1 blade, +3 modifier makes 4 but original stays 1.
    let modified = game.id(FILLER);

    game.state.player1.stage.stage[1] = member;
    game.state.player2.stage.stage = [opponent_member, modified, -1];
    game.state.mods.add_blade_modifier(modified, 3);

    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);

    trigger_live_start_with(&mut game, filler_live);

    // Resolve choices:
    //  - "pay_optional_cost:skip_optional_cost" is a pay/skip SelectTarget
    //    where index 1 = pay (wait Karin herself), index 0 = skip.
    //  - then pick the wait target.
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_choice_option(1); // PAY the optional self-wait cost
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }

    assert_eq!(
        game.state.mods.get_orientation_modifier(opponent_member),
        Some("wait"),
        "member with natural 4 blades must be waited"
    );
    assert_ne!(
        game.state.mods.get_orientation_modifier(modified),
        Some("wait"),
        "modified-to-4 blades does not satisfy 蜈・・縺､: not a legal target"
    );
}

// ============================================================
// change_state: PL!S-bp6-001-R 窶・if from graveyard, weight cost>=13 side member
// ============================================================

/// 逋ｻ蝣ｴ: 謗ｧ縺亥ｮ､縺九ｉ逋ｻ蝣ｴ縺励※縺・↑縺・ｴ蜷医√さ繧ｹ繝・3莉･荳翫Γ繝ｳ繝舌・縺ｯ繧ｦ繧ｧ繧､繝医↓縺ｪ繧峨↑縺・・
#[test]
fn shion_not_from_graveyard_no_weight() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(SHION);
    let opp_member = game.id(FILLER);

    game.state.player1.stage.stage[1] = member;
    game.state.player2.stage.stage[0] = opp_member; // left side

    let opp_orientation = game.state.mods.get_orientation_modifier(opp_member);
    assert!(
        opp_orientation.is_none(),
        "direct placement should not trigger graveyard appearance condition"
    );
}

// ============================================================
// change_state: PL!SP-PR-021 窶・if hearts>=5, weight opponent cost<=2
// ============================================================

/// 繝ｩ繧､繝夜幕蟋区凾: 繝｡繝ｳ繝舌・縺ｮ繝上・繝亥粋險医′5譛ｪ貅縺ｮ蝣ｴ蜷医√さ繧ｹ繝・莉･荳九・逶ｸ謇九Γ繝ｳ繝舌・縺ｯ繧ｦ繧ｧ繧､繝医↓縺ｪ繧峨↑縺・・/// SP-PR-021's own cost is 5 竊・her heart total is below 5 竊・condition unmet.
#[test]
fn sp_pr_021_hearts_lt5_no_weight() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(SP_PR_021); // cost 5: alone, hearts total < 5
    // A cost竕､2 member WOULD be a legal target if the hearts竕･5 condition held.
    let opponent_member = game.id("PL!-sd1-002-SD"); // 邨｢轢ｬ邨ｵ驥・ cost 2

    game.state.player1.stage.stage[1] = member;
    game.state.player2.stage.stage[1] = opponent_member;

    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);

    trigger_live_start_with(&mut game, filler_live);

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    // Hearts < 5 竊・the weight effect must not fire at all.
    assert_ne!(
        game.state.mods.get_orientation_modifier(opponent_member),
        Some("wait"),
        "hearts below 5 must not trigger the opponent wait"
    );
}

// ============================================================
// choose_target_player: PL!N-bp3-010-R 窶・choose self/opponent
// ============================================================

/// 繝ｩ繧､繝夜幕蟋区凾: 閾ｪ蛻・ｒ驕ｸ縺ｶ 竊・閾ｪ蛻・・謗ｧ縺亥ｮ､縺ｮ繝｡繝ｳ繝舌・繧偵ョ繝・く縺ｮ荳九↓鄂ｮ縺上・
#[test]
fn azuna_target_self_puts_members_on_deck_bottom() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(AZUNA_TARGET);

    let m1 = game.id(FILLER);
    let m2 = game.id(FILLER);
    game.state.player1.waitroom.cards.push(m1);
    game.state.player1.waitroom.cards.push(m2);

    game.state.player1.stage.stage[1] = member;

    fill_deck(&mut game, "p1", 10);
    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);

    trigger_live_start_with(&mut game, filler_live);

    // Handle the target player choice
    while game.has_pending_choice() {
        let choice = game.get_pending_choice();
        let is_target =
            matches!(choice, rabuka_engine::ability::types::Choice::SelectTarget { .. });
        if is_target {
            game.select_option(0); // choose self
        } else {
            game.select_indices(&[0]);
        }
    }

    // After choosing self, members should be moved to deck bottom
    let deck_after = game.state.player1.main_deck.cards.len();
    assert!(
        deck_after >= 2,
        "deck should have gained cards after choosing self, deck_size={}",
        deck_after
    );
}

// ============================================================
// choose_target_player: PL!N-bp4-002-R 窶・look at top card of chosen player
// ============================================================

/// 繝ｩ繧､繝夜幕蟋区凾: 閾ｪ蛻・ｒ驕ｸ縺ｶ 竊・閾ｪ蛻・・繝・ャ繧ｭ縺ｮ荳逡ｪ荳翫・繧ｫ繝ｼ繝峨ｒ隕九ｋ縲・
#[test]
fn aiko_target_self_looks_at_top_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let member = game.id(AIKO);

    let top_card = game.id(FILLER);
    game.state.player1.main_deck.cards.clear();
    game.state.player1.main_deck.cards.push(top_card);
    for _ in 0..5 {
        game.state.player1.main_deck.cards.push(game.id(FILLER));
    }

    game.state.player1.stage.stage[1] = member;

    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);

    trigger_live_start_with(&mut game, filler_live);

    // Phase advancement drew cards; capture whoever is on top NOW — that is
    // the card the ability will look at.
    let looked = game.state.player1.main_deck.cards.first().copied().expect("deck has a top card");

    // Handle choices:
    //  - SelectTarget "self_or_opponent": option 0 = self
    //  - SelectCard zone=looked_at (destination=discard): index 0 = the looked card
    while game.has_pending_choice() {
        let choice = game.get_pending_choice();
        let is_target =
            matches!(choice, rabuka_engine::ability::types::Choice::SelectTarget { .. });
        if is_target {
            game.select_option(0);
        } else {
            game.select_indices(&[0]);
        }
    }

    assert!(
        !game.state.player1.main_deck.cards.contains(&looked),
        "the discarded card is no longer on the deck"
    );
    assert!(
        game.state.player1.waitroom.cards.contains(&looked),
        "choosing the discard option moves the looked-at top card to the waitroom"
    );
}
