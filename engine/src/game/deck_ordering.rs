//! Shared GBA deck builder ordering logic.
//!
//! Single source of truth for Series/Rarity ordering and optimal card sorting.
//! Used by both GBA deck_builder.rs and web_ui card_browser.html export.

/// Official series order (matches GBA builder's SERIES_LIST).
/// Order by popularity/frequency to minimize Up/Down presses.
pub const SERIES_ORDER: &[&str] = &[
    "PL!",      // μ's (most common)
    "PL!SP",    // μ's Special
    "PL!S",     // Aqours
    "PL!N",     // 虹ヶ咲
    "PL!HS",    // Liella!
    "LL",       // 蓮ノ空
    "PR",       // Promo
    "Other",    // Everything else
];

/// Official rarity order (matches GBA builder's RARITY_LIST).
/// Order by frequency in competitive decks.
pub const RARITY_ORDER: &[&str] = &[
    "N", "N＋",      // Normal
    "R", "R＋",      // Rare
    "SR", "SR＋",    // Super Rare
    "SEC",           // Secret
    "P", "P＋",      // Parallel
    "PR", "PR＋",    // Promo Rare
    "L",             // Legend
    "PE＋",          // Premium+
    "SECL",          // Secret Legend
    "SRE",           // Super Rare Extra
    "SD", "SD2",     // Starter Deck
    "L+", "LLE",     // Special Legend variants
    "AR", "RM",      // Alternate/Masked
    "SECE", "SECL",  // Secret variants
];

/// Get sort priority for a series (lower = earlier = fewer Up presses).
pub fn series_priority(series: &str) -> usize {
    SERIES_ORDER.iter().position(|&s| s == series).unwrap_or(SERIES_ORDER.len())
}

/// Get sort priority for a rarity (lower = earlier = fewer Up presses).
pub fn rarity_priority(rarity: &str) -> usize {
    RARITY_ORDER.iter().position(|&r| r == rarity).unwrap_or(RARITY_ORDER.len())
}

/// Extract series prefix from card_no (e.g., "PL!-BP1-001-R" -> "PL!").
pub fn extract_series(card_no: &str) -> &str {
    card_no.split('-').next().unwrap_or("")
}

/// Extract rarity suffix from card_no (e.g., "PL!-BP1-001-R" -> "R").
pub fn extract_rarity(card_no: &str) -> &str {
    card_no.split('-').last().unwrap_or("")
}

/// Extract base card number without rarity (e.g., "PL!-BP1-001-R" -> "PL!-BP1-001").
pub fn extract_base(card_no: &str) -> &str {
    let parts: Vec<&str> = card_no.split('-').collect();
    if parts.len() >= 2 {
        &parts[..parts.len() - 1].join("-")
    } else {
        card_no
    }
}

/// Optimal sort key for GBA deck builder navigation.
/// Minimizes D-pad presses by grouping by (series_priority, rarity_priority, card_no).
pub fn gba_sort_key(card_no: &str) -> (usize, usize, String) {
    let series = extract_series(card_no);
    let rarity = extract_rarity(card_no);
    (series_priority(series), rarity_priority(rarity), card_no.to_string())
}

/// Sort a deck's card list for optimal GBA navigation.
pub fn sort_deck_for_gba(cards: &mut [(String, u8)]) {
    cards.sort_by(|a, b| gba_sort_key(&a.0).cmp(&gba_sort_key(&b.0)));
}

/// Build a flat card list (expanded by quantity) sorted for GBA.
pub fn flatten_and_sort_for_gba(cards: &[(String, u8)]) -> Vec<String> {
    let mut flat = Vec::new();
    for (card_no, qty) in cards {
        flat.extend(std::iter::repeat(card_no.as_str()).take(*qty as usize));
    }
    flat.sort_by(|a, b| gba_sort_key(a).cmp(&gba_sort_key(b)));
    flat.into_iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_series_priority() {
        assert_eq!(series_priority("PL!"), 0);
        assert_eq!(series_priority("LL"), 5);
        assert_eq!(series_priority("PR"), 6);
        assert_eq!(series_priority("Unknown"), SERIES_ORDER.len());
    }

    #[test]
    fn test_rarity_priority() {
        assert_eq!(rarity_priority("N"), 0);
        assert_eq!(rarity_priority("R"), 2);
        assert_eq!(rarity_priority("SEC"), 6);
        assert_eq!(rarity_priority("L"), 11);
        assert_eq!(rarity_priority("Unknown"), RARITY_ORDER.len());
    }

    #[test]
    fn test_gba_sort_order() {
        let mut cards = vec![
            ("PL!-BP1-001-R".to_string(), 1),
            ("LL-BP2-001-R+".to_string(), 1),
            ("PL!-BP1-001-N".to_string(), 1),
            ("PL!SP-BP1-005-R".to_string(), 1),
        ];
        sort_deck_for_gba(&mut cards);
        assert_eq!(cards[0].0, "PL!-BP1-001-N");  // PL! N first
        assert_eq!(cards[1].0, "PL!-BP1-001-R");  // PL! R second
        assert_eq!(cards[2].0, "PL!SP-BP1-005-R"); // PL!SP R
        assert_eq!(cards[3].0, "LL-BP2-001-R+");  // LL R+ last
    }
}