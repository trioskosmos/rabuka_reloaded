use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

/// PL!-PR-023-PR (絢瀬絵里) ab#0
///
/// {{jidou.png|自動}}[ターン3回]自分か相手のステージにいるメンバーがウェイト状態に
/// なるたび、ライブ終了時まで、{{icon_blade.png|ブレード}}を得る。
///
/// Printed behaviour under test:
///   * every member - own side OR opponent's - that becomes wait hands Eri +1 blade
///   * the limit is 3 per turn, so a 4th wait transition changes nothing
const ERI: &str = "PL!-PR-023-PR"; // 自動 [ターン3回], cost 11
const WAITER: &str = "PL!-bp3-002-R"; // debut: wait up to 2 opponent members
const OPP: &str = "PL!-sd1-010-SD"; // cost 4, eligible for the debut wait
const FILLER: &str = "PL!-sd1-019-SD";

fn eri_blade(game: &TestGame, eri: i16) -> i16 {
    game.state.mods.get_blade_modifier(eri)
}

/// Play a `PL!-bp3-002-R` onto our stage, pay its optional discard cost and put
/// the opponent members at stage slot `slots` to wait (the debut waits up to 2).
///
/// The debut and Eri's wait-watcher are offered together as a
/// `SelectAutoAbility` ordering choice, so the debut is resolved FIRST — the
/// watcher only has a wait transition to see once the debut has applied it.
fn debut_wait_opponents(game: &mut TestGame, area: MemberArea, slots: &[usize]) {
    use rabuka_engine::ability::types::Choice;
    let waiter = game.new_id(WAITER);
    game.state.player1.hand.cards.push(waiter);
    game.play_to_stage(waiter, area);

    let mut guard = 0;
    let mut stage_picked = false;
    loop {
        guard += 1;
        assert!(guard < 20, "debut chain did not drain in 20 prompts");
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => {
                game.select_option(0);
            }
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                game.select_indices(&[0]);
            }
            Choice::SelectCard { zone, count, .. } if zone == "stage" && !stage_picked => {
                // The offer may be the whole opposing stage or only the members
                // still eligible (already-waited ones drop out), so map the wanted
                // slots onto whatever was actually offered — falling back to the
                // whole offer when none of the wanted slots are in it.
                let idxs: Vec<usize> = slots.iter().copied().filter(|i| i < count).collect();
                let idxs = if idxs.is_empty() {
                    (0..*count).collect()
                } else {
                    idxs
                };
                game.select_indices(&idxs);
                stage_picked = true;
            }
            other => panic!("unexpected choice during debut: {:?}", other),
        }
    }
    assert!(
        stage_picked,
        "the debut must offer the opponent-member selection"
    );
    assert!(
        !game.has_pending_choice(),
        "chain must drain clean, still pending: {:?}",
        game.get_pending_choice()
    );
}

#[test]
fn one_wait_transition_grants_one_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    let opp1 = game.new_id(OPP);
    let opp2 = game.new_id(OPP);
    game.state.player1.stage.stage = [eri, -1, -1];
    game.state.player2.stage.stage = [opp1, opp2, -1];
    game.add_to_hand(game.id(FILLER));
    game.give_energy(25);

    assert_eq!(eri_blade(&game, eri), 0, "starts with no blade");

    debut_wait_opponents(&mut game, MemberArea::Center, &[0]);

    assert_eq!(
        game.state.mods.get_orientation_modifier(opp1),
        Some("wait"),
        "the selected opponent member must actually be waiting"
    );
    assert_eq!(
        eri_blade(&game, eri),
        1,
        "「メンバーがウェイト状態になるたび」: one transition, one blade"
    );
}

#[test]
fn each_further_wait_transition_grants_another_blade() {
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    let opp1 = game.new_id(OPP);
    let opp2 = game.new_id(OPP);
    game.state.player1.stage.stage = [eri, -1, -1];
    game.state.player2.stage.stage = [opp1, opp2, -1];
    game.add_to_hand(game.id(FILLER));
    game.add_to_hand(game.id(FILLER));
    game.give_energy(25);

    debut_wait_opponents(&mut game, MemberArea::Center, &[0, 1]);

    assert_eq!(
        game.state.mods.get_orientation_modifier(opp1),
        Some("wait")
    );
    assert_eq!(
        game.state.mods.get_orientation_modifier(opp2),
        Some("wait")
    );
    assert_eq!(
        eri_blade(&game, eri),
        2,
        "two members became wait in one batch → two blades"
    );
}

/// {{jidou.png|自動}}[ターン3回] — three grants per turn, and no fourth.
///
/// Four wait transitions in one turn: both debuts cover the three opponent
/// members, then Eri's own {{kidou.png|起動}} (ab#1) waits one of her own. The
/// budget fills on the third grant; the fourth must change nothing.
#[test]
fn turn_cap_of_three_blocks_a_fourth_grant() {
    use rabuka_engine::ability::types::Choice;
    let db = load_real_database();
    let mut game = TestGame::new(db);

    let eri = game.id(ERI);
    let opp1 = game.new_id(OPP);
    let opp2 = game.new_id(OPP);
    let opp3 = game.new_id(OPP);
    game.state.player1.stage.stage = [eri, -1, -1];
    game.state.player2.stage.stage = [opp1, opp2, opp3];
    game.add_to_hand(game.id(FILLER));
    game.add_to_hand(game.id(FILLER));
    game.add_to_hand(game.id(FILLER));
    game.give_energy(25);

    debut_wait_opponents(&mut game, MemberArea::Center, &[0, 1]);
    assert_eq!(
        eri_blade(&game, eri),
        2,
        "first debut: two opponent members waited"
    );

    debut_wait_opponents(&mut game, MemberArea::RightSide, &[2]);
    assert_eq!(
        eri_blade(&game, eri),
        3,
        "third transition fills the [ターン3回] budget"
    );

    let own_stage = game.state.player1.stage.stage;
    let waiters_before = own_stage
        .iter()
        .filter(|&&cid| {
            cid >= 0 && game.state.mods.get_orientation_modifier(cid) == Some("wait")
        })
        .count();
    game.activate_ability_index(eri, 1);
    let mut picked = false;
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 30, "activation chain did not drain in 30 prompts");
        if !game.has_pending_choice() {
            break;
        }
        match game.get_pending_choice() {
            Choice::SelectAutoAbility { .. } => game.select_option(0),
            Choice::SelectCard { zone, .. } if zone == "hand" => {
                game.select_indices(&[0]);
            }
            Choice::SelectCard { zone, .. } if zone == "stage" && !picked => {
                // `count` is how many cards to PICK (1 here), not how many were
                // offered — the offer size is the option count. Pick the second
                // member when there is one, else the only offer.
                let opts = game.pending_choice_option_count().unwrap_or(1);
                let pick = 1.min(opts.saturating_sub(1));
                game.select_indices(&[pick]);
                picked = true;
            }
            other => panic!("unexpected choice during Eri's 起動: {:?}", other),
        }
    }
    assert!(picked, "the activation must offer a member to wait");
    assert!(
        !game.has_pending_choice(),
        "chain must drain clean, still pending: {:?}",
        game.get_pending_choice()
    );
    let waiters_after = game
        .state
        .player1
        .stage
        .stage
        .iter()
        .filter(|&&cid| {
            cid >= 0 && game.state.mods.get_orientation_modifier(cid) == Some("wait")
        })
        .count();
    assert_eq!(
        waiters_after,
        waiters_before + 1,
        "the fourth member really did turn wait"
    );
    assert_eq!(
        eri_blade(&game, eri),
        3,
        "[ターン3回]: a fourth wait transition grants nothing"
    );
}
