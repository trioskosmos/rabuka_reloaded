// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::card_loader::*;




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
