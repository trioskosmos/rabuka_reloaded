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


use rabuka_engine::core::card_loader::*;


    #[test]
    fn ability_map_is_non_empty() {
        let map = CardLoader::build_abilities_map_shared_for_tests();
        assert!(
            !map.is_empty(),
            "ability map should contain at least one card"
        );
    }

    #[test]
    fn ability_refs_all_have_valid_indices() {
        let map = CardLoader::build_abilities_map_shared_for_tests();
        for (card_no, refs) in map.iter() {
            for r in refs {
                let idx = r.idx();
                assert!(
                    usize::from(idx) < rabuka_engine::ability::abilities_gen::NUM_ABILITIES,
                    "card {} has ability index {} >= {}",
                    card_no,
                    idx,
                    rabuka_engine::ability::abilities_gen::NUM_ABILITIES
                );
            }
        }
    }
