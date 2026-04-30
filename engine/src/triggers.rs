// Trigger string constants for ability types
// Japanese is the canonical form used in card data (abilities.json)

/// 起動 - Activation ability (player-activated during main phase)
pub const ACTIVATION: &str = "起動";
/// 自動 - Automatic ability (triggers on game events)
pub const AUTO: &str = "自動";
/// 常時 - Continuous/constant ability (always active)
pub const CONSTANT: &str = "常時";
/// 登場 - Debut ability (triggers when card is placed on stage)
pub const DEBUT: &str = "登場";
/// ライブ開始時 - Live start ability (triggers at performance phase start)
pub const LIVE_START: &str = "ライブ開始時";
/// ライブ成功時 - Live success ability (triggers after live success)
pub const LIVE_SUCCESS: &str = "ライブ成功時";
/// パフォーマンスフェイズの始めに - Performance phase start
pub const PERFORMANCE_PHASE_START: &str = "パフォーマンスフェイズの始めに";
/// Main phase ability
pub const MAIN: &str = "メイン";
/// Baton touch ability
pub const BATON_TOUCH: &str = "baton touch";
/// ターンの始めに - Turn start
pub const TURN_START: &str = "ターンの始めに";
/// エネルギーフェイズの始めに - Energy phase start
pub const ENERGY_PHASE_START: &str = "エネルギーフェイズの始めに";
/// ドローフェイズの始めに - Draw phase start
pub const DRAW_PHASE_START: &str = "ドローフェイズの始めに";
/// メインフェイズの始めに - Main phase start
pub const MAIN_PHASE_START: &str = "メインフェイズの始めに";

// English aliases
pub const DEBUT_EN: &str = "Debut";
pub const LIVE_SUCCESS_EN: &str = "live_success";
pub const LIVE_START_EN: &str = "live_start";
pub const TURN_START_EN: &str = "turn_start";
