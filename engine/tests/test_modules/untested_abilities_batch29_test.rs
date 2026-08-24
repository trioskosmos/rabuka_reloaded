/// Untested-abilities batch 29 — cost-gated baton-source debut draws:
/// - PL!-PR-045-PR (登場): replacing a COST-7 member via baton touch ->
///   draw 2, then discard 1 from hand. Negative: replacing a cost-4 member.
use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

const FILLER: &str = "PL!-sd1-010-SD"; // μ's member, cost 4

fn run_flow(db: std::sync::Arc<rabuka_engine::card::CardDatabase>, replaced_no: &str) {
    let mut game = TestGame::new(db);
    let replaced = game.id(replaced_no);
    game.state.player1.stage.stage[0] = replaced;

    let me = game.new_id("PL!S-PR-045-PR");
    game.state.player1.hand.cards.push(me);
    game.give_energy(25);

    let d1 = game.new_id(FILLER);
    let d2 = game.new_id(FILLER);
    game.state.player1.main_deck.cards.push(d1);
    game.state.player1.main_deck.cards.push(d2);

    let deck_before = game.state.player1.main_deck.cards.len();

    game.play_to_stage(me, MemberArea::LeftSide);
    if game.has_pending_choice() {
        game.select_indices(&[0]);
    }

    assert_eq!(
        game.state.player1.main_deck.cards.len(),
        deck_before - 2,
        "replaced {:?} (cost {}) -> deck should shrink by 2 draws",
        replaced_no,
        {
            let c = game.db.get_card(game.id(replaced_no)).map(|x| x.cost).flatten();
            format!("{:?}", c)
        }
    );
}

#[test]
fn pr0045_baton_over_cost7_draws() {
    let db = load_real_database();
    run_flow(db, "PL!-sd1-007-SD"); // lilywhite 東條希, printed cost 7
}

#[test]
fn pr0045_baton_over_cost4_no_draw() {
    let db = load_real_database();
    run_flow(db, "PL!-sd1-001-SD"); // CYaRon 高海千歌, printed cost != 7
}
