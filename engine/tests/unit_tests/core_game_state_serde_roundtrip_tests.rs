// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::Arc;
use rabuka_engine::game_state::*;



    use rabuka_engine::card::CardDatabase;
    use rabuka_engine::player::Player;
    use rabuka_engine::zones::Stage;
    fn test_gs() -> GameState {
        let db = CardDatabase::new();
        let mut p1 = Player::new("p1".into(), "P1".into(), true);
        let mut p2 = Player::new("p2".into(), "P2".into(), false);
        p1.stage = Stage::new();
        p2.stage = Stage::new();
        let mut gs = GameState::new(p1, p2, Arc::new(db));
        gs.current_phase = Phase::Main;
        gs.turn_number = 3;
        gs
    }

    #[test]
    fn game_state_roundtrips_through_rmp() {
        let mut gs = test_gs();
        gs.player1.hand.cards.push(1);
        gs.player2.stage.stage[1] = 5;
        let bytes = rmp_serde::to_vec(&gs).unwrap();
        let mut back: GameState = rmp_serde::from_slice(&bytes).unwrap();
        // card_database is #[serde(skip)] 窶・caller reattaches its own
        back.card_database = gs.card_database.clone();
        assert_eq!(gs.player1.hand.cards, back.player1.hand.cards);
        assert_eq!(gs.player2.stage.stage, back.player2.stage.stage);
        assert_eq!(gs.turn_number, back.turn_number);
        assert_eq!(gs.current_phase, back.current_phase);
    }
