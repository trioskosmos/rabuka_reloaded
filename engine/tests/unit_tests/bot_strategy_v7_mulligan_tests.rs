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


use rabuka_engine::bot::strategy_v7::*;


    /// Version independence, as an executable claim: v7 computes its own
    /// mulligan, and that computation agrees with the v4 rule it replaced.
    /// If someone re-links v7 to v4 this stops testing the thing it claims to
    /// test; if v4 changes, this is the test that tells you v7 did not.
    #[test]
    fn v7_mulligan_matches_the_v4_rule_it_replaced() {
        let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        let db = rabuka_engine::Arc::new(CardDatabase::load_or_create(cards));

        let member_ids: Vec<i16> = db
            .cards
            .values()
            .filter(|c| c.card_type == CardType::Member)
            .take(12)
            .map(|c| db.get_card_id(c.card_no.as_ref()).unwrap())
            .collect();
        let live_ids: Vec<i16> = db
            .cards
            .values()
            .filter(|c| c.card_type == CardType::Live)
            .take(6)
            .map(|c| db.get_card_id(c.card_no.as_ref()).unwrap())
            .collect();
        assert!(member_ids.len() >= 4 && live_ids.len() >= 4, "need real cards");

        for variant in 0..6usize {
            let p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
            let p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
            let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
            gs.current_phase = Phase::MulliganFirstAttacker;
            let seat = gs.active_player_index();
            let lives = (variant % 4) + 1;
            for i in 0..lives {
                gs.seat_player_mut(seat).hand.cards.push(live_ids[i]);
            }
            for i in 0..5 {
                gs.seat_player_mut(seat).hand.cards.push(member_ids[(i + variant) % member_ids.len()]);
            }
            let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
            if actions.len() < 2 {
                continue;
            }
            let v7_pick = choose_mulligan_v7(&gs, &actions, &db);
            let v4_pick = rabuka_engine::bot::strategy_v4::choose_mulligan_v4(&gs, &actions, &db);
            assert_eq!(
                v7_pick.action_type, v4_pick.action_type,
                "variant {variant}: action type diverged"
            );
            assert_eq!(
                v7_pick.parameters.as_ref().and_then(|p| p.card_index),
                v4_pick.parameters.as_ref().and_then(|p| p.card_index),
                "variant {variant}: mulligan pick diverged from the v4 rule"
            );
        }
    }

    /// v7 must not hold 4+ lives: only three live slots exist, so a fourth is
    /// the one card type where being confident is always right.
    #[test]
    fn v7_mulligan_dumps_lives_beyond_the_three_slots() {
        let cards = rabuka_engine::card_loader::CardLoader::load_cards_from_file(std::path::Path::new(
            "../cards/cards.json",
        ))
        .unwrap();
        let db = rabuka_engine::Arc::new(CardDatabase::load_or_create(cards));
        let live_ids: Vec<i16> = db
            .cards
            .values()
            .filter(|c| c.card_type == CardType::Live)
            .take(5)
            .map(|c| db.get_card_id(c.card_no.as_ref()).unwrap())
            .collect();
        assert!(live_ids.len() >= 5, "need five distinct lives");
        let p1 = rabuka_engine::player::Player::new("p1".into(), "P1".into(), true);
        let p2 = rabuka_engine::player::Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
        gs.current_phase = Phase::MulliganFirstAttacker;
        for id in &live_ids {
            let seat = gs.active_player_index();
            gs.seat_player_mut(seat).hand.cards.push(*id);
        }
        let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
        if actions.len() < 2 {
            return;
        }
        let picked = choose_mulligan_v7(&gs, &actions, &db);
        let index = picked
            .parameters
            .as_ref()
            .and_then(|p| p.card_index)
            .expect("mulligan must select a card");
        // A life at hand index < 3 is one of the first three seen, so keeping
        // it is correct; anything else is the overflow.
        assert!(
            index >= 3,
            "v7 kept a fourth live (index {index}) instead of dumping the overflow"
        );
    }


    #[test]
    fn opening_curve_keep_finds_connected_ladder_only() {
        assert_eq!(
            opening_curve_keep(&[Some(2), Some(2), Some(7), Some(11), None, None]),
            vec![0, 1, 2, 3]
        );
        assert!(opening_curve_keep(&[Some(2), Some(2), Some(11), Some(17), None, None]).is_empty());
        assert!(opening_curve_keep(&[Some(2), Some(4), Some(7), Some(11), None, None]).is_empty());
    }

    #[test]
    fn reachable_curve_accepts_short_and_duplicate_cost_lines() {
        assert_eq!(
            reachable_curve_keep(&[Some(2), Some(2), Some(7)]),
            vec![0, 1, 2]
        );
        assert_eq!(
            reachable_curve_keep(&[Some(2), Some(2), Some(7), Some(13)]),
            vec![0, 1, 2, 3]
        );
        assert!(reachable_curve_keep(&[Some(7), Some(11)]).is_empty());
    }

    #[test]
    fn mulligan_default_matches_v4_and_curve_variant_recovers_selection() {
        let db = rabuka_engine::card_loader::test_card_db().clone();
        let names = [
            "PL!SP-bp1-005-R",
            "PL!SP-sd1-019-SD",
            "PL!SP-bp4-011-R＋",
            "PL!SP-bp5-006-R",
            "PL!SP-sd2-023-SD2",
            "PL!SP-bp4-025-L",
        ];
        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        for name in names {
            let id = db.get_card_id(name).unwrap();
            assert_eq!(db.get_card(id).unwrap().card_no, name);
            p1.hand.add_card(id);
        }
        let p2 = Player::new("p2".into(), "P2".into(), false);
        let mut gs = GameState::new(p1, p2, rabuka_engine::Arc::clone(&db));
        gs.current_phase = Phase::MulliganFirstAttacker;
        let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
        let v4 = rabuka_engine::bot::strategy_v4::choose_mulligan_v4(&gs, &actions, &db);
        assert_eq!(v4.parameters.as_ref().and_then(|p| p.card_index), Some(3));
        let default_choice = choose_mulligan_v7(&gs, &actions, &db);
        assert_eq!(default_choice.action_type, v4.action_type);
        assert_eq!(
            default_choice
                .parameters
                .as_ref()
                .and_then(|p| p.card_index),
            v4.parameters.as_ref().and_then(|p| p.card_index)
        );
        gs.mulligan_selected_indices.extend([3, 2, 0]);
        for _ in 0..3 {
            let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
            let action = choose_mulligan_curve(&gs, &actions, &db);
            assert_eq!(action.action_type, ActionType::SelectMulligan);
            assert_eq!(action.selected, Some(true));
            rabuka_engine::game_setup::execute_action(&mut gs, &action).unwrap();
        }
        assert!(gs.mulligan_selected_indices.is_empty());
        let actions = rabuka_engine::game_setup::generate_possible_actions(&gs);
        assert_eq!(
            choose_mulligan_curve(&gs, &actions, &db).action_type,
            ActionType::ConfirmMulligan
        );
    }
