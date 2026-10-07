// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::game_setup::*;
use rabuka_engine::card::*;
use rabuka_engine::game_state::*;
use rabuka_engine::player::*;


use rabuka_engine::bot::strategy_v7::*;


        fn db_real() -> rabuka_engine::Arc<CardDatabase> {
            rabuka_engine::card_loader::test_card_db().clone()
        }

    fn second_attacker_gs(db: &rabuka_engine::Arc<CardDatabase>) -> (GameState, Player, Player) {
        let p1 = Player::new("p1".into(), "P1".into(), true);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p2.is_first_attacker = false;
        let mut gs = GameState::new(p1.clone(), p2.clone(), rabuka_engine::Arc::clone(db));
        gs.current_phase = Phase::LiveCardSetSecondAttacker;
        gs.current_turn_phase = rabuka_engine::game_state::TurnPhase::Live;
        (gs, p1, p2)
    }

    fn add_member_deck(db: &CardDatabase, p: &mut Player, name: &str, n: usize) {
        let id = db.get_card_id(name).unwrap();
        assert_eq!(db.get_card(id).unwrap().card_no, name);
        for _ in 0..n {
            p.main_deck.cards.push(id);
        }
    }

    #[test]
    fn experiment_feasible_matches_engine_bucket_semantics() {
        let need = [0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut pool = [0i32; 8];
        pool[1] = 2;
        assert!(experiment_feasible(&pool, &need));
        pool[1] = 1;
        assert!(!experiment_feasible(&pool, &need));
        pool[7] = 1;
        assert!(experiment_feasible(&pool, &need));
        pool[7] = 0;
        pool[0] = 1;
        assert!(
            !experiment_feasible(&pool, &need),
            "colorless must not fill heart01"
        );
        let bucket_need = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut pool2 = [0i32; 8];
        pool2[0] = 2;
        assert!(!experiment_feasible(&pool2, &bucket_need));
        pool2[0] = 6;
        assert!(experiment_feasible(&pool2, &bucket_need));
        let surplus_need = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut pool3 = [0i32; 8];
        pool3[1] = 2;
        assert!(
            experiment_feasible(&pool3, &surplus_need),
            "surplus feeds bucket"
        );
        pool3[1] = 0;
        pool3[2] = 2;
        assert!(
            !experiment_feasible(&pool3, &surplus_need),
            "wrong color cannot fill specific"
        );
    }

    #[test]
    fn experiment_pass_probability_is_per_color_and_deterministic() {
        let mut cats: Vec<([i32; 8], usize)> = Vec::new();
        let mut v1 = [0i32; 8];
        v1[1] = 1;
        cats.push((v1, 3));
        let mut v6 = [0i32; 8];
        v6[6] = 1;
        cats.push((v6, 3));
        let need = [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        let board = [0i32; 8];
        let p = experiment_pass_probability(&cats, 6, 2, &board, &need);
        let p2 = experiment_pass_probability(&cats, 6, 2, &board, &need);
        assert!((p - p2).abs() < f64::EPSILON, "deterministic");
        assert!(
            (p - 0.75).abs() < 0.15,
            "two flips over a half h06 deck should land near P(>=1 of 2)=0.75, got {p}"
        );
        let h06_only = [0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0];
        let p_two = experiment_pass_probability(&cats, 6, 2, &board, &h06_only);
        assert!(
            p_two < p && (p_two - 0.2).abs() < 0.08,
            "without replacement: two h06 has probability 3/6 * 2/5, got {p_two}"
        );
        let need_zero = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            experiment_pass_probability(&cats, 6, 2, &board, &need_zero),
            1.0
        );
        let mut colorless = [0i32; 8];
        colorless[0] = 3;
        let bucket_need = [3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert_eq!(
            experiment_pass_probability(&cats, 6, 0, &colorless, &bucket_need),
            1.0
        );
        assert_eq!(
            experiment_pass_probability(&cats, 6, 0, &colorless, &need),
            0.0,
            "colorless cannot fill h06 even with pool"
        );
    }

    #[test]
    fn experiment_ignores_deck_order_and_opponent_hidden_cards() {
        let db = db_real();
        let (mut gs, _, _) = second_attacker_gs(&db);
        let live = db.get_card_id("PL!-sd1-019-SD").unwrap();
        assert_eq!(db.get_card(live).unwrap().card_no, "PL!-sd1-019-SD");
        gs.player2.hand.cards.push(live);
        add_member_deck(&db, &mut gs.player2, "PL!-sd1-005-SD", 8);
        add_member_deck(&db, &mut gs.player2, "PL!-sd1-002-SD", 8);
        let member = db.get_card_id("PL!-sd1-010-SD").unwrap();
        gs.player2.stage.stage = [member, -1, -1];
        gs.mods.set_blade_modifier(member, 8);
        let before = experiment_flip_categories(&gs, 1, &db);
        let rank = experiment_portfolio_rank(&gs, 1, &db);
        gs.player2.main_deck.cards.reverse();
        gs.player1.hand.cards.extend([live, member]);
        gs.player1.main_deck.cards.extend([member, live, live]);
        assert_eq!(before, experiment_flip_categories(&gs, 1, &db));
        assert_eq!(rank, experiment_portfolio_rank(&gs, 1, &db));
        assert!(experiment_free_win(&gs, 1, &db).is_none());
        gs.mods.heart_modifiers.entry(member).or_default().insert(
            rabuka_engine::card::HeartColor::Heart06,
            rabuka_engine::core::game_modifiers::ModifierEntry {
                additive: 1,
                ..Default::default()
            },
        );
        assert_eq!(experiment_free_win(&gs, 1, &db), Some(0));
        let mut guard = 0;
        while gs.current_phase == Phase::LiveCardSetSecondAttacker && guard < 4 {
            let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
            let action = choose_live_set_experiment(&gs, &actions, &db);
            rabuka_engine::game_setup::execute_action(&mut gs, &action).unwrap();
            guard += 1;
        }
        assert!(guard < 4, "selection must converge to confirmation");
    }

    #[test]
    fn free_win_requires_true_board_coverage_not_mean_flips() {
        let db = db_real();
        let live = db.get_card_id("PL!-sd1-019-SD").unwrap();
        let need = db.get_card(live).unwrap().need_heart.clone().unwrap();
        assert_eq!(need.hearts.len(), 3);

        let id005 = db.get_card_id("PL!-sd1-005-SD").unwrap();
        let id010 = db.get_card_id("PL!-sd1-010-SD").unwrap();
        let id002 = db.get_card_id("PL!-sd1-002-SD").unwrap();

        let (mut gs, _p1, mut p2) = second_attacker_gs(&db);
        p2.hand.add_card(live);
        add_member_deck(&db, &mut p2, "PL!-sd1-005-SD", 8);
        add_member_deck(&db, &mut p2, "PL!-sd1-010-SD", 8);
        add_member_deck(&db, &mut p2, "PL!-sd1-002-SD", 8);
        p2.stage.stage = [id005, id010, id002];
        gs.player1 = _p1;
        gs.player2 = p2;
        let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
        let chosen = choose_live_set_experiment(&gs, &actions, &db);
        assert_eq!(
            chosen.action_type,
            ActionType::SelectLiveCard,
            "full board coverage must take the free win"
        );

        let (mut gs2, p1b, mut p2b) = second_attacker_gs(&db);
        p2b.hand.add_card(live);
        add_member_deck(&db, &mut p2b, "PL!-sd1-005-SD", 12);
        add_member_deck(&db, &mut p2b, "PL!-sd1-010-SD", 12);
        p2b.stage.stage = [id010, -1, -1];
        gs2.player1 = p1b;
        gs2.player2 = p2b;
        let actions2 = rabuka_engine::game_setup::generate_possible_actions(&gs2);
        let chosen2 = choose_live_set_experiment(&gs2, &actions2, &db);
        assert_ne!(
            chosen2.action_type,
            ActionType::SelectLiveCard,
            "h06 missing from board and no b_heart06 flips: must not set the live"
        );
    }
