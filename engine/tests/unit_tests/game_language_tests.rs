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


use rabuka_engine::game::language::*;


    #[test]
    fn toggle_round_trips() {
        assert_eq!(Lang::Japanese.toggle(), Lang::English);
        assert_eq!(Lang::English.toggle(), Lang::Japanese);
    }

    #[test]
    fn next_cycles_supported_registry() {
        // next() walks SUPPORTED in order and wraps around.
        let mut lang = Lang::DEFAULT;
        for _ in 0..Lang::SUPPORTED.len() {
            lang = lang.next();
        }
        assert_eq!(lang, Lang::DEFAULT);
        assert!(Lang::SUPPORTED.contains(&Lang::Japanese));
        assert!(Lang::SUPPORTED.contains(&Lang::English));
        assert_eq!(Lang::DEFAULT, Lang::Japanese);
    }

    #[test]
    fn labels_and_codes() {
        assert_eq!(Lang::Japanese.label(), "日本語");
        assert_eq!(Lang::English.label(), "English");
        assert_eq!(Lang::Japanese.code(), "jp");
        assert_eq!(Lang::English.code(), "en");
        assert_eq!(Lang::default(), Lang::Japanese);
    }

    #[test]
    fn console_chrome_is_bilingual() {
        assert_eq!(scroll_hint(Lang::Japanese), "A/B/Start:閉じる 上/下:スクロール");
        assert_eq!(scroll_hint(Lang::English), "A/B/Start close, Up/Down scroll");
        assert_eq!(more_line(3, Lang::Japanese), "  .. あと3件");
        assert_eq!(more_line(3, Lang::English), "  .. 3 more");
        assert_eq!(skip_row(Lang::Japanese), "[スキップ]");
        assert_eq!(skip_row(Lang::English), "[Skip]");
    }

    #[test]
    fn from_code_accepts_aliases() {
        assert_eq!(Lang::from_code("en"), Some(Lang::English));
        assert_eq!(Lang::from_code("ENGLISH"), Some(Lang::English));
        assert_eq!(Lang::from_code("jp"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("ja"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("Japanese"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("日本語"), Some(Lang::Japanese));
        assert_eq!(Lang::from_code("fr"), None);
    }
