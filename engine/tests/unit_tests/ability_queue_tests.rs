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


use rabuka_engine::ability::queue::*;

    use rabuka_engine::game_state::AbilityTrigger;

    fn make_entry(card_no: &str, player_id: &str) -> AbilityQueueEntry {
        AbilityQueueEntry {
            card_no: card_no.to_string(),
            player_id: player_id.to_string(),
            ability: Arc::new(Ability::default()),
            ability_index: 0,
            card_id: None,
            trigger_type: AbilityTrigger::Auto,
            completed: false,
            cost_paid: false,
            cost_paid_index: 0,
            choice_card_no: None,
            conditional_choice: None,
            effect_started: false,
            use_limit_recorded: false,
            optional_cost_result: None,
            choice_player_id: None,
            trigger_moved_cards: None,
            trigger_appeared_cards: SmallVec::new(),
            pending_actions: Vec::new(),
            resolver: None,
            triggering_member_id: None,
            snapshot_movements: SmallVec::new(),
            choice_effect_text: None,
            choice_repeats_remaining: 0,
            optional_moves_all_moved: None,
            condition_cache: SmallVec::new(),
        }
    }

    /// A stand-in prompt. Tests here are about queue state, not about the
    /// shape of a choice, so they share one fixture rather than each spelling
    /// out the same eight fields.
    fn test_choice() -> Choice {
        Choice::SelectTarget {
            target: "test".into(),
            description: "test".into(),
            description_en: None,
            description_ja: None,
            allow_skip: false,
            options: None,
        }
    }

    #[test]
    fn queue_starts_idle() {
        let q = AbilityQueue::new();
        assert!(q.is_idle());
        assert!(q.is_empty());
        assert!(q.is_waiting_for_choice().is_none());
    }

    #[test]
    fn start_next_skips_completed() {
        let mut q = AbilityQueue::new();
        let mut e1 = make_entry("card_1", "p1");
        e1.completed = true;
        q.enqueue(e1);
        q.enqueue(make_entry("card_2", "p1"));

        assert!(q.start_next());
        assert!(matches!(q.state, QueueState::PayingCost { entry_index: 1 }));
    }

    #[test]
    fn start_next_refuses_when_busy() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        assert!(q.start_next());
        assert!(!q.start_next());
    }

    #[test]
    fn complete_returns_to_idle() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        q.start_next();
        q.complete_current();
        assert!(q.is_idle());
    }

    #[test]
    fn complete_marks_entry_completed() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        q.start_next();
        q.complete_current();
        assert!(q.get_entry(0).unwrap().completed);
    }

    #[test]
    fn pause_and_resume_choice() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        q.start_next();

        q.pause_for_choice(test_choice());
        assert!(q.is_waiting_for_choice().is_some());
        assert!(matches!(q.state, QueueState::WaitingForChoice { .. }));

        q.resume_with_choice();
        assert!(matches!(q.state, QueueState::ExecutingEffect { .. }));
    }

    #[test]
    fn complete_sets_choice_player_id() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        q.start_next();

        q.pause_for_choice(test_choice());

        let entry = q.get_entry(0).unwrap();
        assert_eq!(entry.choice_player_id.as_deref(), Some("p1"));
    }

    #[test]
    fn sequential_complete_advances_index() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        q.enqueue(make_entry("card_2", "p1"));
        q.start_next();
        q.complete_current();
        assert!(q.start_next());
        assert!(matches!(q.state, QueueState::PayingCost { entry_index: 1 }));
    }

    #[test]
    fn all_completed_returns_idle() {
        let mut q = AbilityQueue::new();
        let mut e1 = make_entry("card_1", "p1");
        e1.completed = true;
        q.enqueue(e1);
        q.enqueue(make_entry("card_2", "p1"));
        q.start_next();
        q.complete_current();
        assert!(!q.start_next());
        assert!(q.is_idle());
    }

    #[test]
    fn auto_ability_choice_returns_to_idle() {
        let mut q = AbilityQueue::new();
        q.pause_for_auto_ability_choice(test_choice());
        assert!(q.is_waiting_for_choice().is_some());
        q.resume_with_choice();
        assert!(q.is_idle());
    }

    #[test]
    fn has_entry_with_id_detects_duplicates() {
        let mut q = AbilityQueue::new();
        q.enqueue(make_entry("card_1", "p1"));
        // Entries no longer carry a prebuilt id, so the identity is rebuilt
        // from card_no + ability_index + trigger_type.
        let hit = AbilityId::from_parts("card_1", 0, &AbilityTrigger::Auto);
        let miss = AbilityId::from_parts("card_99", 0, &AbilityTrigger::Auto);
        assert!(q.has_entry_with_id(&hit));
        assert!(!q.has_entry_with_id(&miss));
    }
