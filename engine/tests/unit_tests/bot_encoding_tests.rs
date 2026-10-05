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


use rabuka_engine::bot::encoding::*;


    #[test]
    fn state_dimension_matches_all_encoded_zones() {
        assert_eq!(EncodedState::state_dim(), 2248);
    }

    #[test]
    fn action_encoding_selects_one_action_type_row() {
        let mut rows = vec![0.0; ACTION_TYPE_COUNT * ACTION_TYPE_EMBED_DIM];
        rows[0] = 1.0;
        rows[ACTION_TYPE_EMBED_DIM] = 2.0;
        let action = ActionEncoding {
            action_type: 1,
            target_card_id: 0,
            target_zone: 0,
            position: 0,
            ability_index: 0,
            choice_option: u16::MAX,
            flags: 0,
            final_cost: u8::MAX,
            card_index: u8::MAX,
            card_indices_count: 0,
        };
        let encoded = action.encode(
            &[0.0; CARD_EMBED_DIM],
            &[0.0; ZONE_EMBED_DIM],
            &rows,
        );
        assert_eq!(encoded.len(), ACTION_ENC_DIM);
        assert_eq!(encoded[0], 2.0);
    }
