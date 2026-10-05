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



    fn sample_decks() -> Vec<SavDeck> {
        vec![
            SavDeck {
                name: String::from("aqours*"),
                cards: vec![
                    String::from("PL!S-bp2-022-L"),
                    String::from("LL-bp2-001-R＋"),
                    String::from("日本語テスト"),
                ],
            },
            SavDeck {
                name: String::from("second"),
                cards: vec![String::from("X-1")],
            },
        ]
    }

    #[test]
    fn roundtrip_preserves_names_and_cards() {
        let decks = sample_decks();
        let bytes = encode_sav(&decks).expect("encode");
        assert_eq!(
            bytes.len(),
            SAV_HEADER_LEN + decks.len() * SAV_ENTRY_LEN
        );
        assert_eq!(&bytes[0..4], b"RBKS");
        assert_eq!(bytes[4], 1);
        assert_eq!(bytes[5], 2);
        assert_eq!(decode_sav(&bytes).expect("decode"), decks);
    }

    #[test]
    fn empty_sram_means_no_decks_not_error() {
        assert_eq!(decode_sav(&[]), Err(SavError::TooShort));
        assert_eq!(decode_sav(&[0u8; 32]), Err(SavError::BadMagic));
        assert_eq!(decode_sav(&[0xFFu8; 64]), Err(SavError::BadMagic));
    }

    #[test]
    fn rejects_bad_version_and_counts() {
        let mut bytes = encode_sav(&sample_decks()).expect("encode");
        bytes[4] = 2;
        assert_eq!(decode_sav(&bytes), Err(SavError::BadVersion(2)));
        let too_many: Vec<SavDeck> = (0..MAX_SAV_DECKS + 1)
            .map(|i| SavDeck {
                name: format!("d{}", i),
                cards: vec![String::from("X")],
            })
            .collect();
        assert!(matches!(
            encode_sav(&too_many),
            Err(SavError::TooManyDecks(_))
        ));
    }

    #[test]
    fn rejects_truncation_and_corruption() {
        let bytes = encode_sav(&sample_decks()).expect("encode");
        assert_eq!(
            decode_sav(&bytes[..bytes.len() - 1]),
            Err(SavError::Truncated)
        );
        let mut corrupt = bytes.clone();
        let last = corrupt.len() - 1;
        corrupt[last] ^= 0xFF;
        assert!(decode_sav(&corrupt).is_err());
        let mut corrupt_name = bytes.clone();
        corrupt_name[SAV_HEADER_LEN] = 0x00;
        assert!(decode_sav(&corrupt_name).is_err());
    }

    #[test]
    fn rejects_oversize_fields_loudly() {
        let long_name: String = (0..SAV_NAME_LEN).map(|_| "x").collect();
        assert_eq!(
            encode_sav(&[SavDeck {
                name: long_name,
                cards: vec![String::from("X")],
            }]),
            Err(SavError::BadName)
        );
        let long_card: String = (0..SAV_CARD_LEN + 1).map(|_| "y").collect();
        assert_eq!(
            encode_sav(&[SavDeck {
                name: String::from("ok"),
                cards: vec![long_card],
            }]),
            Err(SavError::BadCardNo)
        );
        assert_eq!(
            encode_sav(&[SavDeck {
                name: String::from("ok"),
                cards: Vec::new(),
            }]),
            Err(SavError::BadCount(0))
        );
    }
