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

/// Parsed trigger kinds decoded from an ability's `triggers` text field.
/// Single source of truth for trigger matching — consumers ask
/// `Ability::has_trigger` instead of re-running ad-hoc substring checks,
/// which could false-positive on a kind name embedded in another token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TriggerKind {
    /// 起動 — player-initiated
    Activation,
    /// 自動 — fires automatically when its condition is met
    Auto,
    /// 常時 — always-active passive modifier
    Constant,
    /// 登場 — fires when a member is placed on stage
    Debut,
    /// ライブ開始時
    LiveStart,
    /// ライブ成功時
    LiveSuccess,
    /// メイン — available during main phase
    Main,
    /// Runtime metadata marker ("baton touch") attached to synthesized
    /// abilities by baton-touch bookkeeping; never card-printed.
    BatonTouch,
}

impl TriggerKind {
    /// Parse one comma-separated token of a `triggers` field.
    pub fn from_token(token: &str) -> Option<Self> {
        let t = token.trim();
        if t == ACTIVATION {
            Some(Self::Activation)
        } else if t == AUTO {
            Some(Self::Auto)
        } else if t == CONSTANT {
            Some(Self::Constant)
        } else if t == DEBUT || t == DEBUT_EN {
            Some(Self::Debut)
        } else if t == LIVE_START {
            Some(Self::LiveStart)
        } else if t == LIVE_SUCCESS || t == LIVE_SUCCESS_EN {
            Some(Self::LiveSuccess)
        } else if t == MAIN {
            Some(Self::Main)
        } else if t == BATON_TOUCH {
            Some(Self::BatonTouch)
        } else {
            None
        }
    }
}

/// Parse the full `triggers` field (e.g. "起動" or "ライブ開始時, 登場")
/// into its component kinds. Unknown tokens are ignored so newly printed
/// trigger text degrades to "no recognized trigger" rather than misfiring
/// as a different kind.
pub fn parse_triggers(triggers: &str) -> impl Iterator<Item = TriggerKind> + '_ {
    triggers.split(',').filter_map(TriggerKind::from_token)
}

/// Canonical English trigger key recorded in structured-log metadata and used to
/// match a `trigger_evaluation` entry against its eventual `ability_resolution`.
/// Kept in one place so trigger-scan, resolver, and negated-skip all agree.
pub fn canonical_trigger(raw: &str) -> String {
    let key = if raw.contains(DEBUT) || raw.contains(DEBUT_EN) {
        "debut"
    } else if raw.contains(LIVE_START) {
        "live_start"
    } else if raw.contains(LIVE_SUCCESS) || raw.contains(LIVE_SUCCESS_EN) {
        "live_success"
    } else if raw.contains(ACTIVATION) {
        "activation"
    } else if raw.contains(CONSTANT) {
        "constant"
    } else if raw.contains(AUTO) {
        "auto"
    } else {
        "unknown"
    };
    key.to_string()
}

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
//
// Trigger categories from the rulebook that are NOT implemented (audit 2026-08):
//   - Rule 7.4.2  ‘ターンの始めに’ / ‘アクティブフェイズの始めに’ / ‘ゲームの始めに’
//   - Rule 7.5.1/7.6.1/7.7.1  phase-begin triggers (エネルギー/ドロー/メイン)
//   - Rule 8.2.1/8.4.1  ライブカードセット/ライブ判定 フェイズの始めに
//   - Rule 8.4.10-12  ‘ターンの終わりに’ end-of-turn triggers + stability loop
// None of these appear in cards/abilities.json yet, so no card currently
// requires them; add trigger types here when such cards are introduced.

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
