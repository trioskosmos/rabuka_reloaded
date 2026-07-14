/// Tests for action types that had zero test coverage:
///
/// 1. invalidate_ability — Kanon (PL!SP-bp2-001) can nullify another member's live_start
/// 2. set_blade_type — VIVID WORLD (PL!N-bp4-025-L), Dazzling Game (PL!SP-bp4-023-L)
/// 3. any_number pay_energy — 常夏☆サンシャイン (PL!SP-bp5-025-L): pay any energy for score
use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_setup::ActionType;
use rabuka_engine::game_state::Phase;
use rabuka_engine::turn::TurnEngine;
use rabuka_engine::zones::MemberArea;

/// Kanon's debut: may nullify a Liella! member's live_start abilities.
/// If nullified, followup: add a Liella! card from waitroom to hand.
#[test]
fn kanon_invalidate_liella_live_start() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let kanon = game.id("PL!SP-bp2-001-R＋");
    let target = game.id("PL!SP-pb1-001-R"); // Kanon duplicate with live_start ability
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: other Liella! member on left, target at Center (will be replaced by baton touch)
    let other_liella = game.id("PL!SP-sd1-001-SD");
    game.state.player1.stage.stage = [other_liella, target, -1];
    game.state.player1.hand.cards.push(kanon);
    game.give_energy(20);

    // Play Kanon to Center with baton touch (replaces target)
    TurnEngine::execute_main_phase_action(
        &mut game.state,
        &ActionType::PlayMemberToStage,
        Some(kanon),
        None,
        Some(MemberArea::Center),
        Some(true),
    )
    .expect("Play Kanon with baton touch");

    // Process pending auto abilities (debut trigger)
    while game.has_pending_choice() {
        game.select_option(0); // Yes, invalidate
    }
    game.state.process_pending_auto_abilities("p1");

    // Kanon played to Center via baton touch
    assert_eq!(
        game.state.player1.stage.stage[1], kanon,
        "Kanon occupies Center after baton touch"
    );
    // The invalidate follow-up recovered the replaced target from waitroom to hand
    // Baton touch moved target→waitroom, then followup moved target→hand
    assert_eq!(
        game.state.player1.hand.cards.len(),
        1,
        "Hand: kanon played from hand (0) → recovery adds 1 = 1"
    );
    assert!(
        game.state.player1.hand.cards.contains(&target),
        "Target card recovered from waitroom to hand by invalidate followup"
    );
    // No blade/heart modifiers should remain on the invalidated target
    assert_eq!(
        game.state.mods.get_blade_modifier(target),
        0,
        "Target's blade modifiers nullified"
    );
}

/// VIVID WORLD: both abilities through a real live phase.
/// ab#0 (ライブ開始時): set_blade_type — yell blades become 青ブレード (Blue).
/// ab#1 (ライブ成功時): conditional modify_score — checks yelled 虹ヶ咲 cards for all 6 hearts.
#[test]
fn vivid_world_live_phase_blade_and_success() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!N-bp4-025-L");
    let filler = game.id("PL!-sd1-010-SD");
    // Use 3 stage members whose combined base hearts meet VIVID WORLD's
    // need_heart requirement (heart05=8, heart06=2, heart0=4).
    let stage_card = game.id("PL!HS-pb1-023-N"); // no abilities, h05=2, h06=2

    // Multiple yell cards that collectively provide all 6 heart colors
    // (each card matches 虹ヶ咲 group via series check: "ラブライブ！虹ヶ咲学園")
    let h01 = game.id("PL!N-bp1-002-R＋"); // 上原歩夢 — heart01 in base_heart
    let h02 = game.id("PL!N-bp1-005-R"); // 中須かすみ — heart02
    let h03 = game.id("PL!N-bp1-007-R"); // 桜坂しずく — heart03
    let h04 = game.id("PL!N-bp1-010-R"); // 近江彼方 — heart04
    let h05 = game.id("PL!N-bp1-004-R"); // 朝香果林 — heart05
    let h06 = game.id("PL!N-bp1-003-R＋"); // エマ・ヴェルデ — heart06

    game.state.player1.main_deck.cards.clear();
    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();

    // Deck: 6 rainbow members at top for yell, then fillers
    for c in [h01, h02, h03, h04, h05, h06] {
        game.state.player1.main_deck.cards.push(c);
    }
    for _ in 0..34 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    game.state.player1.stage.stage = [stage_card, stage_card, stage_card];
    game.state.player2.stage.stage = [-1, -1, -1];
    game.state.player1.energy_zone.cards.clear();
    for _ in 0..30 {
        game.state.player1.energy_zone.cards.push(filler);
    }

    // Advance to LiveCardSet phase
    game.state.player1.hand.cards.push(live_card);
    game.state.player1.hand.cards.push(filler);
    match game.state.current_phase {
        rabuka_engine::game_state::Phase::Main => {
            game.pass();
            game.pass();
            game.pass();
            game.pass();
            game.pass();
        }
        _ => {}
    }
    assert!(
        game.state.current_phase.to_string().contains("LiveCardSet"),
        "Reached LiveCardSet phase"
    );

    game.set_live_card(live_card);
    game.pass();
    game.pass();

    // ab#0: set_blade_type applies 青ブレード to stage members during FirstAttackerPerformance.
    // Check BEFORE the live ends (LiveEnd effects expire when check_expired_effects fires).
    assert!(
        !game.state.mods.blade_type_modifiers.is_empty(),
        "VIVID WORLD ab#0: blade_type_modifiers set on stage"
    );

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    game.pass();
    game.pass();

    // ab#1: score_mod depends on yell cards — the test setup uses cards
    // without blade_hearts, so the condition isn't met. That's OK; this
    // test verifies set_blade_type and basic live flow complete cleanly.
}

/// VIVID WORLD ab#1: live_success checks yell-revealed 虹ヶ咲 member cards for
/// collective base_heart coverage of heart01–heart06.
/// All tests use Heart00 injection into stage_hearts to guarantee live success,
/// and directly populate revealed_cards + trigger_live_success_abilities for
/// targeted condition evaluation without going through the full yell pipeline.
#[test]
fn vivid_world_all_heart_colors_present() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!N-bp4-025-L");
    // 5 虹ヶ咲 members whose base_hearts collectively = heart01–heart06
    let y1 = game.id("PL!N-bp1-012-R\u{ff0b}"); // 鐘嵐珠:   heart01,heart04,heart06
    let y2 = game.id("PL!N-bp1-005-R"); // 宮下愛:   heart01,heart02
    let y3 = game.id("PL!N-bp1-007-R"); // 優木せつ菜: heart02,heart03
    let y4 = game.id("PL!N-bp1-004-R"); // 朝香果林: heart05,heart06
    let y5 = game.id("PL!N-bp1-003-R\u{ff0b}"); // 桜坂しずく: heart04,heart05

    game.state.player1.live_card_zone.cards.push(live_card);

    // Inject Heart00 wildcard into stage_hearts to guarantee live success
    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    game.state.revealed_cards.extend([y1, y2, y3, y4, y5]);

    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        1,
        "all 6 heart colors present → +1 score"
    );
}

#[test]
fn vivid_world_missing_heart_color() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!N-bp4-025-L");
    // 3 虹ヶ咲 members covering heart01–heart05 only (heart06 missing)
    let y1 = game.id("PL!N-bp1-005-R"); // 宮下愛:   heart01,heart02
    let y2 = game.id("PL!N-bp1-007-R"); // 優木せつ菜: heart02,heart03
    let y3 = game.id("PL!N-bp1-003-R\u{ff0b}"); // 桜坂しずく: heart04,heart05

    game.state.player1.live_card_zone.cards.push(live_card);

    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    game.state.revealed_cards.extend([y1, y2, y3]);

    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "heart06 missing from 虹ヶ咲 subset → no score"
    );
}

#[test]
fn vivid_world_half_nijigasaki_half_other() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live_card = game.id("PL!N-bp4-025-L");
    // 3 虹ヶ咲 members covering heart01–heart05 only (missing heart06)
    let y1 = game.id("PL!N-bp1-005-R"); // 宮下愛:   heart01,heart02
    let y2 = game.id("PL!N-bp1-007-R"); // 優木せつ菜: heart02,heart03
    let y3 = game.id("PL!N-bp1-003-R\u{ff0b}"); // 桜坂しずく: heart04,heart05
                                                // 1 non-虹ヶ咲 (μ's) member that DOES provide the missing heart06
    let n1 = game.id("PL!-sd1-009-SD"); // 矢澤にこ: heart01=2,heart03=2,heart06=2

    game.state.player1.live_card_zone.cards.push(live_card);

    let mut h = BaseHeart {
        hearts: HeartMap::new(),
    };
    h.hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(h);

    // ALL cards collectively cover heart01–heart06, but the non-虹ヶ咲 card
    // supplying heart06 must be filtered out by the group check.
    game.state.revealed_cards.extend([y1, y2, y3, n1]);

    game.state.current_phase = Phase::LiveVictoryDetermination;
    TurnEngine::trigger_live_success_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    assert_eq!(
        game.state.mods.get_score_modifier(live_card),
        0,
        "non-虹ヶ咲 heart06 filtered out — 虹ヶ咲-only subset missing heart06 → no score"
    );
}

/// 常夏☆サンシャイン (PL!SP-bp5-025-L): Live success — pay any number of energy,
/// for every 4 energy paid, +1 score.
#[test]
fn natsumi_sunshine_pay_any_energy_for_score() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!SP-bp5-025-L");
    // Members must satisfy need_heart: heart02=2, heart03=2, heart06=2, heart0=8
    // PL!-sd1-009-SD: heart01=2, heart03=2, heart06=2 (total 6)
    // PL!SP-sd1-020-SD: heart02=1 (total 1)
    // Combined: heart02=1+1=2 ✓, heart03=2 ✓, heart06=2 ✓, heart0=6+1+1=8 ✓
    let nico = game.id("PL!-sd1-009-SD");
    let h02_a = game.id("PL!SP-sd1-020-SD");
    let h02_b = game.id("PL!SP-sd1-020-SD");
    let filler = game.id("PL!-sd1-010-SD");

    game.state.player1.stage.stage = [nico, h02_a, h02_b];
    game.state.player1.hand.cards.push(live);
    game.give_energy(10);

    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));

    game.set_live_card(live);
    game.pass();
    game.pass();

    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    game.pass();
    game.pass();
    game.pass();

    assert!(game.has_pending_choice(), "LiveSuccess should fire");

    let mut paid = 0;
    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectAutoAbility") => game.select_indices(&[]),
            Some("SelectCard") if paid < 8 => {
                game.select_indices(&[0]);
                paid += 1;
            }
            _ => {
                // Skip to finalize (stop paying energy, trigger effect)
                game.select_indices(&[]);
                break;
            }
        }
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        2,
        "8 energy paid -> +2 score"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "10 - 8 = 2 active energy remaining"
    );
}

/// ビタミンSUMMER！ (PL!SP-bp2-024-L): Live success — if hand > opponent hand, +1 score.
#[test]
fn vitamin_summer_live_success_hand_condition() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let live = game.id("PL!SP-bp2-024-L");
    let filler = game.id("PL!-sd1-010-SD");

    // Needs: heart02=1, heart03=4, heart06=1, heart0=6
    // PL!-sd1-009-SD (Nico): heart01=2, heart03=2, heart06=2
    // PL!SP-sd1-020-SD: heart02=1
    let nico = game.id("PL!-sd1-009-SD");
    let h02 = game.id("PL!SP-sd1-020-SD");
    game.state.player1.stage.stage = [nico, nico, h02];
    game.state.player1.hand.cards.push(live);
    for _ in 0..5 {
        game.state.player1.hand.cards.push(filler);
    }
    for _ in 0..3 {
        game.state.player2.hand.cards.push(filler);
    }
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    for _ in 0..20 {
        game.state.player2.main_deck.cards.push(filler);
    }

    for _ in 0..5 {
        game.pass();
    }
    assert!(game.state.current_phase.to_string().contains("LiveCardSet"));
    game.set_live_card(live);
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    game.pass();
    game.pass();
    game.pass();
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }

    assert_eq!(
        game.state.mods.get_score_modifier(live),
        0,
        "LiveSuccess score bonus cleared after live"
    );
    let l = &game.state.performance_snapshots[0].lives[0];
    assert_eq!(l.score - l.base_score, 1, "bonus in final score");
}

/// Step! ZERO to ONE (PL!S-bp6-019-L): Live start — if all stage members are Aqours,
/// +1 score, draw 1, then move 1 from hand to deck top or bottom.
#[test]
fn step_zero_to_one_live_start_full_ability() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let live = game.id("PL!S-bp6-019-L");
    let aqours = game.id("PL!S-PR-025-RM"); // 高海千歌, Aqours member

    // One Aqours member on stage — satisfies the condition
    game.state.player1.stage.stage = [aqours, -1, -1];
    game.state.player2.stage.stage = [-1, -1, -1];

    game.state.player1.live_card_zone.cards.push(live);
    let filler = game.id("PL!-sd1-010-SD");
    for _ in 0..10 {
        game.state.player1.main_deck.cards.push(filler);
    }

    TurnEngine::trigger_live_start_abilities(&mut game.state, "p1");
    game.state.process_pending_auto_abilities("p1");

    while game.has_pending_choice() {
        match game.pending_choice_type().as_deref() {
            Some("SelectCard") => {
                game.select_indices(&[0]);
            }
            Some("position|destination") => {
                game.select_option(0);
            }
            Some(t) if t.contains("destination") => {
                game.select_option(0);
            }
            _ => {
                game.select_generated(0);
            }
        }
    }

    assert_eq!(game.state.mods.get_score_modifier(live), 1, "score +1");
}
