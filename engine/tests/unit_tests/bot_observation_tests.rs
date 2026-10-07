// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::ability::types::*;
use rabuka_engine::game_state::*;


use rabuka_engine::bot::observation::*;

    use rabuka_engine::card::CardDatabase;

    use rabuka_engine::game_setup::{Action, ActionParameters, ActionType};
    use rabuka_engine::player::Player;

    use std::sync::Arc;

        fn test_state() -> (Arc<CardDatabase>, GameState) {
            let db = rabuka_engine::card_loader::test_card_db().clone();
            let p1 = Player::new("p1".into(), "P1".into(), true);
            let p2 = Player::new("p2".into(), "P2".into(), false);
            (Arc::clone(&db), GameState::new(p1, p2, db))
        }

    #[test]
    fn opponent_hand_and_hidden_choice_identity_are_not_exposed() {
        let (db, mut gs) = test_state();
        let hidden = db.get_card_id("PL!SP-bp1-005-P").unwrap();
        gs.player2.hand.cards.push(hidden);
        let obs = PublicObservation::from_state(&gs, 0);
        assert!(obs.opp.hand.is_empty());
        assert_eq!(obs.opp.hand_size, 1);
        let action = Action {
            description: "choice".into(),
            description_ja: None,
            action_type: ActionType::ChoiceSelect,
            parameters: Some(ActionParameters {
                card_id: Some(hidden),
                card_index: Some(0),
                card_indices: Some(vec![0]),
                stage_area: None,
                stage_area_index: None,
                use_baton_touch: None,
                card_name: None,
                card_no: None,
                ability_index: None,
                source_ability: None,
                base_cost: None,
                final_cost: None,
                available_areas: None,
                double_baton_pairs: None,
                disabled: None,
            }),
            selected: None,
        };
        let encoded = rabuka_engine::bot::encoding::ActionEncoding::from_action(&action, &obs);
        assert_eq!(encoded.target_card_id, -1);
        assert_ne!(encoded.flags & 8, 0);
    }

    #[test]
    fn own_deck_projection_is_orderless_and_opponent_choice_is_coarse() {
        let (db, mut gs) = test_state();
        let first = db.get_card_id("PL!SP-bp1-005-P").unwrap();
        let second = db.get_card_id("PL!SP-pb1-018-N").unwrap();
        gs.player1.main_deck.cards.extend([second, first]);
        let choice = Choice::SelectCard {
            zone: "Hand".into(),
            card_type: None,
            count: 1,
            description: "hidden".into(),
            description_en: None,
            description_ja: None,
            allow_skip: false,
            cost_limit: None,
            cost_limit_operator: None,
            cost_total: None,
            cost_total_operator: None,
            cost_values: None,
            group: None,
            characters: None,
            filtered_indices: None,
            is_select_action: false,
            heart_colors: Vec::new(),
            require_all_heart_colors: None,
            name_fragments: None,
            target_player_id: Some("p2".into()),
            blind: true,
            is_reveal: false,
            picker: Some("p2".into()),
            destination: None,
            discard_remaining: None,
        };
        gs.ability_queue.pause_for_choice(choice);
        let obs = PublicObservation::from_state(&gs, 0);
        assert_eq!(obs.me.main_deck_composition, vec![first.min(second), first.max(second)]);
        assert!(obs.pending_choice);
        assert!(!obs.pending_choice_for_viewer);
        assert!(obs.pending_choice_player.is_none());
        assert!(obs.pending_choice_kind.is_none());
    }
