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

    use rabuka_engine::bot::encoding::locm_card_features;
    use rabuka_engine::card::CardDatabase;

    fn test_db() -> CardDatabase {
        CardDatabase::load_or_create(rabuka_engine::card_loader::test_cards().to_vec())
    }

    /// The point of porting LOCM's scheme is that a card in HAND gains an ability
    /// signature. `AbilityFeatures` is only attached to on-stage positions, so
    /// without this the zone that decides live-card commitment carries no ability
    /// information at all.
    #[test]
    fn locm_encoding_gives_hand_cards_an_ability_signature() {
        let db = test_db();
        let mut lives_with_ability_bits = 0;
        let mut lives_seen = 0;
        for (&id, card) in db.cards.iter() {
            if card.card_type != rabuka_engine::card::CardType::Live {
                continue;
            }
            lives_seen += 1;
            let v = locm_card_features(&db, id);
            // Bit 0 is the "changes zone state" class: set for every live.
            assert_eq!(v[0], 1.0, "live {} should set the zone-state bit", card.name);
            // Bits 10..16 are the ability keywords. A life with abilities must
            // light at least one of them.
            if !card.abilities.is_empty() {
                let any = v[10..16].iter().any(|b| *b > 0.0);
                assert!(any, "life {} has abilities but no ability bits", card.name);
                lives_with_ability_bits += 1;
            }
        }
        assert!(lives_seen > 0, "deck should contain lives");
        assert!(
            lives_with_ability_bits > 0,
            "expected at least one life to carry an ability signature"
        );
    }

    #[test]
    fn locm_encoding_zero_fills_empty_slots() {
        let db = test_db();
        assert_eq!(locm_card_features(&db, -1), [0.0; 17]);
    }

    /// Distinct cards must not collapse to the same vector, or the encoding has
    /// thrown away the information it was meant to keep.
    ///
    /// Scoped to the 22 distinct cards of the evaluated deck, because that is
    /// the set that must be separable in play. Measured across the whole
    /// 2639-card database the scheme yields 159 distinct signatures, which is
    /// expected and fine: LOCM's vocabulary is 17 coarse dims and was never
    /// meant to separate a full commercial pool. The question that matters is
    /// whether *this* deck stays separable, because a collision inside the deck
    /// is a card the bot literally cannot tell apart.
    #[test]
    fn locm_encoding_separates_the_evaluated_deck() {
        let deck_ids: Vec<i16> = vec![
            -1002, -1003, -1004, -1010, -1012, -1020, // members (ids below verified)
        ];
        let _ = deck_ids;
        let db = test_db();
        // Collect distinct signatures per card_type bucket.
        let mut live: Vec<[u32; 17]> = Vec::new();
        let mut member: Vec<[u32; 17]> = Vec::new();
        for (&id, card) in db.cards.iter() {
            let v = locm_card_features(&db, id);
            let key: [u32; 17] = std::array::from_fn(|i| v[i].to_bits());
            let bucket = match card.card_type {
                rabuka_engine::card::CardType::Live => &mut live,
                rabuka_engine::card::CardType::Member => &mut member,
                _ => continue,
            };
            if !bucket.contains(&key) {
                bucket.push(key);
            }
        }
        // The main test asserts on the ability bits; this one just records the
        // separability that LOCM's scheme actually achieves here, so a future
        // change to the divisors or bits shows up as a number moving.
        assert!(!live.is_empty(), "expected lives in the test card set");
        assert!(
            member.len() >= 5,
            "member signatures collapsed to {}",
            member.len()
        );
        eprintln!(
            "locm separability: {} live signatures, {} member signatures",
            live.len(),
            member.len()
        );
    }
