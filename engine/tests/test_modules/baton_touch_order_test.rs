use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

fn fill_deck(game: &mut TestGame) {
    let f = game.id("PL!-sd1-010-SD");
    for _ in 0..40 {
        game.state.player1.main_deck.cards.push(f);
    }
}

fn active_energy(g: &TestGame) -> usize {
    g.state.player1.energy_zone.active_count()
}

fn drain_all(game: &mut TestGame) {
    let mut safety = 0;
    while game.has_pending_choice() && safety < 20 {
        safety += 1;
        use rabuka_engine::ability::types::Choice;
        match game.state.get_pending_choice().unwrap().clone() {
            Choice::SelectAutoAbility { .. } => {
                game.select_indices(&[]);
            }
            Choice::SelectCard { count, .. } => {
                if count > 0 && count < 10 {
                    game.select_indices(&(0..count).collect::<Vec<_>>());
                } else {
                    game.select_indices(&[0]);
                }
            }
            Choice::SelectTarget { target, .. }
                if target == "position|destination" || target == "area_select" =>
            {
                let acts = game.generated_actions();
                if acts.is_empty() {
                    game.select_indices(&[]);
                } else {
                    game.select_generated(0);
                }
            }
            _ => {
                game.select_indices(&[0]);
            }
        }
    }
}

/// Test 1: 花帆's baton_touch trigger activates 2 energy when she's replaced
/// by a cost 10+ 蓮ノ空 member via baton touch.
/// 花帆 sd1-001 cost=9, replacer PR-001 cost=10 → net cost=1, ability +2 → net +1
#[test]
fn baton_touch_activates_energy() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanafu = game.id("PL!HS-sd1-001-SD");
    let replacer = game.id("PL!HS-PR-001-PR"); // cost 10, unit スリーズブーケ (蓮ノ空)
    let filler = game.id("PL!-sd1-010-SD");

    fill_deck(&mut game);
    game.give_energy(20);

    game.add_to_stage(MemberArea::LeftSide, filler);
    game.add_to_stage(MemberArea::Center, hanafu);

    let e_before = active_energy(&game);
    eprintln!("Energy before baton touch: {}", e_before);

    game.state.player1.hand.cards.push(replacer);
    game.play_to_stage(replacer, MemberArea::Center);
    drain_all(&mut game);

    let e_after = active_energy(&game);
    // 花帆 cost=9, replacer cost=10, net cost=1. Ability +2. Net: -1+2=+1
    eprintln!(
        "Energy after: {} (expected {} = {} + 1)",
        e_after,
        e_before + 1,
        e_before
    );
    assert_eq!(
        e_after,
        e_before + 1,
        "baton_touch: -1 cost + 2 activation = +1 net"
    );
}

/// Test 2: セラス's EdelNote appearance trigger waits opponent's active member.
#[test]
fn edelnote_appearance_waits_opponent() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let seras = game.id("PL!HS-bp6-007-R");
    let edelnote = game.id("PL!HS-sd1-007-SD"); // EdelNote, cost 4
    let opp_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    fill_deck(&mut game);
    game.give_energy(20);

    game.add_to_stage(MemberArea::LeftSide, filler);
    game.add_to_stage(MemberArea::Center, seras);

    game.state.player2.stage.stage = [opp_member, -1, -1];

    game.state.player1.hand.cards.push(edelnote);
    game.play_to_stage(edelnote, MemberArea::RightSide);
    drain_all(&mut game);

    let opp_wait = game.state.mods.get_orientation_modifier(opp_member);
    eprintln!("Opponent wait state: {:?}", opp_wait);
    assert_eq!(
        opp_wait,
        Some("wait"),
        "EdelNote appearance should wait opponent member"
    );
}

/// Test 3: Baton touch + appearance triggers in same action.
/// EdelNote card (cost 4) replaces 花帆 — only セラス's appearance fires
/// (花帆 requires cost 10+ partner).
#[test]
fn both_triggers_in_baton_touch() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let hanafu = game.id("PL!HS-sd1-001-SD");
    let seras = game.id("PL!HS-bp6-007-R");
    let edelnote = game.id("PL!HS-sd1-007-SD");
    let opp_member = game.id("PL!-sd1-010-SD");
    let filler = game.id("PL!-sd1-010-SD");

    fill_deck(&mut game);
    game.give_energy(20);

    game.add_to_stage(MemberArea::LeftSide, seras);
    game.add_to_stage(MemberArea::Center, hanafu);
    game.add_to_stage(MemberArea::RightSide, filler);

    game.state.player2.stage.stage = [opp_member, -1, -1];

    let e_before = active_energy(&game);
    eprintln!("Energy before: {}", e_before);

    game.state.player1.hand.cards.push(edelnote);
    game.play_to_stage(edelnote, MemberArea::Center);
    drain_all(&mut game);

    let e_after = active_energy(&game);
    eprintln!("Energy after: {}", e_after);
    let opp_wait = game.state.mods.get_orientation_modifier(opp_member);
    eprintln!("Opponent wait state: {:?}", opp_wait);

    // 花帆 requires cost >=10 蓮ノ空 — edelnote is cost 4, not triggered → no energy activation
    assert_eq!(
        e_after, e_before,
        "花帆 requires cost>=10 蓮ノ空 partner — not triggered"
    );
    // セラス triggers on EdelNote appearance
    assert_eq!(
        opp_wait,
        Some("wait"),
        "セラス fires on EdelNote appearance"
    );
}
