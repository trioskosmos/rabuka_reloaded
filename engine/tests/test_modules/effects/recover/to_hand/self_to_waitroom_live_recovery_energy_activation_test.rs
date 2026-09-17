use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD";

fn live_recovery_energy_activation_board_pl_s_bp3_008_r(game: &mut TestGame, active: u8) -> (i16, i16, i16) {
    let mari = game.id("PL!S-bp3-008-R");
    let aqours_high = game.id("PL!S-pb1-023-L");
    let niji_low = game.id("PL!N-bp1-028-L");
    game.add_to_stage(MemberArea::Center, mari);
    game.add_to_discard(aqours_high);
    game.add_to_discard(niji_low);
    let energy = game.id("LL-E-001-SD");
    for _ in 0..6 {
        game.state.player1.energy_zone.cards.push(energy);
    }
    game.state.player1.energy_zone.set_active_count(active);
    (mari, aqours_high, niji_low)
}

#[test]
fn self_to_waitroom_recovers_high_score_aqours_live_and_activates_four_pl_s_bp3_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _) = live_recovery_energy_activation_board_pl_s_bp3_008_r(&mut game, 2);

    game.activate_ability(mari);

    assert!(
        game.state.player1.waitroom.cards.contains(&mari),
        "activation cost sends Mari herself to the waitroom"
    );
    assert_eq!(game.state.player1.stage.stage[1], -1, "Q79: area vacated");

    assert!(game.has_pending_choice(), "live selection must be offered");
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == aqours_high)
        .expect("Aqours high live offered");
    game.select_indices(&[idx]);

    assert!(game.state.player1.hand.cards.contains(&aqours_high));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        6,
        "Aqours score≥6 fetched → activate 4 (2+4=6)"
    );
}

#[test]
fn recovering_wrong_group_live_skips_energy_activation_pl_s_bp3_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, _, niji_low) = live_recovery_energy_activation_board_pl_s_bp3_008_r(&mut game, 2);

    game.activate_ability(mari);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == niji_low)
        .expect("non-Aqours live also selectable");
    game.select_indices(&[idx]);

    assert!(game.state.player1.hand.cards.contains(&niji_low));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "『Aqours』 filter fails → NO activation"
    );
}

#[test]
fn recovering_aqours_live_below_six_skips_energy_activation_pl_s_bp3_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _) = live_recovery_energy_activation_board_pl_s_bp3_008_r(&mut game, 2);

    let aqours_low = game.id("PL!S-PR-024-PR");
    game.state
        .player1
        .waitroom
        .cards
        .retain(|c| *c != aqours_high);
    game.add_to_discard(aqours_low);

    game.activate_ability(mari);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == aqours_low)
        .expect("low Aqours live selectable");
    game.select_indices(&[idx]);

    assert!(game.state.player1.hand.cards.contains(&aqours_low));
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        2,
        "スコア6以上 fails at exactly-below boundary → NO activation"
    );
}

#[test]
fn self_to_waitroom_cost_paid_without_recoverable_live_pl_s_bp3_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let mari = game.id("PL!S-bp3-008-R");
    game.add_to_stage(MemberArea::Center, mari);
    game.add_to_discard(game.new_id(FILLER));

    game.activate_ability(mari);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert!(
        game.state.player1.waitroom.cards.contains(&mari),
        "Q123: usable — cost was paid"
    );
    assert!(
        game.state.player1.hand.cards.is_empty(),
        "nothing added when no live exists"
    );
    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        0,
        "no fetch → no activation"
    );
}

#[test]
fn live_recovery_is_mandatory_and_excludes_members_pl_s_bp3_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _niji_low) = live_recovery_energy_activation_board_pl_s_bp3_008_r(&mut game, 2);
    let member_in_waitroom = game.new_id(FILLER);
    game.add_to_discard(member_in_waitroom);

    game.activate_ability(mari);

    let member_idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == member_in_waitroom)
        .expect("member present in waitroom");

    let member_pick = game.try_select_indices(&[member_idx]);
    let _ = member_pick;
    assert!(
        !game.state.player1.hand.cards.contains(&member_in_waitroom),
        "member card must NEVER reach the hand via this ability"
    );

    while game.has_pending_choice() {
        let live_here = game
            .state
            .player1
            .waitroom
            .cards
            .iter()
            .position(|&c| c == aqours_high);
        match live_here {
            Some(idx) => {
                game.select_indices(&[idx]);
                break;
            }
            None => game.select_indices(&[0]),
        }
    }
    assert!(
        game.state.player1.hand.cards.contains(&aqours_high),
        "Q123: mandatory fetch — a live card MUST reach the hand"
    );
    assert!(
        !game.state.player1.hand.cards.contains(&member_in_waitroom)
            && game.state.player1.waitroom.cards.contains(&member_in_waitroom),
        "the member stayed in the waitroom"
    );
}

#[test]
fn recovered_live_activates_only_available_energy_candidates_pl_s_bp3_008_r() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let (mari, aqours_high, _) = live_recovery_energy_activation_board_pl_s_bp3_008_r(&mut game, 1);
    while game.state.player1.energy_zone.cards.len() > 3 {
        game.state.player1.energy_zone.cards.pop();
    }
    game.activate_ability(mari);
    let idx = game
        .state
        .player1
        .waitroom
        .cards
        .iter()
        .position(|&c| c == aqours_high)
        .expect("Aqours high live offered");
    game.select_indices(&[idx]);

    assert_eq!(
        game.state.player1.energy_zone.active_count(),
        4,
        "activate 4 requested but only 3 candidate energies exist → Q167 partial \
         resolution activates those 3 instead of aborting (1+3=4)"
    );
}
