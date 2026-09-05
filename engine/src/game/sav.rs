//! SRAM deck image codec (`.sav` decks for the GBA port).
//!
//! The GBA bakes a fixed deck list into ROM. This module is the other half
//! of loading decks *after* the ROM is built: a tiny binary image that a PC
//! converter writes into the 32 KiB SRAM save file (which emulators persist
//! as `romname.sav` next to the ROM and flashcarts persist on hardware).
//!
//! Layout (all integers little-endian, byte-exact — the Python converter
//! mirrors this table, keep them in sync):
//!
//! ```text
//! offset  size  field
//! 0       4     magic "RBKS"
//! 4       1     format version (= 1)
//! 5       1     deck count (0..=MAX_SAV_DECKS)
//! 6       2     reserved (0)
//! 8       2     header checksum: wrapping sum of ALL deck-entry bytes
//! 10      ...   deck entries, ENTRY_LEN bytes each
//!
//! per deck entry (ENTRY_LEN = 1764):
//! 0       32    name, UTF-8 NUL-padded (max 31 content bytes)
//! 32      1     card count (1..=MAX_CARDS_PER_DECK)
//! 33      1     reserved (0)
//! 34      2     entry checksum: wrapping sum of this entry's card bytes
//! 36      1728  card slots: MAX_CARDS_PER_DECK x 24-byte UTF-8 NUL-padded
//!               card numbers, only the first `count` valid
//! ```
//!
//! Design notes (stolen fair and square):
//! - Raw SRAM, not `agb`'s slot manager: the manager *reformats* storage
//!   whose magic it doesn't recognise, which would wipe converter-written
//!   files on first boot. Our own magic + checksum validates instead.
//! - Card numbers as strings, not blob indices: self-describing across DB
//!   updates and across builds/ports. Main-deck cards only — no energy
//!   cards are stored or built (the engine adds default energy at setup).
//! - Each entry is self-checksummed and self-contained so it can later be
//!   sent verbatim as a link-cable deck packet for multiplayer. The one
//!   shared rule, enforced here and later on receipt: never trust the
//!   bytes — every card number is re-resolved against the local DB and a
//!   deck with any unresolvable card is dropped whole.
//! - Empty/erased SRAM (all 0x00 or all 0xFF) fails the magic check and
//!   means "no SRAM decks", never an error.
//!
//! `no_std`-safe (`alloc` only).

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(not(feature = "no_std"))]
use std::string::String;
#[cfg(not(feature = "no_std"))]
use std::vec::Vec;

/// Magic at image offset 0.
pub const SAV_MAGIC: &[u8; 4] = b"RBKS";
/// The only format version this codec reads and writes.
pub const SAV_VERSION: u8 = 1;
/// Max decks per image (image stays ~14 KiB of 32 KiB SRAM).
pub const MAX_SAV_DECKS: usize = 8;
/// Max cards per deck (largest baked deck is 72).
pub const MAX_CARDS_PER_DECK: usize = 72;
/// Fixed deck-name field length (NUL-padded UTF-8).
pub const SAV_NAME_LEN: usize = 32;
/// Fixed card-number field length (NUL-padded UTF-8; longest real number
/// is 19 chars / 21 bytes with fullwidth punctuation).
pub const SAV_CARD_LEN: usize = 24;
/// Fixed deck-entry length: 32 + 1 + 1 + 2 + 72 * 24.
pub const SAV_ENTRY_LEN: usize = SAV_NAME_LEN + 1 + 1 + 2 + MAX_CARDS_PER_DECK * SAV_CARD_LEN;
/// Header length: magic + version + count + reserved + checksum.
pub const SAV_HEADER_LEN: usize = 10;

/// One SRAM deck: display name + main-deck card numbers in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavDeck {
    pub name: String,
    pub cards: Vec<String>,
}

/// Codec failures. `Truncated` also covers short reads of erased SRAM
/// past the header; anything without the magic is not an error, it is
/// just "no SRAM decks" (see [`decode_sav`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavError {
    /// Fewer than [`SAV_HEADER_LEN`] bytes. Not an error condition for
    /// callers: treat as no decks.
    TooShort,
    BadMagic,
    BadVersion(u8),
    TooManyDecks(u8),
    Truncated,
    /// Name is empty, not valid UTF-8, or doesn't fit [`SAV_NAME_LEN`].
    BadName,
    /// Card count outside 1..=[`MAX_CARDS_PER_DECK`].
    BadCount(u8),
    /// Card number empty, not UTF-8, or longer than [`SAV_CARD_LEN`].
    BadCardNo,
    BadChecksum,
    BadEntryChecksum(usize),
}

/// Wrapping byte sum used for both checksums.
fn checksum(bytes: &[u8]) -> u16 {
    bytes.iter().fold(0u16, |acc, &b| acc.wrapping_add(b as u16))
}

/// Read a NUL-padded UTF-8 field. Empty (all NUL) is `None` only when
/// `allow_empty`; otherwise empty is an error.
fn read_padded_str(field: &[u8], allow_empty: bool) -> Option<String> {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    let text = core::str::from_utf8(&field[..end]).ok()?;
    if text.is_empty() && !allow_empty {
        return None;
    }
    Some(text.to_string())
}

/// Encode decks into a `.sav` image. Rejects oversize names / card
/// numbers / counts loudly instead of truncating silently.
pub fn encode_sav(decks: &[SavDeck]) -> Result<Vec<u8>, SavError> {
    if decks.len() > MAX_SAV_DECKS {
        return Err(SavError::TooManyDecks(decks.len() as u8));
    }
    let mut out: Vec<u8> = Vec::with_capacity(SAV_HEADER_LEN + decks.len() * SAV_ENTRY_LEN);
    out.extend_from_slice(SAV_MAGIC);
    out.push(SAV_VERSION);
    out.push(decks.len() as u8);
    out.extend_from_slice(&[0u8; 2]);
    // Checksum placeholder; patched after the entries are written.
    out.extend_from_slice(&[0u8; 2]);

    for deck in decks {
        let entry_start = out.len();
        let name_bytes = deck.name.as_bytes();
        if name_bytes.is_empty() || name_bytes.len() >= SAV_NAME_LEN {
            return Err(SavError::BadName);
        }
        if deck.cards.is_empty() || deck.cards.len() > MAX_CARDS_PER_DECK {
            return Err(SavError::BadCount(deck.cards.len().min(255) as u8));
        }
        let mut name_field = [0u8; SAV_NAME_LEN];
        name_field[..name_bytes.len()].copy_from_slice(name_bytes);
        out.extend_from_slice(&name_field);
        out.push(deck.cards.len() as u8);
        out.push(0u8);
        // Entry-checksum placeholder.
        out.extend_from_slice(&[0u8; 2]);
        let cards_start = out.len();
        for card_no in &deck.cards {
            let card_bytes = card_no.as_bytes();
            if card_bytes.is_empty() || card_bytes.len() > SAV_CARD_LEN {
                return Err(SavError::BadCardNo);
            }
            // NUL-pad each slot; interior NULs can't occur (UTF-8 never
            // encodes 0x00 inside a multi-byte sequence, and card numbers
            // have no reason to contain one — a hostile one fails the
            // UTF-8 read on decode and the deck is dropped).
            let mut slot = [0u8; SAV_CARD_LEN];
            slot[..card_bytes.len()].copy_from_slice(card_bytes);
            out.extend_from_slice(&slot);
        }
        // Pad unwritten slots (deck shorter than the max).
        let missing = MAX_CARDS_PER_DECK - deck.cards.len();
        out.extend(core::iter::repeat(0u8).take(missing * SAV_CARD_LEN));
        let entry_sum = checksum(&out[cards_start..]);
        out[entry_start + SAV_NAME_LEN + 2..entry_start + SAV_NAME_LEN + 4]
            .copy_from_slice(&entry_sum.to_le_bytes());
    }

    let header_sum = checksum(&out[SAV_HEADER_LEN..]);
    out[8..10].copy_from_slice(&header_sum.to_le_bytes());
    Ok(out)
}

/// Decode a `.sav` image. `TooShort` / `BadMagic` mean "no SRAM decks"
/// (fresh or erased SRAM) — callers treat those as empty, not failure.
pub fn decode_sav(bytes: &[u8]) -> Result<Vec<SavDeck>, SavError> {
    if bytes.len() < SAV_HEADER_LEN {
        return Err(SavError::TooShort);
    }
    if &bytes[0..4] != SAV_MAGIC {
        return Err(SavError::BadMagic);
    }
    if bytes[4] != SAV_VERSION {
        return Err(SavError::BadVersion(bytes[4]));
    }
    let count = bytes[5] as usize;
    if count > MAX_SAV_DECKS {
        return Err(SavError::TooManyDecks(bytes[5]));
    }
    let want_len = SAV_HEADER_LEN + count * SAV_ENTRY_LEN;
    if bytes.len() < want_len {
        return Err(SavError::Truncated);
    }
    if checksum(&bytes[SAV_HEADER_LEN..want_len]) != u16::from_le_bytes([bytes[8], bytes[9]]) {
        return Err(SavError::BadChecksum);
    }
    let mut decks: Vec<SavDeck> = Vec::with_capacity(count);
    for i in 0..count {
        let base = SAV_HEADER_LEN + i * SAV_ENTRY_LEN;
        let name = read_padded_str(&bytes[base..base + SAV_NAME_LEN], false)
            .ok_or(SavError::BadName)?;
        let card_count = bytes[base + SAV_NAME_LEN] as usize;
        if card_count == 0 || card_count > MAX_CARDS_PER_DECK {
            return Err(SavError::BadCount(bytes[base + SAV_NAME_LEN]));
        }
        let cards_base = base + SAV_NAME_LEN + 4;
        if checksum(&bytes[cards_base..cards_base + MAX_CARDS_PER_DECK * SAV_CARD_LEN])
            != u16::from_le_bytes([
                bytes[base + SAV_NAME_LEN + 2],
                bytes[base + SAV_NAME_LEN + 3],
            ])
        {
            return Err(SavError::BadEntryChecksum(i));
        }
        let mut cards: Vec<String> = Vec::with_capacity(card_count);
        for s in 0..card_count {
            let slot = &bytes[cards_base + s * SAV_CARD_LEN..cards_base + (s + 1) * SAV_CARD_LEN];
            cards.push(read_padded_str(slot, false).ok_or(SavError::BadCardNo)?);
        }
        decks.push(SavDeck { name, cards });
    }
    Ok(decks)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
