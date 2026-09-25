use crate::helpers::*;

const FILLER: &str = "PL!-sd1-010-SD";
const FILLER_LIVE: &str = "PL!-sd1-019-SD";
const NIJI_BLADE_4: &str = "PL!N-bp5-004-R";
const SP_PR_021: &str = "PL!SP-PR-021-PR";

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

#[test]
fn pl_n_bp5_004_r_wait_original_four_blades_excludes_modified_four() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(NIJI_BLADE_4);
    let opponent_member = game.id("PL!-PR-003-PR");
    let modified = game.id(FILLER);
    game.state.player1.stage.stage[1] = member;
    game.state.player2.stage.stage = [opponent_member, modified, -1];
    game.state.mods.add_blade_modifier(modified, 3);
    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);
    trigger_live_start_with(&mut game, filler_live);
    let mut guard = 0;
    while game.has_pending_choice() && guard < 10 {
        guard += 1;
        match game.get_pending_choice() {
            rabuka_engine::ability::types::Choice::SelectTarget { target, .. }
                if target == "pay_optional_cost:skip_optional_cost" =>
            {
                game.select_choice_option(1);
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

#[test]
fn pl_sp_pr_021_pr_below_five_hearts_no_opponent_wait() {
    let db = load_real_database();
    let mut game = TestGame::new(db);
    let member = game.id(SP_PR_021);
    let opponent_member = game.id("PL!-sd1-002-SD");
    game.state.player1.stage.stage[1] = member;
    game.state.player2.stage.stage[1] = opponent_member;
    fill_deck(&mut game, "p2", 10);
    let filler_live = game.id(FILLER_LIVE);
    trigger_live_start_with(&mut game, filler_live);
    while game.has_pending_choice() {
        game.select_indices(&[]);
    }
    assert_ne!(
        game.state.mods.get_orientation_modifier(opponent_member),
        Some("wait"),
        "hearts below 5 must not trigger the opponent wait"
    );
}
