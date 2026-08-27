use crate::helpers::*;
use rabuka_engine::card::HeartColor;

// PL!SP-bp2-015-N (Sumire) and PL!SP-bp2-021-N (Wien) — yell auto with blade-heart absence condition
// 自動 ターン1回 エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、ライブ終了時まで heart を得る
// Validated thoroughly: positive, negative, turn limit, multi-card, and is_null yell.

fn setup_yell(game: &mut TestGame, sumire: i16, wien: i16, revealed_ids: &[i16]) {
    let bladed = game.id("PL!S-sd1-003-SD"); // placeholder for stage filler, not used for yell
    game.state.player1.stage.stage = [bladed, sumire, wien];
    game.state.revealed_cards.clear();
    for &id in revealed_ids {
        game.state.revealed_cards.push(id);
    }
    game.state.yell_occurred = true;
    // Also push to waitroom as phases.rs does (revealed pool is action detail, but some handlers expect waitroom)
    for &id in revealed_ids {
        game.state.player1.waitroom.cards.push(id);
    }
}

#[test]
fn sumire_yell_no_blade_gains_heart06() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    let m_no_blade = game.id("PL!S-bp2-002-R"); // Riko, no blade_heart per upper_batch
    game.state.player1.stage.stage = [filler, sumire, -1];
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m_no_blade);
    game.state.yell_occurred = true;
    game.state.player1.waitroom.cards.push(m_no_blade);
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 0);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1, "Sumire should gain heart06 when yell has no blade heart");
}

#[test]
fn sumire_yell_with_blade_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    let m_blade = game.id("PL!-pb1-014-R"); // has blade_heart
    setup_yell(&mut game, sumire, wien, &[m_blade]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 0, "Sumire must NOT gain when yell contains blade heart");
    assert_eq!(game.state.mods.get_heart_modifier(wien, HeartColor::Heart03), 0, "Wien must NOT gain when yell contains blade heart");
}

#[test]
fn yell_mixed_blade_and_no_blade_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    let m_no_blade = game.id("PL!S-bp2-002-R");
    let m_blade = game.id("PL!-pb1-014-R");
    setup_yell(&mut game, sumire, wien, &[m_no_blade, m_blade]);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 0, "Mixed yell with 1 blade must still block");
}

#[test]
fn yell_empty_revealed_no_gain() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let wien = game.id("PL!SP-bp2-021-N");
    // Empty yell: per phases.rs yell_occurred = !revealed.is_empty() => false, so no trigger
    game.state.player1.stage.stage = [game.id("PL!S-sd1-003-SD"), sumire, wien];
    game.state.revealed_cards.clear();
    game.state.yell_occurred = false;
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 0, "Empty yell (yell_occurred false) must not trigger");
}

#[test]
fn yell_turn1_blocks_second_trigger() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let sumire = game.id("PL!SP-bp2-015-N");
    let filler = game.id("PL!-sd1-010-SD");
    let m_no_blade = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [filler, sumire, -1];
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m_no_blade);
    game.state.yell_occurred = true;
    game.state.player1.waitroom.cards.push(m_no_blade);
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1);
    // Second trigger same turn should be blocked by ターン1回 — clear revealed and retrigger, modifier must stay 1
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m_no_blade);
    game.state.yell_occurred = true;
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(sumire, HeartColor::Heart06), 1, "Second yell same turn must be blocked by turn1");
}

#[test]
fn wien_yell_no_blade_gains_heart03_independent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    // Only Wien on stage — Sumire absent, so only Wien triggers
    let wien = game.id("PL!SP-bp2-021-N");
    let filler = game.id("PL!-sd1-010-SD");
    let m_no_blade = game.id("PL!S-bp2-002-R");
    game.state.player1.stage.stage = [filler, wien, -1];
    game.state.revealed_cards.clear();
    game.state.revealed_cards.push(m_no_blade);
    game.state.yell_occurred = true;
    game.state.trigger_auto_abilities_for_player("p1");
    game.state.process_pending_auto_abilities("p1");
    assert_eq!(game.state.mods.get_heart_modifier(wien, HeartColor::Heart03), 1);
    // Sumire not on stage → no heart06 anywhere
    assert_eq!(game.state.mods.get_heart_modifier(filler, HeartColor::Heart06), 0);
}
