use crate::helpers::*;

fn pl_bp3_012_pr_advance_to_live_card_set_p1(game: &mut TestGame) {
    game.pass();
    game.pass();
    game.pass();
    game.pass();
    game.pass();
}
fn pl_bp3_012_pr_advance_to_live_start(game: &mut TestGame) {
    game.pass();
    game.pass();
}

#[test]
fn pl_bp3_012_pr_chosen_heart_color_scales_with_success_zone_count() {
    let db = load_real_database();
    use rabuka_engine::card::HeartColor;
    {
        let mut g = TestGame::new(db.clone());
        let kotori = g.id("PL!-bp3-012-PR");
        let live = g.id("PL!-sd1-010-SD");
        g.state.player2.hand.cards.push(g.new_id("PL!-sd1-010-SD"));
        g.state.player1.stage.stage[1] = kotori;
        g.state.player1.hand.cards.push(live);
        g.give_energy(10);
        let filler = g.new_id("PL!-sd1-010-SD");
        for _ in 0..10 {
            g.state.player1.main_deck.cards.push(filler);
        }
        for _ in 0..10 {
            g.state.player2.main_deck.cards.push(filler);
        }
        pl_bp3_012_pr_advance_to_live_card_set_p1(&mut g);
        g.set_live_card(live);
        pl_bp3_012_pr_advance_to_live_start(&mut g);
        assert!(g.has_pending_choice(), "Should prompt for heart color");
        g.select_option(0);
        let heart01 = g.state.mods.get_heart_modifier(kotori, HeartColor::Heart01);
        assert_eq!(
            heart01, 0,
            "0 cards in success zone → 0 hearts (got {})",
            heart01
        );
    }
    {
        let mut g = TestGame::new(db.clone());
        let kotori = g.id("PL!-bp3-012-PR");
        let live = g.id("PL!-sd1-010-SD");
        g.state.player2.hand.cards.push(g.new_id("PL!-sd1-010-SD"));
        g.state.player1.stage.stage[1] = kotori;
        g.state.player1.hand.cards.push(live);
        for _ in 0..3 {
            g.state
                .player1
                .success_live_card_zone
                .cards
                .push(g.new_id("PL!-sd1-010-SD"));
        }
        g.give_energy(10);
        let filler = g.new_id("PL!-sd1-010-SD");
        for _ in 0..10 {
            g.state.player1.main_deck.cards.push(filler);
        }
        for _ in 0..10 {
            g.state.player2.main_deck.cards.push(filler);
        }
        pl_bp3_012_pr_advance_to_live_card_set_p1(&mut g);
        g.set_live_card(live);
        pl_bp3_012_pr_advance_to_live_start(&mut g);
        assert!(
            g.has_pending_choice(),
            "Should prompt for heart color (3 cards)"
        );
        g.select_option(0);
        let heart01 = g.state.mods.get_heart_modifier(kotori, HeartColor::Heart01);
        assert_eq!(
            heart01, 3,
            "3 cards in success zone → 3 hearts (got {})",
            heart01
        );
    }
}
