// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
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


use rabuka_engine::bot::strategy::*;


    fn view() -> SideView {
        SideView {
            success_count: 0,
            stage_total_cost: 0,
            stage_hearts: 0,
            stage_blades: 0,
            active_energy: 0,
            hand_size: 0,
            hand_live_cards: 0,
            is_first_attacker: false,
        }
    }

    fn eval(my: &SideView, opp: &SideView) -> f64 {
        let w = StrategyWeights::fair();
        combine(
            my,
            opp,
            &w,
            1.0,
            my.success_count as f64 - opp.success_count as f64,
        )
    }

    /// Every field `combine` differences must contribute with its true sign.
    ///
    /// The evaluator used to write these as `(my.x - opp.x) as f64`, which
    /// subtracts in the *integer* type first. Four of the fields are `u8`, so
    /// every position where the opponent held more of one underflowed: debug
    /// builds panicked, release wrapped, and `0 - 3` became `+759` points —
    /// larger than the whole `success_card` weight, so the policy came out
    /// preferring LESS energy. Asserting the sign on both sides of the
    /// subtraction is what makes that unreintroducible: it does not care what
    /// the weights are, only that more is better and less is worse.
    #[test]
    fn evaluator_differences_keep_their_sign() {
        let fields: [(&str, fn(&mut SideView, u8), f64); 6] = [
            (
                "success_count",
                |v, x| v.success_count = x,
                StrategyWeights::fair().success_card,
            ),
            (
                "stage_total_cost",
                |v, x| v.stage_total_cost = i16::from(x),
                StrategyWeights::fair().stage_cost,
            ),
            (
                "stage_hearts",
                |v, x| v.stage_hearts = i16::from(x),
                StrategyWeights::fair().heart,
            ),
            (
                "stage_blades",
                |v, x| v.stage_blades = i16::from(x),
                StrategyWeights::fair().blade,
            ),
            (
                "active_energy",
                |v, x| v.active_energy = x,
                StrategyWeights::fair().active_energy,
            ),
            (
                "hand_size",
                |v, x| v.hand_size = x,
                StrategyWeights::fair().hand_size,
            ),
        ];

        for (name, set, weight) in fields {
            let mine_ahead = {
                let mut v = view();
                set(&mut v, 5);
                v
            };
            let opponent_ahead = {
                let mut v = view();
                set(&mut v, 5);
                v
            };
            let opponent_level = view();
            let mine_level = view();

            let ahead = eval(&mine_ahead, &opponent_level);
            let behind = eval(&mine_level, &opponent_ahead);
            assert!(
                ahead > behind,
                "`{name}` is scored backwards: holding 5 scores {ahead}, \
                 holding 0 against 5 scores {behind}. A `(a - b) as f64` on an \
                 unsigned field wraps instead of going negative."
            );
            // 5 ahead vs 5 behind is a swing of 2 * 5 * weight.
            let want = 10.0 * weight;
            assert!(
                (ahead - behind - want).abs() < 1e-9,
                "`{name}` swing is {}, expected {want}",
                ahead - behind
            );
        }
    }

    /// `hand_live_cards` is the one field scored without a difference (it only
    /// ever rewards our own side), so it must not be double-counted or sign-
    /// flipped by the same class of edit.
    #[test]
    fn own_hand_live_cards_only_ever_helps() {
        let with = {
            let mut v = view();
            v.hand_live_cards = 3;
            eval(&v, &view())
        };
        let without = eval(&view(), &view());
        assert!(
            with > without,
            "live cards in our own hand must raise our score: {with} vs {without}"
        );
        assert_eq!(
            (with - without - 3.0 * StrategyWeights::fair().hand_live_card).abs(),
            0.0,
            "own-hand live cards must contribute exactly count * weight"
        );
    }
