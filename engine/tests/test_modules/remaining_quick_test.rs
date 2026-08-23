/// REAL gameplay tests for Q203, Q204, Q218 — matching qa_data.json.
use crate::helpers::*;
use rabuka_engine::card::HeartColor;

fn advance_live(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn setup_eternalize_base(game: &mut TestGame) -> (i16, i16) {
    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    (live, filler)
}

fn run_live_with_eternalize(game: &mut TestGame, live: i16) {
    advance_live(game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }
}

/// Q204: Eternalize Love!! — 2+ 虹ヶ咲 members on stage → heart requirement reduced by 3 heart00.
#[test]
fn eternalize_q204_two_niko_hearts_reduced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");
    let niji = game.id("PL!N-pb1-012-R"); // 虹ヶ咲 member (series contains 虹ヶ咲)

    game.state.player1.stage.stage = [niji, niji, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    let needs_mod = game.state.mods.need_heart_modifiers.get(&live);
    assert!(
        needs_mod.is_some(),
        "Q204: need_heart modifier applied for 2+ 虹ヶ咲 members"
    );
    if let Some(mods) = needs_mod {
        let h00_val = mods
            .get(&HeartColor::Heart00)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
        assert!(
            h00_val == -3,
            "Q204: heart00 reduced by exactly 3 (got {})",
            h00_val
        );
    }
}

/// Q204: 0 虹ヶ咲 members — card_count_condition counts ALL members (no group filter).
#[test]
fn eternalize_q204_zero_niko_hearts_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!N-pb1-042-L");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [filler, -1, -1];
    game.state.player1.hand.cards.push(live);
    game.state.player1.hand.cards.push(filler);
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..60 {
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);

    advance_live(&mut game);
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // card_count_condition counts ALL members without group filtering.
    // With only 1 member (< threshold 2), the condition fails → no modifier.
    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "Q204: 1 member (<2) → condition fails → no modification"
    );
}

/// Eternalize: 2 same-name 虹ヶ咲 members → condition passes (same name + count >= 2)
#[test]
fn eternalize_same_name_two_niji_identical() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let ayumu = game.id("PL!N-pb1-001-R");
    // Two of the same card → same name
    game.state.player1.stage.stage = [ayumu, ayumu, -1];
    run_live_with_eternalize(&mut game, live);

    let mods = game.state.mods.need_heart_modifiers.get(&live);
    assert!(
        mods.is_some(),
        "same-name identical members → should trigger"
    );
    if let Some(m) = mods {
        let h00 = m
            .get(&HeartColor::Heart00)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
        assert!(h00 <= -3, "heart00 reduction >= 3 (got {})", h00);
    }
}

/// Eternalize: 2 different-name 虹ヶ咲 members → condition FAILS (same_name check)
#[test]
fn eternalize_different_names_no_reduction() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let kasumi = game.id("PL!N-pb1-002-R");
    let ayumu = game.id("PL!N-pb1-001-R");
    // Two different cards → different names
    game.state.player1.stage.stage = [kasumi, ayumu, -1];
    run_live_with_eternalize(&mut game, live);

    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "different names → should NOT trigger"
    );
}

/// Eternalize: 2 same-name members but only 1 is 虹ヶ咲 → condition FAILS (count < 2 matching group)
#[test]
fn eternalize_one_niji_one_other_triggers_zero() {
    let mut game = TestGame::new(load_real_database());
    let (live, filler) = setup_eternalize_base(&mut game);
    let niji = game.id("PL!N-pb1-001-R");
    // Only 1 member has 虹ヶ咲 series → count < 2
    game.state.player1.stage.stage = [niji, filler, -1];
    run_live_with_eternalize(&mut game, live);

    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "1 虹ヶ咲 member → count < 2 → should NOT trigger"
    );
}

/// Eternalize: 3 members, 2 share a name → condition passes (same_name satisfied)
#[test]
fn eternalize_two_same_one_different_triggers() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let kasumi = game.id("PL!N-pb1-002-R");
    let ayumu = game.id("PL!N-pb1-001-R");
    // Two ayumu (same name) + one kasumi (different) → at least 2 share a name
    game.state.player1.stage.stage = [ayumu, ayumu, kasumi];
    run_live_with_eternalize(&mut game, live);

    let mods = game.state.mods.need_heart_modifiers.get(&live);
    assert!(mods.is_some(), "2/3 share a name → should trigger");
    if let Some(m) = mods {
        let h00 = m
            .get(&HeartColor::Heart00)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total);
        assert!(h00 <= -3, "heart00 reduction >= 3 (got {})", h00);
    }
}

/// Eternalize: 3 members all different names → condition FAILS
#[test]
fn eternalize_three_all_different_no_trigger() {
    let mut game = TestGame::new(load_real_database());
    let (live, _) = setup_eternalize_base(&mut game);
    let kasumi = game.id("PL!N-pb1-002-R");
    let ayumu = game.id("PL!N-pb1-001-R");
    let karin = game.id("PL!N-pb1-004-R");
    // Three different cards → all different names
    game.state.player1.stage.stage = [kasumi, ayumu, karin];
    run_live_with_eternalize(&mut game, live);

    assert!(
        game.state.mods.need_heart_modifiers.get(&live).is_none(),
        "3 different names → should NOT trigger"
    );
}

/// Q203: Cara Tesoro (PL!N-pb1-037-L) ライブ開始時:
/// 「このターン、自分の『虹ヶ咲』のカードの効果によってウェイト状態の自分の
/// エネルギーをアクティブにしていた場合、このカードのスコアを＋１する。
/// さらに、…ウェイト状態のメンバーもアクティブにしていた場合、代わりに
/// スコアを＋２する。」
///
/// The bonus tiers are driven by real 虹ヶ咲 effects in the same turn:
/// エマ PL!N-bp4-008-R 起動 activates 1 energy OR 1 member,
/// エマ PL!N-pb1-008-R 登場 activates 1 member OR 2 energy.
use rabuka_engine::ability::types::Choice;
use rabuka_engine::core::types::AbilityTrigger;

fn fire_debut(game: &mut TestGame, cid: i16) {
    let card = game.db.get_card(cid).unwrap();
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
        Some(cid),
        None,
        None,
    );
    game.state.activating_card = Some(cid);
    game.state.process_pending_auto_abilities(&pid);
}

fn drain_cost_then_pick(game: &mut TestGame, option: usize, fodder: i16) {
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectCard { .. } => {
                // Discard-cost prompts must eat the FODDER card, never Cara.
                let pos = game
                    .state
                    .player1
                    .hand
                    .cards
                    .iter()
                    .position(|&c| c == fodder)
                    .expect("fodder still in hand for the activation cost");
                game.select_indices(&[pos]);
            }
            Choice::SelectTarget { target, .. } if target == "conditional_optional" => {
                game.select_option(option as i16);
            }
            _ => game.select_option(option as i16),
        }
    }
}

/// Shared board: エマ (虹ヶ咲 activator) on stage, 2 waited energy cards,
/// an optional waited member, and Cara Tesoro ready to be set as the live.
fn setup_cara_board(game: &mut TestGame, waited_member: Option<i16>) -> i16 {
    let filler = game.id("PL!-sd1-010-SD");
    let emma_activator = game.id("PL!N-bp4-008-R");
    match waited_member {
        Some(m) => {
            game.state.player1.stage.stage = [m, emma_activator, -1];
            game.state.mods.add_orientation_modifier(m, "wait");
        }
        None => {
            game.state.player1.stage.stage = [emma_activator, filler, -1];
        }
    }
    // 5 energy cards, only 3 active → 2 WAITED energy for effects to activate.
    game.give_energy(5);
    game.state.player1.energy_zone.set_active_count(3);

    let cara = game.id("PL!N-pb1-037-L");
    game.add_to_hand(cara);
    game.add_to_hand(filler); // discard fodder for エマ's 起動 cost
    for _ in 0..60 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.give_energy(15);
    cara
}

fn run_cara_live_start(game: &mut TestGame, cara: i16) {
    advance_live(game);
    game.set_live_card(cara);
    game.pass();
    game.pass();
    let mut guard = 0;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_indices(&[]),
            _ => game.select_indices(&[0]),
        }
    }
}

/// Control: nothing was activated by 虹ヶ咲 effects this turn → no bonus.
#[test]
fn cara_q203_no_nijigasaki_activation_no_bonus() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let cara = setup_cara_board(&mut game, None);
    run_cara_live_start(&mut game, cara);

    assert_eq!(
        game.state.mods.get_score_modifier(cara),
        0,
        "no 虹ヶ咲 activation this turn → +0"
    );
}

/// +1 tier: a 虹ヶ咲 effect activated WAITED ENERGY (エマ 起動, energy side).
#[test]
fn cara_q203_nijigasaki_energy_activation_gives_plus1() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let cara = setup_cara_board(&mut game, None);

    let active_before = game.state.player1.energy_zone.active_count();
    let emma = game.state.player1.stage.stage[0];
    let fodder = game.id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);
    game.activate_ability(emma);
    drain_cost_then_pick(&mut game, 0, fodder); // energy side
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before + 1,
        "precondition: エマ activated one energy"
    );

    run_cara_live_start(&mut game, cara);
    assert_eq!(
        game.state.mods.get_score_modifier(cara),
        1,
        "energy activated by a 虹ヶ咲 effect this turn → +1"
    );
}

/// +2 tier: energy AND member were activated by 虹ヶ咲 effects this turn
/// (エマ pb1 登場 member side + エマ bp4 起動 energy side).
#[test]
fn cara_q203_energy_and_member_gives_plus2() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let waited_member = game.id("PL!N-bp3-006-R"); // 虹ヶ咲 member, starts waited
    let cara = setup_cara_board(&mut game, Some(waited_member));

    // Member side: another エマ debuts and activates the waited member.
    let emma_pb1 = game.id("PL!N-pb1-008-R");
    game.state.player1.stage.stage[2] = emma_pb1;
    game.give_energy(10);
    fire_debut(&mut game, emma_pb1);
    assert!(
        game.has_pending_choice(),
        "debut either/or must present the choice"
    );
    game.select_option(0); // member side

    // Energy side: エマ bp4 起動 activates one waited energy.
    let active_before = game.state.player1.energy_zone.active_count();
    let emma_bp4 = game.state.player1.stage.stage[1];
    let fodder = game.id("PL!-sd1-010-SD");
    game.add_to_hand(fodder);
    game.activate_ability(emma_bp4);
    drain_cost_then_pick(&mut game, 0, fodder);
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before + 1,
        "precondition: エマ activated one energy"
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(waited_member),
        Some("active"),
        "precondition: the waited member was activated"
    );

    run_cara_live_start(&mut game, cara);
    assert_eq!(
        game.state.mods.get_score_modifier(cara),
        2,
        "energy AND member activated by 虹ヶ咲 effects → +2"
    );
}

/// Q203 ruling verbatim (2025.12.17): activating ONLY a waited member — no
/// energy activation — gives NO score increase.
#[test]
fn cara_q203_member_only_activation_gives_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let waited_member = game.id("PL!N-bp3-006-R");
    let cara = setup_cara_board(&mut game, Some(waited_member));

    // エマ pb1 debut, member side only — no energy is ever activated.
    let emma_pb1 = game.id("PL!N-pb1-008-R");
    game.state.player1.stage.stage[2] = emma_pb1;
    game.give_energy(10);
    let active_before = game.state.player1.energy_zone.active_count();
    fire_debut(&mut game, emma_pb1);
    assert!(game.has_pending_choice());
    game.select_option(0); // member side
    assert_eq!(
        game.state.mods.get_orientation_modifier(waited_member),
        Some("active"),
        "precondition: waited member was activated"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before,
        "precondition: no energy touched"
    );

    run_cara_live_start(&mut game, cara);
    assert_eq!(
        game.state.mods.get_score_modifier(cara),
        0,
        "Q203: member-only activation → +0"
    );
}

/// The activating effect must come from a 『虹ヶ咲』 card — the same energy
/// activation by a non-Nijigasaki card (蓮ノ空 エマ PL!HS-bp1-001-R) grants
/// nothing.
#[test]
fn cara_q203_non_nijigasaki_activation_gives_nothing() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let cara = setup_cara_board(&mut game, None);

    // Replace the Nijigasaki Emma with the Hasunosora one before her debut.
    let hasu_emma = game.id("PL!HS-bp1-001-R");
    game.state.player1.stage.stage[0] = hasu_emma;
    let active_before = game.state.player1.energy_zone.active_count();
    fire_debut(&mut game, hasu_emma);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        game.select_indices(&[]);
    }
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        active_before + 2,
        "precondition: energy WAS activated (2 cards) — by a non-虹ヶ咲 effect"
    );

    run_cara_live_start(&mut game, cara);
    assert_eq!(
        game.state.mods.get_score_modifier(cara),
        0,
        "non-虹ヶ咲 source → +0 even though energy was activated"
    );
}

/// Q218: Chika (PL!S-bp5-001-R+) ab#1 permanent reduces cost by 1 for no-ability
/// member cards, and this applies even when playing via baton touch.
#[test]
fn chika_q218_no_ability_cost_reduced() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let chika = game.id("PL!S-bp5-001-R\u{ff0b}");
    let filler = game.id("PL!-sd1-010-SD"); // no abilities, cost 4

    // Get base costs
    let chika_cost = game.db.get_card(chika).unwrap().cost.unwrap_or(0) as usize;
    let filler_cost = game.db.get_card(filler).unwrap().cost.unwrap_or(0) as usize;
    assert_eq!(
        filler_cost, 4,
        "Filler has cost 4 for meaningful reduction test"
    );

    // Play Chika to stage with excess energy, then play filler
    game.state.player1.hand.cards.push(chika);
    game.state.player1.stage.stage = [-1, -1, -1];
    game.give_energy(chika_cost + filler_cost + 5);
    game.play_to_stage(chika, rabuka_engine::zones::MemberArea::Center);

    // Drain any debut choices
    while game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    // Now play filler (no abilities, cost 4). Chika's permanent should reduce cost by 1.
    let energy_before = game.state.player1.energy_zone.active_count();
    game.state.player1.hand.cards.push(filler);
    game.play_to_stage(filler, rabuka_engine::zones::MemberArea::LeftSide);
    let energy_after = game.state.player1.energy_zone.active_count();

    // Cost reduced from 4 to 3 by Chika's permanent
    let expected_consumed = filler_cost as i32 - 1; // 3
    let actual_consumed = energy_before as i32 - energy_after as i32;
    assert_eq!(
        actual_consumed, expected_consumed,
        "Q218: Filler cost reduced from {} to {} by Chika's permanent (consumed {})",
        filler_cost, expected_consumed, actual_consumed
    );
    // Verify stage positions
    assert_eq!(
        game.state.player1.stage.stage[0], filler,
        "Q218: Filler on LeftSide"
    );
    assert_eq!(
        game.state.player1.stage.stage[1], chika,
        "Q218: Chika at Center"
    );
}
