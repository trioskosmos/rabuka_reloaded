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


use rabuka_engine::ability::describe::*;


    #[test]
    fn choice_prompt_templates_all_have_japanese() {
        for tpl in CHOICE_PROMPT_TEMPLATES_EN {
            assert!(
                translate_choice_prompt_en_to_ja(tpl).is_some(),
                "choice prompt template missing Japanese translation: {:?}",
                tpl
            );
        }
    }

    #[test]
    fn choice_prompt_translator_handles_parameterized_prompts() {
        assert_eq!(
            translate_choice_prompt_en_to_ja("Select up to 3 card(s) to keep"),
            Some("最大3枚まで手札に残すカードを選択".to_string())
        );
        assert_eq!(
            translate_choice_prompt_en_to_ja("Select up to 1 member(s) to change state"),
            Some("状態を変更するメンバーを最大1体選択".to_string())
        );
        assert_eq!(
            translate_choice_prompt_en_to_ja("Select up to 3 card(s) from the 5 looked-at cards (or skip)"),
            Some("見たカードから最大3枚を選択（スキップ可）".to_string())
        );
        assert_eq!(
            translate_choice_prompt_en_to_ja("Repeat effect?"),
            Some("効果を繰り返しますか？".to_string())
        );
        assert_eq!(translate_choice_prompt_en_to_ja("Some unrelated text"), None);
    }
