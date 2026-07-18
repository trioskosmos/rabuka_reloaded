// Trigger type constants used for ability matching and each_time watcher text.
// These correspond to the trigger text embedded in card abilities in abilities.json.

/// Ability activation trigger: "起動" — player-initiated, once per turn
#[cfg(feature = "no_std")]
use alloc::string::{String, ToString};
pub const ACTIVATION: &str = "起動";
/// Auto trigger: "自動" — fires automatically when its condition is met.
/// Sub-types are distinguished by condition.text keywords and
/// evaluated via the each_time watcher system or the main auto scan.
pub const AUTO: &str = "自動";
/// Constant trigger: "常時" — always-active passive modifier
pub const CONSTANT: &str = "常時";
/// Debut trigger: "登場" — fires when a member is placed on stage
pub const DEBUT: &str = "登場";
/// Live start trigger: "ライブ開始時" — fires at the start of a live performance
pub const LIVE_START: &str = "ライブ開始時";
/// Live success trigger: "ライブ成功時" — fires after a successful live
pub const LIVE_SUCCESS: &str = "ライブ成功時";
/// Main phase trigger: "メイン" — available during main phase
pub const MAIN: &str = "メイン";
/// Baton touch event marker (english, not a trigger type per se)
pub const BATON_TOUCH: &str = "baton touch";
pub const DEBUT_EN: &str = "Debut";
pub const LIVE_SUCCESS_EN: &str = "live_success";

// Jidou auto trigger types parsed from abilities.json (17 sub-types):
// The TAS scan catches all of them via standard condition evaluation.
//
//  1. on_yell         — "自分がエールしたとき" / "エールにより公開された"
//  2. on_area_move    — "このメンバーがエリアを移動したとき/するたび"
//  3. on_discard_from_stage — "このメンバーがステージから控え室に置かれたとき"
//  4. on_ally_appear_on_stage — "自分のステージに...登場したとき"
//  5. on_state_changed_to_wait — "ウェイト状態になったとき"
//  6. on_ally_appear_each_time — "自分のステージに...登場するたび"
//  7. on_live_start_resolved   — "ライブ開始時能力が解決するたび"
//  8. on_live_success_resolved — "ライブ成功時能力が解決するたび"
//  9. on_move_or_energy        — "エリアを移動するか...エネルギーが置かれた"
// 10. on_any_to_discard_each   — "いずれかの領域から控え室に置かれるたび"
// 11. on_live_zone_to_discard  — "ライブカード置き場から控え室に置かれた"
// 12. on_hand_to_discard_each  — "手札から...控え室に置かれるたび"
// 13. on_baton_touch_appear    — "バトンタッチして登場したとき"
// 14. on_discard_to_hand       — "控え室から手札に加えられた"
// 15. on_placed_in_live_zone   — "表向きでライブカード置き場に置かれた"
// 16. on_energy_placed_each    — "エネルギー置き場に...置かれるたび"
// 17. on_baton_touch_to_discard— "バトンタッチして控え室に置かれた"

/// Map a trigger string to its texticon filename for card badge display.
/// Used by gain_ability to show the gained ability's trigger type as a texticon on the card.
pub fn trigger_to_texticon(trigger: &str) -> String {
    match trigger {
        CONSTANT => "jyouji".to_string(),
        LIVE_SUCCESS => "live_success".to_string(),
        LIVE_START => "live_start".to_string(),
        DEBUT | DEBUT_EN => "toujyou".to_string(),
        ACTIVATION => "kidou".to_string(),
        AUTO => "jidou".to_string(),
        _ => "jyouji".to_string(), // fallback
    }
}

// All jidou/each_time abilities are handled by the unified TAS scan
// (trigger_auto_abilities_for_player_with_event). No separate scan needed.
// 控え室に置かれ → matches discard watchers
// エネルギー置き場 → matches energy placement watchers
// エール → matches yell watchers
