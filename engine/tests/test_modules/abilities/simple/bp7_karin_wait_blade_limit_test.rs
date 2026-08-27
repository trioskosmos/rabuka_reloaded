/// BP07 C5: PL!N-bp7-004-R 朝香果林 ab#0.
///
/// 起動：エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：相手のステージにいる、
/// 元々持つブレードの数がこのメンバーの下にあるエネルギーカードの枚数に1を足した数以下の
/// メンバー1人をウェイトにする。
///
/// (Activation) Place 1 energy from the energy zone under this member: WAIT 1 member
/// on the opponent's stage whose ORIGINAL blade count is <= (energy cards under this
/// member + 1).
///
/// These tests prove the DYNAMIC limit: it must be computed from the energy actually
/// under 朝香果林 (after the cost), not a constant. The discriminating pair is a
/// blade-4 member: NOT waited when the limit is 2, but waited when pre-seeded energy
/// under 朝香果林 raises the limit to 5.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const KARIN: &str = "PL!N-bp7-004-R"; // 朝香果林 — DiverDiva, cost 2
const ENEMY_BLADE1: &str = "PL!-sd1-010-SD"; // 高坂穂乃果, original blade 1
const ENEMY_BLADE4: &str = "PL!N-bp1-001-R"; // 上原歩夢, original blade 4
const ENERGY: &str = "LL-E-001-SD";

/// Put 朝香果林 on center with `pre` energy already under her, plus 1 WAIT energy in
/// the energy zone to pay the activation cost.
fn setup(game: &mut TestGame, pre_under: usize) -> i16 {
    let karin = game.id(KARIN);
    game.state.player1.stage.stage[1] = karin;
    for _ in 0..pre_under {
        let e = game.id(ENERGY);
        game.state.player1.stage.place_under_card(MemberArea::Center, e);
    }
    let e = game.id(ENERGY);
    game.state.player1.energy_zone.cards.push(e);
    karin
}

/// Pay the cost prompt(s): select the first (only) energy card to place under karin.
fn pay_cost(game: &mut TestGame) {
    while game.has_pending_choice() {
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { .. } => {
                game.select_indices(&[0]);
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // Pay
            }
            _ => {
                game.select_indices(&[]);
            }
        }
    }
}

fn opponent_waited(game: &TestGame, id: i16) -> bool {
    game.state.mods.get_orientation_modifier(id).as_deref() == Some("wait")
}

fn energy_under_center(game: &TestGame) -> usize {
    game.state.player1.stage.get_under_cards(MemberArea::Center).len()
}

/// The cost must actually move 1 energy under 朝香果林, and a blade-1 opponent
/// member (<= limit 2) is waited.
#[test]
fn karin_cost_places_energy_and_waits_below_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = setup(&mut game, 0);
    let enemy = game.id(ENEMY_BLADE1);
    game.state.player2.stage.stage = [enemy, -1, -1];

    game.activate_ability(karin);
    pay_cost(&mut game);

    assert_eq!(
        energy_under_center(&game),
        1,
        "activation cost must place exactly 1 energy under 朝香果林"
    );
    assert!(
        opponent_waited(&game, enemy),
        "blade-1 opponent (<= limit 2) should be waited"
    );
}

/// With 1 energy under 朝香果林 (limit 2), a blade-4 opponent member is NOT waited.
#[test]
fn karin_does_not_wait_above_limit() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = setup(&mut game, 0);
    let enemy = game.id(ENEMY_BLADE4);
    game.state.player2.stage.stage = [enemy, -1, -1];

    game.activate_ability(karin);
    pay_cost(&mut game);

    assert_eq!(energy_under_center(&game), 1, "1 energy under 朝香果林 → limit 2");
    assert!(
        !opponent_waited(&game, enemy),
        "blade-4 opponent (> limit 2) must NOT be waited"
    );
}

/// With BOTH a blade-1 and a blade-4 member, only the in-limit one is waited.
#[test]
fn karin_waits_only_in_limit_when_both_present() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = setup(&mut game, 0);
    let low = game.id(ENEMY_BLADE1);
    let high = game.id(ENEMY_BLADE4);
    game.state.player2.stage.stage = [low, high, -1];

    game.activate_ability(karin);
    pay_cost(&mut game);

    assert!(opponent_waited(&game, low), "blade-1 member should be waited");
    assert!(
        !opponent_waited(&game, high),
        "blade-4 member must not be waited"
    );
}

/// DYNAMIC limit proof: pre-seed 3 energy under 朝香果林. After the cost adds 1,
/// energy_under = 4 → limit 5, so the SAME blade-4 member that failed at limit 2 is
/// now waited. This proves the limit is driven by the energy actually under her.
#[test]
fn karin_dynamic_limit_scales_with_energy_under() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = setup(&mut game, 3);
    let enemy = game.id(ENEMY_BLADE4);
    game.state.player2.stage.stage = [enemy, -1, -1];

    game.activate_ability(karin);
    pay_cost(&mut game);

    assert_eq!(
        energy_under_center(&game),
        4,
        "3 pre-seeded + 1 cost = 4 energy under → limit 5"
    );
    assert!(
        opponent_waited(&game, enemy),
        "blade-4 member (<= limit 5) should be waited when energy_under = 4"
    );
}

/// With TWO eligible (in-limit, active) opponent members, the engine must
/// present a member-selection choice (filtered to both) so the player can pick
/// WHICH to wait, rather than silently auto-selecting the first.
#[test]
fn karin_two_eligible_members_offers_selection_choice() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = setup(&mut game, 0); // limit = energy_under(0) + 1 = 1
    let low_a = game.id(ENEMY_BLADE1);
    let low_b = game.id(ENEMY_BLADE1);
    // both blade-1 members are <= limit 1 → both eligible
    game.state.player2.stage.stage = [low_a, low_b, -1];

    game.activate_ability(karin);
    let mut guard = 0;
    let mut stage_choice_filtered: Option<Vec<usize>> = None;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { zone, filtered_indices, .. } => {
                if zone == "energy_zone" || zone == "energy" {
                    game.select_indices(&[0]); // pay cost energy
                } else if zone == "stage" {
                    stage_choice_filtered = filtered_indices.clone();
                    game.select_indices(&[1]); // pick the SECOND eligible member
                } else {
                    game.select_indices(&[0]);
                }
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // Pay
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
    game.drain_auto_ability_choices();

    let fi = stage_choice_filtered.expect("stage member-selection choice should be offered");
    assert_eq!(fi, vec![0, 1], "both eligible members must be selectable");
    assert!(
        opponent_waited(&game, low_b),
        "the SECOND member (index 1) chosen by the player should be waited"
    );
    assert!(
        !opponent_waited(&game, low_a),
        "the FIRST member should NOT be waited when the second was chosen"
    );
}

/// The ENERGY card selection for the cost must offer the actual energy zone
/// cards as a selectable choice (not auto-pick the first). Here we seed two
/// energy cards and choose the second one to place under 朝香果林.
#[test]
fn karin_cost_energy_choice_selects_chosen_card() {
    let db = load_real_database();
    let mut game = TestGame::new(db.clone());

    let karin = setup(&mut game, 0);
    let enemy = game.id(ENEMY_BLADE1);
    game.state.player2.stage.stage = [enemy, -1, -1];
    // Add a second energy card to the zone so there's a choice.
    let e2 = game.id(ENERGY);
    game.state.player1.energy_zone.cards.push(e2);

    game.activate_ability(karin);
    let mut guard = 0;
    let mut saw_energy_choice = false;
    while game.has_pending_choice() && guard < 20 {
        guard += 1;
        let choice = game.get_pending_choice().clone();
        match choice {
            rabuka_engine::ability::types::Choice::SelectCard { zone, count, filtered_indices, .. } => {
                if zone == "energy_zone" || zone == "energy" {
                    saw_energy_choice = true;
                    assert_eq!(count, 1, "must place exactly 1 energy under");
                    assert!(
                        filtered_indices.is_none(),
                        "energy choice must offer ALL energy cards (no index filter)"
                    );
                    game.select_indices(&[1]); // pick the SECOND energy card
                } else if zone == "stage" {
                    game.select_indices(&[0]); // pick the single eligible opponent
                } else {
                    game.select_indices(&[0]);
                }
            }
            rabuka_engine::ability::types::Choice::SelectTarget { .. } => {
                game.select_choice_option(1); // Pay
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
    game.drain_auto_ability_choices();

    assert!(saw_energy_choice, "an energy-card selection choice must be offered");
    // The second energy card was placed under karin (center).
    let under = game.state.player1.stage.get_under_cards(MemberArea::Center);
    assert_eq!(under.len(), 1, "one energy placed under karin");
    assert_eq!(under[0], e2, "the SECOND (chosen) energy card is the one placed");
}

