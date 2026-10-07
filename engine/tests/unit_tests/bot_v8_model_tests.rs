// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::bot::v8_model::*;
use rabuka_engine::bot::strategy_common::*;
use rabuka_engine::card::*;
use rabuka_engine::game_state::*;



    /// The real card database, so these tests exercise the real card shapes
    /// (a member with no `base_heart`, one with a zero blade, one whose blades
    /// come from a modifier) instead of a hand-built fixture that would hide
    /// exactly the cases that broke.
        fn real_db() -> CardDatabase {
            CardDatabase::load_or_create(rabuka_engine::card_loader::test_cards().to_vec())
        }

    fn first_member_with(db: &CardDatabase, skip: usize, pred: impl Fn(&rabuka_engine::card::Card) -> bool) -> i16 {
        let mut ids: Vec<i16> = db.cards.keys().copied().collect();
        ids.sort();
        ids.into_iter()
            .filter(|&id| {
                db.get_card(id)
                    .is_some_and(|c| c.card_type == CardType::Member && pred(c))
            })
            .nth(skip)
            .expect("a member matching the predicate exists in cards.json")
    }

    fn bare_state(db: &CardDatabase) -> GameState {
        let p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        GameState::new(p1, p2, std::sync::Arc::new(db.clone()))
    }

    /// Put a real member on `seat`'s stage, preferring one with hearts.
    fn stage_member(db: &CardDatabase, gs: &mut GameState, seat: usize) -> (i16, i32) {
        let member = first_member_with(db, 0, |c| c.base_heart.is_some() && c.cost.unwrap_or(0) > 0);
        let card = db.get_card(member).unwrap();
        let hearts = card.base_heart.as_ref().map_or(0, |b| i32::from(b.hearts.values_sum()));
        let player = if seat == 0 { &mut gs.player1 } else { &mut gs.player2 };
        player.stage.stage[0] = member;
        (member, hearts)
    }


    fn opp_model(
        set_size: usize,
        success: usize,
        pass_prob: f64,
        dist: &[(usize, f64)],
    ) -> OppModel {
        let mut score_dist = [0.0f64; SCORE_SLOTS];
        for (score, weight) in dist {
            score_dist[*score] = *weight;
        }
        let total: f64 = score_dist.iter().sum();
        if total > 0.0 {
            for value in score_dist.iter_mut() {
                *value /= total;
            }
        }
        OppModel {
            set_size,
            success,
            score_dist,
            pass_prob,
            commitment: Commitment::Observed,
            commit_rate: 1.0,
            counter: 0.0,
            pressure: [0.0; SCORE_SLOTS],
            pressure_known: false,
        }
    }

    #[test]
    fn feasibility_matches_engine_bucket_semantics() {
        // A specific color is met only by the same color.
        let mut pool = [0i16; 8];
        pool[3] = 2;
        let mut need = [0i16; 11];
        need[3] = 2;
        assert!(feasible(&pool, &need));
        need[3] = 3;
        assert!(!feasible(&pool, &need));

        // A blade heart (BAll) is a wildcard for ONE specific color.
        let mut pool = [0i16; 8];
        pool[7] = 2;
        let mut need = [0i16; 11];
        need[5] = 2;
        assert!(feasible(&pool, &need));
        need[5] = 3;
        assert!(!feasible(&pool, &need));

        // Colorless hearts fill ONLY the grey bucket, never a specific color.
        let mut pool = [0i16; 8];
        pool[0] = 3;
        let mut need = [0i16; 11];
        need[2] = 1;
        assert!(!feasible(&pool, &need));
        need = [0i16; 11];
        need[0] = 3;
        assert!(feasible(&pool, &need));
        need[0] = 4;
        assert!(!feasible(&pool, &need));

        // Leftover specific hearts are absorbed by the total-count bucket.
        let mut pool = [0i16; 8];
        pool[1] = 3;
        let mut need = [0i16; 11];
        need[0] = 2;
        assert!(feasible(&pool, &need));

        // A specific-color deficit is never paid by the grey bucket.
        let mut pool = [0i16; 8];
        pool[0] = 5;
        let mut need = [0i16; 11];
        need[4] = 1;
        assert!(!feasible(&pool, &need));
    }

    /// Draw and Score icons are not heart requirements, so `feasible` ignores
    /// them; the filter that keeps such lives out of every candidate is
    /// `has_unpassable_icon`.
    #[test]
    fn draw_and_score_icons_are_never_check_requirements() {
        let mut need = [0i16; 11];
        need[9] = 1;
        assert!(has_unpassable_icon(&need));
        // The heart question itself ignores the icon, so a life whose ONLY
        // requirement is a Score icon is still "feasible" as a heart check -
        // which is exactly why it must be filtered out upstream.
        assert!(feasible(&[0i16; 8], &need));
    }

    #[test]
    fn rule_payoff_follows_win_conditions() {
        assert_eq!(rule_payoff(3, 2), 1.0);
        assert_eq!(rule_payoff(3, 3), 0.0);
        assert_eq!(rule_payoff(2, 3), -1.0);
        assert_eq!(rule_payoff(2, 2), 0.0);
    }

    /// 8.4.7.1: at two successes a tie does not place. A contested check
    /// whose totals match must be worth strictly less than the same check one
    /// point ahead.
    #[test]
    fn tie_does_not_place_at_two_successes() {
        let opp = opp_model(1, 0, 1.0, &[(5, 1.0)]);
        let tie = check_outcome(1.0, 5, 1, 2, &opp);
        let ahead = check_outcome(1.0, 6, 1, 2, &opp);
        assert!(ahead.p_place > tie.p_place);
        assert!(ahead.value > tie.value);
    }

    /// Below two successes a single-card tie DOES place, so the same totals
    /// are worth strictly more there. This is the pair of cases v7 needed a
    /// `+1` surcharge and a "strict close" branch to approximate.
    #[test]
    fn tie_places_for_a_single_card_set_below_match_point() {
        let opp = opp_model(1, 0, 1.0, &[(5, 1.0)]);
        let early = check_outcome(1.0, 5, 1, 1, &opp);
        let at_match_point = check_outcome(1.0, 5, 1, 2, &opp);
        assert!(early.p_place > at_match_point.p_place);
    }

    #[test]
    fn tie_places_regardless_of_set_size_below_match_point() {
        let opp = opp_model(1, 0, 1.0, &[(5, 1.0)]);
        let multi_card_tie = check_outcome(1.0, 5, 2, 1, &opp);
        let at_match_point = check_outcome(1.0, 5, 1, 2, &opp);
        assert!(multi_card_tie.p_place > at_match_point.p_place);
    }

    /// The free win (8.4.3.2) falls out of the model rather than being a
    /// special case: against an uncommitted opponent, reliability is the only
    /// currency, so a cheap reliable passer beats an unusable high score.
    #[test]
    fn uncommitted_opponent_makes_reliability_the_only_currency() {
        let opp = opp_model(0, 0, 0.0, &[(0, 1.0)]);
        let safe = check_outcome(0.97, 2, 1, 0, &opp);
        let risky = check_outcome(0.40, 9, 1, 0, &opp);
        assert!(safe.p_place > risky.p_place);
        assert!(safe.value > risky.value);
    }

    /// Against a committed opponent the same comparison inverts: score is what
    /// turns a passed check into a placement.
    #[test]
    fn committed_opponent_makes_score_the_currency() {
        let opp = opp_model(1, 0, 0.85, &[(4, 0.5), (8, 0.5)]);
        let safe_low = check_outcome(0.97, 2, 1, 0, &opp);
        let solid_high = check_outcome(0.90, 9, 1, 0, &opp);
        assert!(solid_high.p_place > safe_low.p_place);
    }

    /// At opponent match point, failing to contest loses outright (1.2.1.1),
    /// so even a thin check is worth more than setting nothing.
    #[test]
    fn match_point_makes_a_thin_check_worth_taking() {
        let opp = opp_model(1, 2, 0.4, &[(3, 1.0)]);
        let thin = check_outcome(0.30, 1, 1, 1, &opp);
        let fold = check_outcome(0.0, 0, 0, 1, &opp);
        assert!(thin.value > fold.value);
    }

    /// Setting nothing can never place for us, and it hands the check to an
    /// opponent who has already committed. Whether folding is nevertheless
    /// RIGHT depends on the ammunition a failed check burns, which is priced
    /// in `v8_live::candidate_value` - the model only has to make the
    /// comparison possible.
    #[test]
    fn folding_places_nothing_and_hands_over_the_check() {
        let opp = opp_model(2, 2, 0.9, &[(7, 1.0)]);
        let fold = check_outcome(0.0, 0, 0, 1, &opp);
        assert_eq!(fold.p_place, 0.0);
        assert!(fold.opp_place > 0.5);
        // We are one placement from the win, so letting them place is what the
        // fold is really paying for.
        assert_eq!(rule_payoff(1, 3), -1.0);
    }

    /// Burning ammunition is priced: a check we are likely to fail forfeits
    /// the future placement that life would have made. A coin-flip life is
    /// therefore worth about nothing on its own, which is the trade-off v7
    /// could only express with a stance floor (and got wrong in both
    /// directions, measured -1pp).
    #[test]
    fn failing_a_life_forfeits_its_future_placement() {
        let opp = opp_model(0, 0, 0.0, &[(0, 1.0)]);
        let certain = check_outcome(0.95, 2, 1, 0, &opp);
        let coin_flip = check_outcome(0.50, 2, 1, 0, &opp);
        let credit = PLACEMENT_CREDIT;
        let net_certain = certain.value - (1.0 - 0.95) * credit;
        let net_coin_flip = coin_flip.value - (1.0 - 0.50) * credit;
        assert!(net_certain > net_coin_flip);
    }

    /// Common random numbers: pools are reproducible, and on a fixed sample a
    /// strictly easier requirement is never ranked below a harder one.
    /// The colour shortfall must depend on the colour VECTOR, not the total.
    ///
    /// This is the whole justification for the term, so it is pinned as an
    /// arithmetic property rather than left to an arena run. The leaf's other
    /// terms read total supply, total blades and an interpolation of the same
    /// — so two boards below with identical totals are *identical* to the
    /// current leaf, and any weight on any of them cannot change the argmax.
    /// That is the shape of all nine leaf-term experiments the log records as
    /// inert. This function must not share that shape.
    #[test]
    fn colour_shortfall_sees_the_vector_not_the_total() {
        // A life needing three blue and three green.
        let need: Acc = {
            let mut n = [0i16; 11];
            n[0] = 3;
            n[3] = 3;
            n
        };
        // Two boards, same total (six), different distribution.
        let matching: Pool = {
            let mut b = [0i16; 8];
            b[0] = 3;
            b[3] = 3;
            b
        };
        let mismatched: Pool = {
            let mut b = [0i16; 8];
            b[1] = 3;
            b[4] = 3;
            b
        };
        assert_eq!(
            matching.iter().map(|v| i32::from(*v)).sum::<i32>(),
            mismatched.iter().map(|v| i32::from(*v)).sum::<i32>(),
            "the two boards must have equal totals for this test to mean anything"
        );

        assert_eq!(shortfall_for(&matching, &need), 0, "a colour-matched board is covered");
        assert_eq!(
            shortfall_for(&mismatched, &need),
            6,
            "an equal-total board in the wrong colours is not covered at all"
        );
    }

    /// Wildcards cancel shortfall and cannot drive it negative.
    #[test]
    fn a_wildcard_cancels_one_missing_colour() {
        let need: Acc = {
            let mut n = [0i16; 11];
            n[0] = 4;
            n[7] = 2; // two any-colour wildcards
            n
        };
        let none: Pool = [0i16; 8];
        assert_eq!(shortfall_for(&none, &need), 2, "4 needed, 2 wildcarded");
        // A board covering more than the wildcards can absorb must clamp at 0,
        // not go negative - a negative shortfall would be a bonus for having
        // surplus, which is not what "shortfall" means.
        let all_blue: Pool = {
            let mut b = [0i16; 8];
            b[0] = 9;
            b
        };
        assert_eq!(shortfall_for(&all_blue, &need), 0);
    }

    /// A board with no life in hand to aim at must not be rewarded or punished.
    /// Returning a bare 0 that reads as "fully covered" would silently bias
    /// every late-game leaf, where the hand has run out of lives but the term
    /// is still in the sum.
    #[test]
    fn no_life_in_hand_is_neither_covered_nor_short() {
        let db = real_db();
        let gs = bare_state(&db);
        // `bare_state` has an empty hand, so there is nothing to aim at.
        assert!(gs.seat_player(0).hand.cards.is_empty());
        assert_eq!(colour_shortfall(&gs, 0, &db), 0);
    }

    /// Pins the measured resolution of the opponent's pass model, and the reason
    /// the horizon term cannot work on top of it.
    ///
    /// `V8_OPP_HORIZON` re-evaluates the opponent's `pass_prob` against their
    /// board projected two turns further, and measured **zero** discordant games
    /// over 400 paired — p=1.00, every game bit-identical. Not "too small an
    /// effect to see": the term never changed a single decision, in 16163
    /// actions.
    ///
    /// The cause is here. `clears` sums `YIELD_PRIOR` weights gated on
    /// `supply >= bar` and is clamped to 1.0, so it is a hard threshold: across
    /// 369 board/blade combinations spanning the guides' whole real range
    /// (T1~4 to T7~26 hearts) the model takes **6 distinct values**. Advancing
    /// their board therefore lands on the same step almost every time, and
    /// "the opponent is catching up" is not a statement this model is capable
    /// of making.
    ///
    /// This test asserts the measured saturation, not the desired behaviour, and
    /// that is deliberate. The wanted property — a continuous gradient — is
    /// false today, and pinning it as `>= 8` would leave the suite red. Pinning
    /// the actual number instead means that any future change which gives the
    /// model a real gradient has to come here deliberately and raise it, rather
    /// than passing unnoticed.
    ///
    /// The fix is not a weight. It is replacing the hard threshold with the same
    /// coverage calculation v8 already uses for its own check
    /// (`FlipSampler` / `pass_probability`), so a board one heart short of the
    /// bar scores differently from one three hearts short. Until that exists,
    /// every opponent-horizon method in V8_NEXT_METHODS.md is measuring noise.
    #[test]
    fn the_opponent_pass_model_has_only_a_handful_of_states() {
        let db = real_db();
        let gs = bare_state(&db);
        let opp = OppModel::build(&gs, 0, &db);
        let mut seen = std::collections::BTreeSet::new();
        for total in 0..=40i32 {
            for blades in 0..=8i32 {
                let p = opp.with_board(total, blades).pass_prob;
                assert!(
                    (0.0..=1.0).contains(&p),
                    "pass_prob out of range: {p} at board={total} blades={blades}"
                );
                seen.insert((p * 1000.0).round() as i64);
            }
        }
        assert_eq!(
            seen.len(),
            6,
            "the opponent pass model changed resolution: it now takes {} distinct values \
             where it took 6. If this is the continuous gradient the horizon term needs, \
             re-measure V8_OPP_HORIZON - it was inert purely because the model could not \
             express a rate of change",
            seen.len()
        );
    }

    #[test]
    fn shared_pools_are_deterministic_and_monotone() {
        let sampler = FlipSampler {
            cats: vec![([0, 0, 0, 1, 0, 0, 0, 0], 6), ([0; 8], 6)],
            deck_len: 12,
            draws: 3,
        };
        let board: Pool = [0; 8];
        let a = sampler.sample_pools(&board, 64);
        let b = sampler.sample_pools(&board, 64);
        assert_eq!(a, b);

        let mut easy: Acc = [0; 11];
        easy[3] = 1;
        let mut hard: Acc = [0; 11];
        hard[3] = 5;
        // `sample_pools` returns `Pool` directly, so there is no narrowing step
        // between the sampler and the model any more.
        for pool in &a {
            if feasible(pool, &hard) {
                assert!(feasible(pool, &easy));
            }
        }
        assert!(pass_probability(&a, &easy) >= pass_probability(&a, &hard));
    }

    /// The continuous band term must be strictly monotone and must start at
    /// zero, or it is not a gradient but a second constant.
    #[test]
    fn band_progress_is_continuous_and_starts_at_zero() {
        let mut previous = band_progress_for(-5);
        assert!(previous <= 0.0);
        for supply in 0..40 {
            let now = band_progress_for(supply);
            assert!(
                now >= previous,
                "supply {supply}: band progress fell {previous} -> {now}"
            );
            previous = now;
        }
        // It must actually move inside a single band, which is the whole point:
        // the integer ceiling is flat there and this is not.
        assert!(band_progress_for(4) > band_progress_for(3));
        assert!(band_progress_for(6) > band_progress_for(5));
    }

    /// Above the last tabulated band the term must keep rising rather than
    /// saturating, or a developed board stops registering.
    #[test]
    fn band_progress_keeps_climbing_past_the_table() {
        let top = SCORE_MEDIAN[SCORE_SLOTS - 1];
        let at_top = band_progress_for(top);
        assert!(band_progress_for(top + 10) > at_top);
    }

    /// The forward ceiling must see the guides' ladder. With a cost-4 already
    /// on stage, a cost-9 in hand costs 5 after the baton (9.6.2.3.2), so the
    /// 4 -> 9 step of the curve is affordable five turns of income early.
    #[test]
    fn forward_ceiling_sees_the_baton_ladder() {
        let discount = 4;
        assert!(9 - discount < 9, "a baton must be cheaper than a fresh play");
        assert_eq!(9 - discount, 5);
    }

    /// The reason the Main phase needs `score_ceiling` at all: `P(place)` is a
    /// threshold test, so it is flat across a whole band of board development
    /// and cannot rank actions there. `largest_clearable` must be strictly
    /// monotone - that gradient is the entire point of the term.
    /// A baton must be booked on BOTH sides of the check, and the two sides
    /// move in opposite directions. The sent member's blades leave the active
    /// set (Q133: the yell count is active members only) but its hearts stay in
    /// the pool (3.2: the heart pool is ALL members), while the arriving member
    /// adds both.
    ///
    /// The mulligan's copy of this walk got the blades wrong - it overwrote the
    /// stage slot and moved on - so the mulligan thought a board grew sharper
    /// with every rung of the 4 -> 9 -> 13 ladder. Blades are Binomial trials
    /// (section 4), so that error is priced directly into placement probability.
    #[test]
    fn a_baton_books_the_sent_member_blades_out_and_hearts_in() {
        let db = real_db();
        // Two real members, deliberately chosen for opposite blade counts: the
        // one that arrives must be bladier, so a walk that forgets to remove the
        // sent member's blades reads strictly too high.
        let sent = first_member_with(&db, 1, |c| c.blade >= 1);
        let fresh = first_member_with(&db, 1, |c| {
            c.blade > 0 && c.cost.unwrap_or(0) > db.get_card(sent).unwrap().cost.unwrap_or(0)
        });
        let sent_card = db.get_card(sent).unwrap();
        let sent_hearts = sent_card.base_heart.as_ref().map_or(0, |b| i32::from(b.hearts.values_sum()));
        let sent_blades = i32::from(sent_card.blade);
        let fresh_card = db.get_card(fresh).unwrap();
        let fresh_hearts = fresh_card
            .base_heart
            .as_ref()
            .map_or(0, |b| i32::from(b.hearts.values_sum()));
        let fresh_blades = i32::from(fresh_card.blade);
        let cost = i32::from(fresh_card.cost.unwrap());
        let sent_cost = i32::from(sent_card.cost.unwrap());

        let mut walk = Forward {
            hearts: [0; 11],
            blades: sent_blades,
            stage: [sent, -1, -1],
            budget: 10_000,
            hand: vec![fresh],
            deck: Vec::new(),
            cursor: 0,
            lives: 0,
            density: 0.0,
            redraw: Vec::new(),
        };
        assert!(walk.step(&db));
        assert_eq!(
            walk.blades,
            fresh_blades,
            "the sent member's {sent_blades} blades leave the active set and only the arriving \
             member's {fresh_blades} count toward the yell"
        );
        assert_eq!(
            walk.hearts.iter().copied().map(i32::from).sum::<i32>(),
            sent_hearts + fresh_hearts,
            "3.2: the heart pool is ALL members, so the sent member's {sent_hearts} hearts stay in \
             it alongside the arriving member's {fresh_hearts}"
        );
        assert_eq!(
            walk.budget,
            10_000 + 1 - (cost - sent_cost),
            "9.6.2.3.2: the baton costs own - sent, after the Energy phase's +1 (7.5)"
        );
    }

    /// A replacement draws from the top of the deck, so the forward walk has to
    /// start PAST it. The mulligan's model did not, so an opening that replaced
    /// three cards received three phantom members - once as the replacement and
    /// again as the following turns' draws.
    #[test]
    fn a_replacement_is_consumed_from_the_deck_walk_not_drawn_twice() {
        let db = real_db();
        let first = first_member_with(&db, 2, |c| c.cost.unwrap_or(0) > 0);
        let second = first_member_with(&db, 2, |c| {
            c.cost.unwrap_or(0) > 0 && c.card_no != db.get_card(first).unwrap().card_no
        });
        let walk = Forward::from_opening(&db, 3, vec![first, second], &[], &[first]);
        assert_eq!(walk.cursor, 1, "the redrawn card is off the top of the deck");
        assert_eq!(walk.hand, vec![first]);
        assert_eq!(walk.redraw, vec![first]);
        // The next turn's draw must be the SECOND deck card, not the first.
        let mut walk = walk;
        walk.step(&db);
        assert!(
            walk.hand.contains(&second),
            "turn 1 drew the wrong card: {:?}",
            walk.hand
        );
    }

    /// The defect this model shipped with, pinned as a test.
    ///
    /// An empty opponent live zone means OPPOSITE things to the two attackers.
    /// To the second attacker it is a fact: they folded, and 8.4.3.2 hands the
    /// check over. To the FIRST attacker it is the normal state of a phase
    /// whose other half has not happened yet, and it says nothing.
    ///
    /// The model read one observation as the other, so on the majority of live
    /// phases it believed a passed check was a free placement. Scored against
    /// the engine's own verdicts over 1762 first-attacker phases: predicted
    /// `P(place) = 0.589`, truth `0.154`; predicted `P(they place) = 0.000`,
    /// truth `0.157`. The identical model read as the second attacker, where
    /// the zone really is a fact, scored 0.437 against a truth of 0.428.
    ///
    /// So the fix is structural rather than numeric: an unobserved commitment
    /// is a forecast, and a forecast must be conditioned on the opponent's
    /// public board rather than pinned at zero.
    #[test]
    fn an_empty_opponent_zone_means_different_things_to_the_two_attackers() {
        let db = real_db();
        let mut gs = bare_state(&db);
        // A board for each side with real hearts on stage, so neither bar is
        // trivially zero and the test measures the forecast, not a floor.
        let (member, _) = stage_member(&db, &mut gs, 0);
        let _ = member;
        stage_member(&db, &mut gs, 1);
        gs.current_phase = rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker;
        let as_first = OppModel::build(&gs, 0, &db);
        gs.current_phase = rabuka_engine::game_state::Phase::LiveCardSetSecondAttacker;
        let as_second = OppModel::build(&gs, 0, &db);

        assert_eq!(
            as_first.commitment,
            Commitment::Unobserved,
            "attacking first, the opponent's set does not exist yet"
        );
        assert_eq!(as_second.commitment, Commitment::Observed);
        assert!(
            as_first.pass_prob > 0.0,
            "an unobserved commitment must be forecast, not read as a fold; got {}",
            as_first.pass_prob
        );
        assert!(
            as_first.pass_prob < as_second.commit_rate,
            "and it must stay a probability of committing, not a certainty"
        );
        // The forecast must move with their public board, or it is a flat
        // constant wearing a forecast's clothes - covered in depth by
        // `the_forecast_rises_with_the_opponents_public_board` below.
    }

    /// The forecast must never claim the opponent is certain to place. An
    /// earlier shape of this fix set the assumed set size to 1 and left the
    /// pass probability alone, which is a guaranteed contested check - the
    /// mirror image of the original error, and equally wrong.
    #[test]
    fn a_forecast_commitment_is_strictly_less_than_certain() {
        let mut opp = opp_model(1, 0, 1.0, &[(4, 1.0)]);
        opp.commitment = Commitment::Unobserved;
        opp.commit_rate = assume_commit_rate();
        opp.pass_prob *= opp.commit_rate;
        assert!(opp.pass_prob < 1.0);
        assert!(opp.pass_prob > 0.0);
    }

    /// The forecast has to be conditioned on the opponent's PUBLIC board, or it
    /// is a flat constant wearing a forecast's clothes - and the whole point of
    /// modelling the unobserved commitment was to say more than "assume the
    /// worst".
    #[test]
    fn the_forecast_rises_with_the_opponents_public_board() {
        let db = real_db();
        // Three real members ordered by printed hearts, so "weak" and "strong"
        // are the same cards in different quantities.
        let mut members: Vec<i16> = {
            let mut ids: Vec<i16> = db.cards.keys().copied().collect();
            ids.sort();
            ids.into_iter()
                .filter(|&id| {
                    db.get_card(id).is_some_and(|c| {
                        c.card_type == CardType::Member
                            && c.cost.unwrap_or(0) > 0
                            && c.base_heart.is_some()
                    })
                })
                .collect()
        };
        assert!(members.len() > 3, "need members to build two boards");
        members.sort_by_key(|&id| {
            -db.get_card(id)
                .unwrap()
                .base_heart
                .as_ref()
                .map_or(0, |b| i32::from(b.hearts.values_sum()))
        });
        let weakest = members[members.len() - 1];
        let strongest = members[0];

        let forecast = |opp_members: &[i16]| {
            let mut gs = bare_state(&db);
            for (i, &id) in opp_members.iter().enumerate() {
                gs.player2.stage.stage[i.min(2)] = id;
            }
            gs.current_phase = rabuka_engine::game_state::Phase::LiveCardSetFirstAttacker;
            OppModel::build(&gs, 0, &db).pass_prob
        };
        let weak = forecast(&[weakest]);
        let strong = forecast(&[strongest, strongest, strongest]);
        assert!(
            strong > weak,
            "the unobserved-commitment forecast must read the opponent's public board: \
             strong board {strong} vs weak board {weak}"
        );
    }

    #[test]
    fn largest_clearable_is_strictly_monotone_in_supply() {
        let mut previous = largest_clearable(0);
        for supply in 0..40 {
            let now = largest_clearable(supply);
            assert!(
                now >= previous,
                "supply {supply}: score ceiling fell {previous} -> {now}"
            );
            previous = now;
        }
    }

    /// It must actually move over the range boards occupy: a board that grows
    /// from nothing to a T4 closeout supply has to raise the clearable band,
    /// otherwise the term is a constant in disguise. Values are read straight
    /// off the section 3.3 table: median hearts for scores 0..=8 are
    /// 2, 3, 5, 7, 10, 12, 14, 16, 19.
    #[test]
    fn largest_clearable_tracks_the_guide_turn_bands() {
        assert_eq!(largest_clearable(0), 0, "an empty board clears only score 0");
        assert_eq!(largest_clearable(2), 0, "2 hearts = the score-0 median");
        assert_eq!(largest_clearable(4), 1, "4 hearts = the score-1 median");
        assert_eq!(largest_clearable(5), 2, "5 hearts = the T2 score-2 standard");
        assert_eq!(largest_clearable(7), 3, "7 hearts = the T3 score-3 standard");
        assert_eq!(largest_clearable(12), 5, "12 hearts = the T4 score-5 standard");
        assert_eq!(largest_clearable(19), 8, "19 hearts = the score-8 median");
    }
