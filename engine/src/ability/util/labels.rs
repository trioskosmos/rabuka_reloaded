// Shared labels: singular/plural + card-type names (EN/JA).
//
// Extracted from `ability/util.rs` so display, choice-prompt and log code
// share one lookup instead of copy-pasting matches.

#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};

/// Returns "card" for count=1, "cards" otherwise.
pub fn card_plural(count: usize) -> &'static str {
    if count == 1 { "card" } else { "cards" }
}

/// Returns "member" for count=1, "members" otherwise.
pub fn member_plural(count: usize) -> &'static str {
    if count == 1 { "member" } else { "members" }
}

/// Returns "energy" for count=1, "energy" (same for both) - Japanese uses same word.
pub fn energy_plural(_count: usize) -> &'static str {
    "energy"
}

/// Convert internal card type to player-friendly string
pub fn card_type_label(ct: &str) -> String {
    match ct {
        "member_card" => "member".to_string(),
        "live_card" => "live card".to_string(),
        "energy_card" => "energy card".to_string(),
        _ => ct.to_string(),
    }
}

/// Convert internal card type to player-friendly Japanese string
pub fn card_type_label_ja(ct: &str) -> String {
    match ct {
        "member_card" => "メンバー".to_string(),
        "live_card" => "ライブカード".to_string(),
        "energy_card" => "エネルギー".to_string(),
        _ => ct.to_string(),
    }
}
