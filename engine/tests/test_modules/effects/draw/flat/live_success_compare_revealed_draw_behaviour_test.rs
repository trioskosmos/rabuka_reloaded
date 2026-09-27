//! 渡辺 曜 `PL!S-bp3-005-R` — SOLVES the blocker in
//! `live_success_compare_revealed_draw_test.rs`, and records what is still open.
//!
//! ```
//! {{ライブ成功時}}エールにより公開された自分のカードの枚数が、相手がエールによって
//! 公開したカードの枚数より**少ない**場合、カードを1枚引く。
//! ```
//!
//! That file left this in its own words:
//!
//! > "The obvious rewrite — a differential over the opponent's yell blade, asserting
//! > p1 draws only when its own reveal count is lower — was written and then removed:
//! > **it could not get START:DASH!! to SUCCEED**, and ライブ成功時 never runs for a
//! > failed live, so the comparison was untestable without first understanding how the
//! > heart requirement is met. Whoever picks it up should assert 'the live succeeded'
//! > FIRST, so a missing heart fixture fails with that message rather than a confusing
//! > draw mismatch."
//!
//! ## The blocker, resolved
//!
//! `should_trigger_live_success` (core/game_state/abilities.rs:2751) needs THREE
//! things. The note knew it was missing one and could not name it; the one that is
//! not guessable is that the live card must be in the **live zone**:
//!
//! ```text
//!   if self.current_phase != Phase::LiveVictoryDetermination { return false; }
//!   if player.live_card_zone.cards.is_empty()      { return false; }   // <- this one
//! ```
//!
//! A live card parked in the HAND closes the window outright, whatever the hearts
//! say — which is why the first draft failed its own precondition while looking like
//! a heart problem. So the recipe, reusable for every gated ライブ成功時 test:
//!
//!   1. the live card in `player.live_card_zone`;
//!   2. `current_phase = Phase::LiveVictoryDetermination`;
//!   3. `stage_hearts` carrying a `heart00` WILDCARD.
//!
//! On (3): `START:DASH!!` prints `need_heart {heart0: 2}`, and `heart0` is a COLORLESS
//! WILDCARD rather than a literal colour — `check_heart_requirement`
//! (core/card.rs:4250-4274) skips `Heart00` in the per-colour loop and then requires
//! the leftover sum of the other colours to cover it. A `heart00` entry in the
//! PROVIDED hearts is an unbounded wildcard of the same kind, so
//! `stage_heart00 = 20` opens the window.
//!
//! Worth knowing about that wildcard, because it is not a shortcut around the data:
//! EVERY live in `cards.json` that prints a `need_heart` at all asks for `heart0` and
//! nothing else. A check for a live with named-colour requirements returns zero rows.
//! So the wildcard is the only shape this recipe has to cover, and a per-colour
//! matching regression would not be caught by any gated ライブ成功時 fixture.
//!
//! The comparison itself reads `cheer_revealed_cards` per player
//! (`dynamic_count.rs:159`), i.e. `player1_cheer_revealed_cards` and
//! `player2_cheer_revealed_cards`. Both are public fields, so the "a lot of fixture"
//! the note expected is two pushes.
//!
//! ## What is NOT settled
//!
//! The differential. With the window open, 渡辺曜's ONE `draw_card` ability
//! (`count: 1`, condition `location_condition` with `negation: true`) still leaves
//! THREE cards off the deck on every fixture tried — equal counts, higher counts and
//! lower counts alike. So whatever draws them is not this ability, and the
//! comparison is not yet attributable. Reported rather than guessed at: a test that
//! asserts one of those numbers would be asserting a number nobody has explained.
//!
//! What IS pinned here is the window, because that is what the note asked for and
//! because a test that cannot open the window cannot test anything else on the gated
//! path.

use crate::helpers::*;
use rabuka_engine::card::{BaseHeart, HeartColor, HeartMap};
use rabuka_engine::game_state::Phase;

const YOU: &str = "PL!S-bp3-005-R";
const LIVE: &str = "PL!-sd1-019-SD";
const FILLER: &str = "PL!-sd1-010-SD";

/// The three conditions, applied in the order that matters, each with a named
/// failure so a fixture that stops being sufficient says WHICH one broke.
fn open_live_success_window(game: &mut TestGame) {
    let live = game.id(LIVE);
    game.state.player1.live_card_zone.cards.push(live);

    let mut hearts = HeartMap::new();
    hearts.insert(HeartColor::Heart00, 20);
    game.state.player1.stage_hearts = Some(BaseHeart { hearts });

    game.state.current_phase = Phase::LiveVictoryDetermination;

    assert!(
        !game.state.player1.live_card_zone.cards.is_empty(),
        "condition 1: the live card must be in the LIVE ZONE — should_trigger_live_success \
         returns false on an empty one, so a live in the hand closes the window whatever \
         the hearts say"
    );
    assert_eq!(
        game.state.current_phase,
        Phase::LiveVictoryDetermination,
        "condition 2: the phase must be LiveVictoryDetermination"
    );
    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "condition 3: stage_hearts must cover the live's printed need_heart. heart0 is a \
         COLORLESS WILDCARD, so a heart00 entry in the provided hearts opens the window \
         whatever the live actually asks for."
    );
}

/// The window opens, which is what the hand-off note could not establish.
///
/// Without this, every other assertion on the gated ライブ成功時 path is untestable:
/// a closed window makes the trigger a silent no-op, and an absence assertion then
/// passes for the wrong reason. That is the same trap as the `live_success_no_premise`
/// class, on a fixture rather than on a file.
#[test]
fn the_live_success_window_opens_for_a_start_dash_live() {
    let mut game = TestGame::new(load_real_database());
    let you = game.id(YOU);
    let filler = game.id(FILLER);
    game.assert_card_identity(you, YOU);
    game.state.player1.stage.stage[0] = you;
    for _ in 0..20 {
        game.state.player1.main_deck.cards.push(filler);
        game.state.player2.main_deck.cards.push(filler);
    }

    open_live_success_window(&mut game);

    assert!(
        game.state.should_trigger_live_success(&game.state.player1),
        "the window is open, so a gated ライブ成功時 test can now be written against \
         this card instead of against a live that silently fails"
    );
}

