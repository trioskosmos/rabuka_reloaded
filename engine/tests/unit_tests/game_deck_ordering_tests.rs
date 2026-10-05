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


use rabuka_engine::game::deck_ordering::*;


    #[test]
    fn test_series_priority() {
        assert_eq!(series_priority("PL!"), 0);
        assert_eq!(series_priority("LL"), 5);
        assert_eq!(series_priority("PR"), 6);
        assert_eq!(series_priority("Unknown"), SERIES_ORDER.len());
    }

    #[test]
    fn test_rarity_priority() {
        assert_eq!(rarity_priority("N"), 0);
        assert_eq!(rarity_priority("R"), 2);
        assert_eq!(rarity_priority("SEC"), 6);
        assert_eq!(rarity_priority("L"), 11);
        assert_eq!(rarity_priority("Unknown"), RARITY_ORDER.len());
    }

    #[test]
    fn test_gba_sort_order() {
        let mut cards = vec![
            ("PL!-BP1-001-R".to_string(), 1),
            ("LL-BP2-001-R+".to_string(), 1),
            ("PL!-BP1-001-N".to_string(), 1),
            ("PL!SP-BP1-005-R".to_string(), 1),
        ];
        sort_deck_for_gba(&mut cards);
        assert_eq!(cards[0].0, "PL!-BP1-001-N");  // PL! N first
        assert_eq!(cards[1].0, "PL!-BP1-001-R");  // PL! R second
        assert_eq!(cards[2].0, "PL!SP-BP1-005-R"); // PL!SP R
        assert_eq!(cards[3].0, "LL-BP2-001-R+");  // LL R+ last
    }
