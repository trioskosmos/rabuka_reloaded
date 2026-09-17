use crate::helpers::*;
use rabuka_engine::zones::MemberArea;

#[test]
fn pl_hs_bp2_009_r_baton_cheaper_mirakura_fire_wrong_group_or_cost_no_fire() {
    let db = load_real_database();
    use rabuka_engine::card::HeartColor;
    {
        let mut g = TestGame::new(db.clone());
        let himeno = g.id("PL!HS-bp2-009-R");
        let non_mirakura = g.id("PL!-sd1-010-SD");
        g.state.player1.stage.stage[1] = non_mirakura;
        g.state.player1.hand.cards.push(himeno);
        g.give_energy(20);
        g.play_to_stage(himeno, MemberArea::Center);
        assert!(
            g.has_pending_choice(),
            "Test1: Should have optional cost prompt"
        );
        g.select_option(1);
        assert!(
            !g.has_pending_choice(),
            "Test1: No further choice — condition should fail (wrong group)"
        );
        let heart01 = g.state.mods.get_heart_modifier(himeno, HeartColor::Heart01);
        assert_eq!(
            heart01, 0,
            "Test1: Should NOT gain heart01 — baton touch source is not みらくらぱーく！"
        );
    }
    {
        let mut g = TestGame::new(db.clone());
        let himeno = g.id("PL!HS-bp2-009-R");
        let mirakura_low = g.id("PL!HS-sd1-011-SD");
        g.state.player1.stage.stage[1] = mirakura_low;
        g.state.player1.hand.cards.push(himeno);
        g.give_energy(20);
        g.play_to_stage(himeno, MemberArea::Center);
        assert!(
            g.has_pending_choice(),
            "Test2: Should have optional cost prompt"
        );
        g.select_option(1);
        let heart01 = g
            .state
            .mods
            .get_heart_modifier(g.state.player1.stage.stage[1], HeartColor::Heart01);
        assert_eq!(
            heart01, 2,
            "Test2: Should gain 2 heart01 — correct group + lower cost"
        );
    }
    {
        let mut g = TestGame::new(db.clone());
        let himeno = g.id("PL!HS-bp2-009-R");
        let mirakura_high = g.id("PL!HS-bp2-006-R");
        g.state.player1.stage.stage[1] = mirakura_high;
        g.state.player1.hand.cards.push(himeno);
        g.give_energy(20);
        g.play_to_stage(himeno, MemberArea::Center);
        assert!(
            g.has_pending_choice(),
            "Test3: Should have optional cost prompt"
        );
        g.select_option(1);
        let heart01 = g
            .state
            .mods
            .get_heart_modifier(g.state.player1.stage.stage[1], HeartColor::Heart01);
        assert_eq!(
            heart01, 0,
            "Test3: Should NOT gain heart01 — baton touch source has higher cost"
        );
    }
}
