/// Tests for Dazzling Game (PL!SP-bp4-023-L) — Q187: exclude_selected
/// sequential select: pick 1 from {澁谷かのん, ウィーン・マルガレーテ, 鬼塚冬毬},
/// then pick 1 Liella! member OTHER than that. Both gain blade.
use crate::helpers::*;

fn advance_to_live_set(game: &mut TestGame) {
    for _ in 0..5 {
        game.pass();
    }
}

fn run_dazzling_yell(game: &mut TestGame, deck: Vec<i16>) {
    let live = game.new_id("PL!SP-bp4-023-L");
    let host = game.new_id("PL!HS-pb1-023-N");
    let extra_member = game.new_id("PL!-sd1-006-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    game.state.player1.hand.cards.clear();
    game.state.player1.waitroom.cards.clear();
    game.state.player1.main_deck.cards.clear();
    game.state.player2.hand.cards.clear();
    game.state.player2.main_deck.cards.clear();
    game.state.player1.stage.stage = [host, extra_member, -1];
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }
    game.add_to_hand(live);

    advance_to_live_set(game);
    game.set_live_card(live);
    game.state.player1.main_deck.cards = deck.into();
    game.pass();
    game.pass();
    game.drain_auto_ability_choices();
    game.pass();

    assert!(
        !game.has_pending_choice(),
        "unexpected choice: {:?}",
        game.get_pending_choice()
    );
}

fn last_p1_snap(game: &TestGame) -> &rabuka_engine::types::PerformanceSnapshot {
    game.state
        .performance_snapshots
        .iter()
        .rev()
        .find(|snapshot| snapshot.player_id == "p1")
        .expect("P1 should have a performance snapshot")
}

fn yell_card<'a>(
    snapshot: &'a rabuka_engine::types::PerformanceSnapshot,
    card_no: &str,
) -> &'a rabuka_engine::types::YellCardResult {
    snapshot
        .yell_cards
        .iter()
        .find(|card| card.card_no.as_ref() == card_no)
        .unwrap_or_else(|| {
            panic!(
                "missing yell card {card_no}: {:?}",
                snapshot
                    .yell_cards
                    .iter()
                    .map(|card| card.card_no.as_ref())
                    .collect::<Vec<_>>()
            )
        })
}

#[test]
fn dazzling_live_start_recolors_listed_blade_hearts_to_purple() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let h01 = game.new_id("PL!-sd1-013-SD");
    let h02 = game.new_id("PL!S-PR-017-PR");
    let h03 = game.new_id("PL!-sd1-010-SD");
    let h04 = game.new_id("PL!S-PR-015-PR");
    let h05 = game.new_id("PL!S-bp2-015-PR");
    let h06 = game.new_id("PL!N-bp1-021-N");
    let all = game.new_id("PL!-sd1-020-SD");
    let score = game.new_id("PL!-sd1-019-SD");
    let filler = game.new_id("PL!-sd1-010-SD");

    run_dazzling_yell(
        &mut game,
        vec![filler, h01, h02, h03, h04, h05, h06, all, score],
    );

    let snapshot = last_p1_snap(&game);
    for (card_no, original_index) in [
        ("PL!-sd1-013-SD", 1),
        ("PL!S-PR-017-PR", 2),
        ("PL!-sd1-010-SD", 3),
        ("PL!S-PR-015-PR", 4),
        ("PL!S-bp2-015-PR", 5),
    ] {
        let result = yell_card(snapshot, card_no);
        assert_eq!(result.blade_hearts[original_index], 0);
        assert_eq!(result.blade_hearts[6], 1);
        for index in 1..=6 {
            if index != original_index && index != 6 {
                assert_eq!(
                    result.blade_hearts[index], 0,
                    "{card_no} retained blade heart at index {index}"
                );
            }
        }
    }

    let purple_result = yell_card(snapshot, "PL!N-bp1-021-N");
    assert_eq!(purple_result.blade_hearts[6], 1);
    for index in 1..=5 {
        assert_eq!(purple_result.blade_hearts[index], 0);
    }

    let all_result = yell_card(snapshot, "PL!-sd1-020-SD");
    assert_eq!(all_result.blade_hearts[7], 0);
    assert_eq!(all_result.blade_hearts[6], 1);
    let score_result = yell_card(snapshot, "PL!-sd1-019-SD");
    assert_eq!(score_result.note_icons, 1);
}

#[test]
fn dazzling_live_start_leaves_draw_and_colorless_icons_unchanged() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let draw = game.new_id("PL!N-bp1-027-L");
    let colorless = game.new_id("PL!N-bp7-030-L");
    let filler = game.new_id("PL!-sd1-010-SD");
    let extra = game.new_id("PL!-sd1-010-SD");

    run_dazzling_yell(
        &mut game,
        vec![
            filler, draw, colorless, extra, extra, extra, extra, extra, extra,
        ],
    );

    let snapshot = last_p1_snap(&game);
    let draw_result = yell_card(snapshot, "PL!N-bp1-027-L");
    assert_eq!(draw_result.draw_icons, 1);
    assert_eq!(draw_result.blade_hearts[5], 0);
    assert_eq!(draw_result.blade_hearts[6], 1);

    let colorless_result = yell_card(snapshot, "PL!N-bp7-030-L");
    assert_eq!(colorless_result.blade_hearts[0], 2);
    assert_eq!(colorless_result.blade_hearts[6], 0);
}

/// Two eligible Liella! members on stage. First select picks かのん (one of the 3).
/// Second select must pick a Liella! member OTHER than かのん.
/// If exclude_selected works, only the other member is pickable → blade to both.
#[test]
fn dazzling_q187_exclude_selected_liella_other_pickable() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let dazzling = game.id("PL!SP-bp4-023-L");
    let filler = game.id("PL!-sd1-010-SD");
    // Liella! member (澁谷かのん) — one of the 3 named
    let kanon = game.id("PL!SP-pb1-001-R"); // 澁谷かのん, Liella!
                                            // Another Liella! member — use 唐 可可 (different name from Kanon)
    let liella = game.id("PL!SP-sd1-002-SD"); // 唐 可可, Liella!

    game.state.player1.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(filler);
    }
    game.state.player2.main_deck.cards.clear();
    for _ in 0..40 {
        game.state.player2.main_deck.cards.push(filler);
    }

    // Stage: kanon + another Liella! member
    game.state.player1.stage.stage = [kanon, liella, filler];
    game.state.player1.hand.cards.push(dazzling);

    advance_to_live_set(&mut game);
    game.set_live_card(dazzling);
    game.pass();
    game.pass();
    game.drain_auto_ability_choices();

    // LiveStart fires: 3 sequential actions
    // Action 0: select 1 from the 3 named members.
    // Observed: SelectCard zone=stage count=1 is prompted.
    assert!(
        game.has_pending_choice(),
        "first member selection must be prompted"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard stage prompt"
    );
    game.select_indices(&[0]); // pick kanon (index 0 on stage)
    // Action 1: select 1 Liella! member OTHER than kanon (exclude_selected=true).
    assert!(
        game.has_pending_choice(),
        "exclude_selected second selection must be prompted"
    );
    assert_eq!(
        game.pending_choice_type().as_deref(),
        Some("SelectCard"),
        "expected SelectCard stage prompt"
    );
    game.select_indices(&[0]); // pick the other Liella! member

    // After both selections, blade gain_resource should add blade to both selected
    let _has_blade = |id: i16| {
        game.state
            .mods
            .blade_modifiers
            .get(&id)
            .map_or(0, rabuka_engine::core::game_modifiers::ModifierEntry::total)
            > 0
    };
    // Both selected members should have blade
    let kanon_blade = game.state.mods.get_blade_modifier(kanon);
    let liella_blade = game.state.mods.get_blade_modifier(liella);
    assert!(
        kanon_blade > 0,
        "Kanon should gain blade from Dazzling Game"
    );
    assert!(
        liella_blade > 0,
        "Liella should gain blade from Dazzling Game"
    );
}
