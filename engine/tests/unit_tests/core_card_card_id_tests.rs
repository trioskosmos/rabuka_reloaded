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
use rabuka_engine::core::card::*;


    #[test]
    fn conversions_round_trip() {
        let id = CardId::from(42_i16);
        assert_eq!(id.raw(), 42);
        assert_eq!(CardId::from_raw(-1), CardId::EMPTY);
        assert_eq!(usize::from(CardId::from_raw(7)), 7usize);
    }

    #[test]
    fn empty_and_negative_detection() {
        assert!(CardId::EMPTY.is_empty());
        assert!(CardId::from_raw(-5).is_empty());
        assert!(!CardId::from_raw(0).is_empty());
        assert!(!CardId::from_raw(1).is_empty());
    }

    #[test]
    fn usable_as_map_key() {
        let mut m = HashMap::new();
        m.insert(CardId::from_raw(3), "x");
        assert_eq!(m.get(&CardId::from_raw(3)), Some(&"x"));
        assert_eq!(m.get(&CardId::from_raw(4)), None);
    }
