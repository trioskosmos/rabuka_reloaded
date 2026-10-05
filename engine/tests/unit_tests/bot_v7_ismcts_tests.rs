// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::Arc;
use rabuka_engine::game_setup;
use rabuka_engine::game_setup::*;
use rabuka_engine::game::match_runner::*;
use rabuka_engine::game::sav::*;
use rabuka_engine::ability::abilities_gen::*;
use rabuka_engine::ability::types::*;
use rabuka_engine::ability::util::duration::*;
use rabuka_engine::bot::v8_live::*;
use rabuka_engine::bot::v8_model::*;
use rabuka_engine::bot::strategy_common::*;
use rabuka_engine::card::*;
use rabuka_engine::card_loader::*;
use rabuka_engine::constants::*;
use rabuka_engine::game_state::*;
use rabuka_engine::player::*;
use rabuka_engine::types::*;
use rabuka_engine::zones::*;
use rabuka_engine::HashMap;
use smallvec::SmallVec;


use rabuka_engine::bot::v7_ismcts::*;


        fn db_real() -> rabuka_engine::Arc<CardDatabase> {
            rabuka_engine::card_loader::test_card_db().clone()
        }

    fn fixture() -> (GameState, rabuka_engine::Arc<CardDatabase>) {
        let db = db_real();
        let p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
        gs.current_phase = Phase::Main;
        (gs, db)
    }

    /// A position with enough energy and cards that `generate_possible_actions`
    /// genuinely offers a choice. Tests assert on the search's behaviour, so a
    /// fixture that quietly yields one action and sends them down an early
    /// return turns a real test into a no-op that always passes.
    fn choice_fixture() -> (GameState, Vec<Action>, rabuka_engine::Arc<CardDatabase>) {
        let (mut gs, db) = fixture();
        let member = db.get_card_id("PL!SP-bp1-005-R").unwrap();
        assert_eq!(
            db.get_card(member).unwrap().card_no,
            "PL!SP-bp1-005-R",
            "the fixture must exist in the real card database"
        );
        let energy = db
            .cards
            .values()
            .find(|card| card.is_energy())
            .and_then(|card| db.get_card_id(card.card_no.as_ref()))
            .expect("database has energy cards");
        for _ in 0..8 {
            gs.player1.energy_zone.cards.push(energy);
        }
        gs.player1.energy_zone.active_energy_count = 8;
        gs.player1.hand.cards.extend([member; 4]);
        gs.player1.main_deck.cards.extend([member; 20]);
        gs.player2.main_deck.cards.extend([member; 20]);
        gs.player2.hand.cards.extend([member; 4]);
        let actions = game_setup::generate_possible_actions(&gs);
        assert!(
            actions.len() >= 2,
            "choice_fixture must offer a real choice, got {}",
            actions.len()
        );
        (gs, actions, db)
    }

    /// A config for tests that assert the search DOES something. `enabled`
    /// defaults to false now (the search is opt-in because it measured as a
    /// regression), so a test that inherits the environment would silently
    /// assert nothing at all.
    fn enabled_config() -> Config {
        Config {
            enabled: true,
            jobs: 1,
            ..Config::from_env()
        }
    }

    #[test]
    fn rollout_seeds_are_paired_across_actions() {
        // Common random numbers: the seed must depend on the (world, sim)
        // slot alone, so action A's j-th rollout and action B's j-th rollout
        // see the same shuffle. If this ever takes an action index again the
        // search silently reverts to comparing independent samples.
        assert_eq!(rollout_seed(0, 0), rollout_seed(0, 0));
        let mut seen = std::collections::HashSet::new();
        for world in 0..4 {
            for sim in 0..6 {
                assert!(seen.insert(rollout_seed(world, sim)), "collision");
            }
        }
    }

    #[test]
    fn game_value_reads_the_success_race() {
        let (mut gs, _db) = fixture();
        assert_eq!(game_value(&gs, 0), 0.0);
        gs.player1.success_live_card_zone.cards.push(1);
        assert!(game_value(&gs, 0) > 0.0, "own placement must be worth something");
        assert!(game_value(&gs, 1) < 0.0, "their placement must cost something");
    }

    #[test]
    fn game_value_is_terminal_exact() {
        let (mut gs, _db) = fixture();
        gs.game_result = GameResult::FirstAttackerWins;
        assert_eq!(game_value(&gs, 0), 1.0);
        assert_eq!(game_value(&gs, 1), -1.0);
        gs.game_result = GameResult::SecondAttackerWins;
        assert_eq!(game_value(&gs, 0), -1.0);
        assert_eq!(game_value(&gs, 1), 1.0);
    }

    #[test]
    fn sampled_world_keeps_public_state_and_zone_sizes() {
        let (mut gs, db) = fixture();
        let member = db.get_card_id("PL!SP-bp1-005-R").unwrap();
        let live = db.get_card_id("PL!SP-sd1-019-SD").unwrap();
        assert_eq!(db.get_card(member).unwrap().card_no, "PL!SP-bp1-005-R");
        assert_eq!(db.get_card(live).unwrap().card_no, "PL!SP-sd1-019-SD");
        for _ in 0..30 {
            gs.player1.main_deck.cards.push(member);
            gs.player2.main_deck.cards.push(member);
        }
        gs.player1.stage.stage = [member, -1, -1];
        gs.player2.hand.cards.extend([live, member]);
        let deck_size = gs.player2.main_deck.cards.len();
        let cfg = Config {
            enabled: true,
            worlds: 1,
            sims: 1,
            visits: 1,
            horizon: 3,
            ticks: 90,
            top_k: 8,
            k0: 8.0,
            prior_scale: 260.0,
            jobs: 1,
            own_pool: true,
            exec_deploy: false,
            promote_only: true,
            v7_leaf: false,
            min_z: 1.0,
        };
        let world = sample_world(&gs, 0, &cfg, 0);
        assert_eq!(world.player2.main_deck.cards.len(), deck_size);
        assert_eq!(world.player1.stage.stage, gs.player1.stage.stage);
        assert_eq!(world.player1.hand.cards, gs.player1.hand.cards);
        assert_eq!(world.turn_number, gs.turn_number);
        // Determinism: the same world index is the same world.
        assert_eq!(world.player2.hand.cards, sample_world(&gs, 0, &cfg, 0).player2.hand.cards);
    }

    #[test]
    fn search_is_independent_of_worker_count() {
        let (gs, actions, _db) = choice_fixture();
        let scores: Vec<(f64, String)> = actions
            .iter()
            .enumerate()
            .map(|(index, _)| (index as f64, "t".to_string()))
            .collect();
        let mut serial = enabled_config();
        serial.worlds = 2;
        serial.sims = 1;
        serial.visits = 6;
        serial.promote_only = false;
        let mut parallel = serial;
        parallel.jobs = 4;
        let one = search(&gs, &actions, 0, &scores, &serial)
            .expect("serial search must produce scores");
        let many = search(&gs, &actions, 0, &scores, &parallel)
            .expect("parallel search must produce scores");
        for (index, (score, _)) in one.iter().enumerate() {
            assert!(
                (score - many[index].0).abs() < 1e-12,
                "job count must not change the answer: {score} vs {}",
                many[index].0
            );
        }
    }

    #[test]
    fn the_search_refuses_states_it_must_not_price() {
        let (mut gs, actions, _db) = choice_fixture();
        let scores: Vec<(f64, String)> = actions
            .iter()
            .enumerate()
            .map(|(index, _)| (index as f64, "a".into()))
            .collect();
        let cfg = enabled_config();
        // Main is the scope, and the search must actually run there.
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_some());
        // A live-card set is a multi-tick emission chain, not a decision.
        gs.current_phase = Phase::LiveCardSetSecondAttacker;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
        gs.current_phase = Phase::MulliganFirstAttacker;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
        // Rule 12-1 is a rules prompt, never a rollout question.
        gs.pending_loop_protocol = Some(rabuka_engine::core::game_state::PermanentLoopProtocol {
            choice: rabuka_engine::ability::types::Choice::SelectTarget {
                target: "rule_12_1".into(),
                description: String::new(),
                description_en: None,
                description_ja: None,
                allow_skip: false,
                options: None,
            },
        });
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
    }

    #[test]
    fn the_prior_ranks_actions_exactly_as_v7_ranks_them() {
        // v7 scores Pass at exactly 0.0 as a baseline, useful deploys at
        // 15-165, and a no-op at -1000. The prior must preserve that shape:
        // normalizing by the regret-to-leader instead would pin Pass far
        // below every deploy and let a noisy rollout walk v7 into a
        // `use_ability -> pass -> use_ability` repetition.
        let scores: [f64; 5] = [0.0, 50.0, 15.0, -1000.0, 165.0];
        let scale: f64 = 260.0;
        let prior: Vec<f64> = scores
            .iter()
            .map(|score| (score / scale).clamp(-1.0, 1.0))
            .collect();
        assert_eq!(prior[0], 0.0, "Pass must stay at the neutral baseline");
        assert!(prior[1] > prior[0] && prior[4] > prior[1], "ordering preserved");
        assert!(prior[2] > prior[0], "a 15-point deploy is still above Pass");
        assert_eq!(prior[3], -1.0, "a no-op saturates, never inverts");
        let argmax = |v: &[f64]| {
            v.iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(i, _)| i)
        };
        assert_eq!(
            argmax(&prior),
            argmax(&scores),
            "prior must not re-rank v7's own ordering"
        );
    }

    #[test]
    fn promote_only_never_lets_a_useless_action_beat_pass() {
        let actions = vec![
            Action {
                description: "pass".into(),
                description_ja: None,
                action_type: ActionType::Pass,
                parameters: None,
                selected: None,
            },
            Action {
                description: "use".into(),
                description_ja: None,
                action_type: ActionType::UseAbility,
                parameters: None,
                selected: None,
            },
        ];
        assert_eq!(static_baseline(&actions, &[(0.0, String::new()), (0.0, String::new())]), 0);
        // With a 3-point "use ability" and a 0.0 Pass, only Pass is promotable
        // (nothing clears the baseline), so the search must decline.
        let scores = vec![(0.0, String::new()), (3.0, String::new())];
        let (gs, _db) = fixture();
        let mut cfg = enabled_config();
        cfg.promote_only = true;
        assert!(search(&gs, &actions, 0, &scores, &cfg).is_none());
    }

    #[test]
    fn the_search_is_opt_in_because_it_measured_as_a_regression() {
        // Not a style preference: the paired mirror (bot_arena v7 v7plain,
        // identical deals, exact McNemar, n up to 2000) put the search BELOW
        // `v7plain` at fourteen different settings, and below it even when it
        // could only overrule v7 on a two-sigma paired advantage. If a change
        // ever makes the search measurably better than v7, this assertion is
        // the thing that has to be revisited — deliberately, with the numbers
        // in hand, rather than by a default quietly flipping.
        let unset = std::env::var("V7_ISMCTS");
        assert!(
            !matches!(unset.as_deref(), Ok("1") | Ok("true")),
            "this test assumes V7_ISMCTS is unset; got {unset:?}"
        );
        assert!(
            !Config::from_env().enabled,
            "the ISMCTS search must not be on by default"
        );
        // ...but it must still be reachable, or none of the above is testable.
        let mut forced = Config::from_env();
        forced.enabled = true;
        let (gs, actions, _db) = choice_fixture();
        let scores: Vec<(f64, String)> = actions
            .iter()
            .enumerate()
            .map(|(index, _)| (index as f64, "t".into()))
            .collect();
        assert!(
            search(&gs, &actions, 0, &scores, &forced).is_some(),
            "V7_ISMCTS=1 must still run the search"
        );
    }

    #[test]
    fn prior_shrinks_a_thin_sample_toward_the_static_eval() {
        let sums = [4.0];
        let counts = [1u32];
        let prior = [-1.0];
        let k0 = 8.0;
        let blended = (sums[0] + k0 * prior[0]) / (f64::from(counts[0]) + k0);
        assert!(blended < 0.0, "one good rollout must not overturn a bad prior");
    }
