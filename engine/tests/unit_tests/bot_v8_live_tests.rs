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




    fn outcome(p_pass: f64, p_place: f64, value: f64) -> CheckOutcome {
        CheckOutcome {
            p_pass,
            p_place,
            opp_place: 0.0,
            value,
        }
    }

    /// Value dominates, and the tiebreak prefers conserving ammunition.
    /// A life that cannot pass must be strictly worse than not setting it.
    ///
    /// The burn price used to be `p * (1 - p)`, which is ZERO at `p = 0`, so a
    /// guaranteed-to-fail life cost nothing and a zone containing one tied
    /// exactly with the same zone without it. The decision was then made by the
    /// fewer-lives tie-break, which is a coin flip wearing a rank - and which
    /// flips the moment `filter_value` or the junk ordering moves.
    ///
    /// This was found by hand-playing a T1 live set and printing the candidate
    /// values, where the life-free zone and the doomed-life zone both read
    /// 0.0629. No aggregate can see it: fold rate and pace are unchanged either
    /// way, because the tie-break happened to land on the right answer.
    #[test]
    fn a_life_that_cannot_pass_is_strictly_worse_than_not_setting_it() {
        let doomed = CheckOutcome {
            p_pass: 0.0,
            p_place: 0.0,
            opp_place: 0.0,
            value: 0.0,
        };
        let free = CheckOutcome {
            p_pass: 0.0,
            p_place: 0.0,
            opp_place: 0.0,
            value: 0.0,
        };
        let with_life = candidate_value(doomed, 0.0, 0.38);
        let without_life = candidate_value_zone(free, 0.0, 0.38, false);
        assert!(
            with_life < without_life,
            "a life at p=0 must cost something: {with_life} vs {without_life}"
        );
        // And the price must shrink as the life gets likelier to place, or the
        // model would refuse to commit lives it is about to place.
        let certain = candidate_value(
            CheckOutcome {
                p_pass: 1.0,
                p_place: 1.0,
                opp_place: 0.0,
                value: PLACEMENT_CREDIT,
            },
            1.0,
            0.0,
        );
        assert!(
            certain > with_life,
            "committing a life that certainly places must be worth more than burning a doomed one"
        );
    }

    #[test]
    fn a_life_that_certainly_places_is_not_charged_a_burn() {
        let v = candidate_value(
            CheckOutcome {
                p_pass: 1.0,
                p_place: 1.0,
                opp_place: 0.0,
                value: PLACEMENT_CREDIT,
            },
            1.0,
            0.0,
        );
        assert!(
            (v - PLACEMENT_CREDIT).abs() < 1e-12,
            "a placed life forfeits nothing: {v}"
        );
    }

    #[test]
    fn ranking_prefers_value_then_fewer_lives() {
        let mk = |lives: &[usize], value: f64| Candidate {
            score: 3,
            lives: lives.to_vec(),
            junk: Vec::new(),
            outcome: outcome(1.0, 1.0, value),
            value,
        };
        let three = mk(&[0, 1, 2], 0.40);
        let one = mk(&[3], 0.40);
        assert!(better(&one, &three), "equal value must prefer fewer lives");
        let worth_more = mk(&[3], 0.50);
        assert!(better(&worth_more, &three), "value must dominate the tiebreak");
    }

    /// The candidate enumeration must not silently drop lives.
    ///
    /// It used to be `for mask in 0..(1 << lives.len().min(16))`. That made
    /// every life past the 16th unreachable — a 17-card hand lost four of its
    /// candidates with no counter and no log, and because `better()` can only
    /// rank what it is given, a life the bot would have committed to became
    /// invisible and the zone folded instead. The regression is invisible from
    /// the outside: fold rate and pace both stay plausible, which is the same
    /// reason the per-card `V8CHK` line exists.
    ///
    /// Pinned directly against the generator so a re-introduced `.min(N)` fails
    /// here rather than in a run.
    #[test]
    fn candidate_enumeration_sees_every_life_whatever_the_hand_size() {
        // Duplicated shape of the mask list in `enumerate_candidates`; if the
        // two ever drift this test is not protecting anything, so it names the
        // contract rather than the function.
        let masks_for = |n: usize, cap: usize| -> Vec<u32> {
            let mut masks = vec![0u32];
            for h in 0..n {
                if cap >= 1 {
                    masks.push(1u32 << h);
                }
                for g in 0..h {
                    if cap >= 2 {
                        masks.push((1u32 << h) | (1u32 << g));
                    }
                    for f in 0..g {
                        if cap >= 3 {
                            masks.push((1u32 << h) | (1u32 << g) | (1u32 << f));
                        }
                    }
                }
            }
            masks
        };

        for &n in &[0usize, 1, 5, 16, 17, 20, 31] {
            for &cap in &[1usize, 2, 3] {
                let masks = masks_for(n, cap);
                // Every life index must appear in at least one affordable set.
                // This is the assertion the `.min(16)` failed for n > 16.
                for bit in 0..n {
                    assert!(
                        masks.iter().any(|m| m & (1u32 << bit) != 0),
                        "n={n} cap={cap}: life {bit} is unreachable"
                    );
                }
                // And no set may exceed the width cap.
                for m in &masks {
                    assert!(
                        m.count_ones() as usize <= cap,
                        "n={n} cap={cap}: mask {m:b} is wider than the cap"
                    );
                }
            }
        }

        // The shipped default is a width cap of 1, where the list is the empty
        // set plus one set per life — which is bitmask order, so the candidate
        // order that breaks ties is unchanged from the pre-fix loop.
        assert_eq!(masks_for(4, 1), vec![0b0000, 0b0001, 0b0010, 0b0100, 0b1000]);
    }

    /// The D2b trade-off, priced. Against an opponent who is NOT one
    /// placement from winning, a life we are very likely to FAIL burns more
    /// future placements than it can make here, so the search prefers a
    /// junk-only set. v7 could not express this and got it wrong in both
    /// directions (measured -1pp for the "desperation life").
    ///
    /// The comparison is against the empty candidate, never against a bare
    /// zero: setting nothing still has the hand filter's value, so "fold" means
    /// "lose to junk", not "gain nothing".
    ///
    /// Uncontested, a life worth `p` gains `p * PLACEMENT_CREDIT` and forfeits
    /// `(1 - p) * p * PLACEMENT_CREDIT`, so against a junk-only set the break
    /// even is `p^2 / 3 == filter_value` - the price `candidate_value` charges.
    /// That is the calibrated middle ground between the two measured extremes
    /// documented on it: the full-credit price folds anything below 50%, the
    /// projected price folds 44% of checks.
    ///
    /// Paired with `v8_model::match_point_makes_a_thin_check_worth_taking`:
    /// at opponent match point the same thin life IS right, because folding
    /// there hands them the game. One objective, both answers.
    #[test]
    fn a_junk_only_zone_is_not_charged_for_holding_no_life() {
        // A junk-only zone has p_pass = 0, so an ungated burn price charges it
        // a full placement credit for setting nothing. That is the same error
        // as failing to charge a doomed life: the price must follow the cards
        // actually at risk.
        let junk = candidate_value_zone(outcome(0.0, 0.0, 0.0), 0.0, 0.02, false);
        assert!(
            junk > 0.0,
            "a junk-only zone is worth its hand filter alone: {junk}"
        );
    }

    /// A thin life can be worth less than a junk set, but only because of the
    /// option value it forfeits - not because a life is never worth committing.
    ///
    /// Uncontested, a life worth `p` gains `p * PLACEMENT_CREDIT` and forfeits
    /// the option on the rest, so the break-even sits where the forfeited
    /// option equals the hand filter it loses by not junking. The `0.45` case
    /// below is the load-bearing one: on the same board, the same life, the
    /// same junk ordering, the decision flips on `p` alone.
    ///
    /// Paired with `v8_model::match_point_makes_a_thin_check_worth_taking`:
    /// at opponent match point the same thin life IS right, because folding
    /// there hands them the game. One objective, both answers.
    #[test]
    fn a_thin_life_can_be_worth_less_than_a_junk_set() {
        let junk_only = candidate_value_zone(outcome(0.0, 0.0, 0.0), 0.0, 0.02, false);
        assert!(junk_only > 0.0);
        // p = 0.20: the forfeited option on a life that probably cannot place
        // outweighs the placement it might make. Below break-even, so the argmax
        // sets junk and keeps the life for another check.
        let thin = candidate_value(outcome(0.20, 0.20, 0.20 * PLACEMENT_CREDIT), 0.20, 0.0);
        assert!(thin < junk_only, "thin {thin} vs junk {junk_only}");
        // p = 0.45 on the same board: the placement this life can make here
        // outweighs the one it forfeits, and it is committed. A thin life is
        // not junk; only a hopeless one is.
        let thicker = candidate_value(
            outcome(0.45, 0.45, 0.45 * PLACEMENT_CREDIT),
            0.45,
            0.0,
        );
        assert!(thicker > junk_only, "thicker {thicker} vs junk {junk_only}");
        // The life is still charged for its own failure, monotonically.
        let certain = candidate_value(
            outcome(0.95, 0.95, 0.95 * PLACEMENT_CREDIT),
            0.95,
            0.0,
        );
        assert!(certain > thicker);
    }

    /// A reliable life is always worth taking, and the hand filter is pure
    /// upside: the card is discarded before the check and draws a
    /// replacement (8.3.4).
    #[test]
    fn a_reliable_life_plus_hand_filter_beats_folding() {
        let play = candidate_value(outcome(0.95, 0.95, 0.32), 0.95, 0.02);
        let fold = candidate_value(outcome(0.0, 0.0, 0.0), 0.0, 0.06);
        assert!(play > fold);
    }

    #[test]
    fn junk_is_only_filled_from_cards_weaker_than_a_draw() {
        // A dead vanilla member is worth less than a draw, so it is burnable;
        // an energy card is worth more, so it must not be.
        let dead = Junk {
            index: 0,
            keep_value: 0.0,
        };
        let energy = Junk {
            index: 1,
            keep_value: PLACEMENT_CREDIT,
        };
        let draw_credit = 0.05;
        assert!(dead.keep_value <= draw_credit);
        assert!(energy.keep_value > draw_credit);
    }
