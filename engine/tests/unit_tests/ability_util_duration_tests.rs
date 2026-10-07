// Relocated from src/: these names used to arrive via `use super::*;`,
// which also pulled in the owning module's own `use` list. A glob on the
// module does not re-export those, so they are imported here explicitly.
use rabuka_engine::ability::util::duration::*;
use rabuka_engine::game_state::*;



    #[test]
    fn parses_canonical_duration_codes() {
        assert_eq!(parse_duration("this_turn"), Some(Duration::ThisTurn));
        assert_eq!(parse_duration("live_end"), Some(Duration::LiveEnd));
        assert_eq!(parse_duration("as_long_as"), Some(Duration::AsLongAs));
        assert_eq!(parse_duration("unless"), Some(Duration::Unless));
        assert_eq!(parse_duration("permanent"), Some(Duration::Permanent));
    }

    #[test]
    fn rejects_noncanonical_duration_codes() {
        assert_eq!(parse_duration("turn_end"), None);
        assert_eq!(parse_duration("this_live"), None);
        assert_eq!(parse_duration("until_used"), None);
        assert_eq!(parse_duration("next_turn"), None);
    }
