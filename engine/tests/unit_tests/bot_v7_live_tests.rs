// `feasible` exists in both bot::v7_live and bot::v8_model and both
// are glob-imported below; name the one under test explicitly.
use rabuka_engine::bot::v7_live::feasible;
// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::bot::strategy_common::*;
use rabuka_engine::card::*;
use rabuka_engine::game_state::*;


use rabuka_engine::bot::v7_live::*;

    use rabuka_engine::player::Player;

        fn fixture() -> (GameState, rabuka_engine::Arc<CardDatabase>) {
            let db = rabuka_engine::card_loader::test_card_db().clone();
        let p1 = Player::new("p1".into(), "P1".into(), true);
        let p2 = Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
        gs.current_phase = Phase::LiveCardSetSecondAttacker;
        gs.current_turn_phase = rabuka_engine::game_state::TurnPhase::Live;
        (gs, db)
    }

    fn card(db: &CardDatabase, name: &str) -> i16 {
        let id = db.get_card_id(name).unwrap();
        assert_eq!(db.get_card(id).unwrap().card_no, name);
        id
    }

    #[test]
    fn wrong_color_total_approximation_and_joint_allocation() {
        let wrong: Pool = [0, 0, 2, 0, 0, 0, 0, 0];
        let need: Acc = [1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(wrong.iter().map(|&v| i32::from(v)).sum::<i32>()
            >= need.iter().map(|&v| i32::from(v)).sum::<i32>());
        assert!(!feasible(&wrong, &need));
        assert!(feasible(&[0, 2, 0, 0, 0, 0, 0, 0], &need));
        assert!(feasible(&[1, 0, 0, 0, 0, 0, 0, 1], &need));
        let mut impossible = need;
        impossible[8] = 1;
        assert!(!feasible(&[20; 8], &impossible));
        println!("wrong-color total approximation=true; joint color feasibility=false; surplus-specific joint allocation=true");
    }

    #[test]
    fn real_double_grey_icon_uses_engine_conversion() {
        let (_, db) = fixture();
        let cid = card(&db, "PL!S-bp7-022-L");
        let mut total = [0u8; 8];
        let mut scratch = BaseHeart { hearts: HeartMap::new() };
        let mut cheer = 0;
        rabuka_engine::turn::live::process_yell_revealed_card_icons(
            db.get_card(cid).unwrap(), None, &mut scratch, &mut total, &mut cheer,
        );
        assert_eq!(total, [2, 0, 0, 0, 0, 0, 0, 0]);
        let pools = sample_pools(vec![total.map(i16::from)], 1, [0; 8]);
        let grey_need: Acc = [2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(pools.iter().all(|p| feasible(p, &grey_need)));
        assert!(!feasible(&[1, 0, 0, 0, 0, 0, 0, 0], &grey_need));
        assert!(!feasible(&total.map(i16::from), &[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0]));
        println!("PL!S-bp7-022-L one b_heart07: old one-unit approximation fails need2; authoritative two-grey passes need2, cannot satisfy specific color");
    }

    #[test]
    fn sampling_matches_without_replacement_probability() {
        let red: Pool = [0, 1, 0, 0, 0, 0, 0, 0];
        let blue: Pool = [0, 0, 0, 0, 0, 0, 1, 0];
        let deck = vec![red, red, red, blue, blue, blue];
        let pools = sample_pools(deck.clone(), 2, [0; 8]);
        let need: Acc = [0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        let p = pools.iter().filter(|pool| feasible(pool, &need)).count() as f64 / SAMPLES as f64;
        assert!((p - 0.8).abs() < 0.08, "exact hypergeometric probability=.8 sampled={p}");
        let mut reversed = deck;
        reversed.reverse();
        assert_eq!(pools, sample_pools(reversed, 2, [0; 8]));
    }

    #[test]
    fn real_candidates_ignore_order_hidden_cards_and_engine_rng() {
        let (mut gs, db) = fixture();
        let live = card(&db, "PL!-sd1-019-SD");
        let a = card(&db, "PL!-sd1-005-SD");
        let b = card(&db, "PL!-sd1-010-SD");
        gs.player2.hand.cards.push(live);
        gs.player2.stage.stage = [b, -1, -1];
        gs.player2.main_deck.cards.extend([a, b, a, b]);
        let rng = rabuka_engine::rng::checkpoint();
        let before = candidate_metrics(&gs, &db);
        assert_eq!(rng, rabuka_engine::rng::checkpoint());
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].heart_pass_probability, 0.0);
        gs.player2.main_deck.cards.reverse();
        gs.player1.hand.cards.extend([live, b]);
        gs.player1.main_deck.cards.extend([b, live]);
        gs.player1.live_card_zone.cards.push(live);
        let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
        let first = choose_live_set(&gs, &actions, &db);
        gs.player1.live_card_zone.cards[0] = b;
        assert_eq!(before, candidate_metrics(&gs, &db));
        let again = choose_live_set(&gs, &actions, &db);
        assert_eq!(first.action_type, again.action_type);
        assert_eq!(first.description, again.description);
        assert_eq!(first.selected, again.selected);
        assert_eq!(rng, rabuka_engine::rng::checkpoint());
    }

    #[test]
    fn real_free_win_sets_one_live_and_converges() {
        let (mut gs, db) = fixture();
        let live = card(&db, "PL!-sd1-019-SD");
        let a = card(&db, "PL!-sd1-005-SD");
        let b = card(&db, "PL!-sd1-010-SD");
        let c = card(&db, "PL!-sd1-002-SD");
        gs.player2.stage.stage = [a, b, c];
        gs.player2.hand.cards.extend([live, live, a]);
        gs.player2.main_deck.cards.extend([a, b, c, live]);
        let metrics = candidate_metrics(&gs, &db);
        let selected = select_candidate(&metrics, true, false, 0.45, 0.25).unwrap();
        assert!(metrics[selected].board_pass);
        assert_eq!(metrics[selected].hand_indices.len(), 1);
        assert!(metrics.iter().all(|c| c.placement_upper_bound <= 1.0));
        let mut confirmed = false;
        for _ in 0..4 {
            let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
            let action = choose_live_set(&gs, &actions, &db);
            let confirming = action.action_type == rabuka_engine::game_setup::ActionType::ConfirmLiveCardSet;
            rabuka_engine::game_setup::execute_action(&mut gs, &action).unwrap();
            if confirming {
                confirmed = true;
                break;
            }
        }
        assert!(confirmed);
        assert_eq!(gs.player2.live_card_zone.cards.len(), 1);
    }
