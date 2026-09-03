//! Explicit screen state machine for the GBA port.
//!
//! Every screen the player can be on, which buttons do what there, and which
//! screen each button leads to. The flow is driven by `bin/rabuka_gba.rs`
//! (`ModeSelect -> DeckSelect -> Match -> Result -> ModeSelect`); the engine
//! owns the in-match details (action list, pending-choice prompts) but each
//! of those renders through one of the screens below.
//!
//! ```text
//! Screen::ModeSelect --A/Start--> Screen::DeckSelectP1 --A/Start--> Match
//!     ^                               | (TwoPlayer only: DeckSelectP2)
//!     |                               v
//!     +--A/Start-- Screen::Result <-- match ends --+
//!
//! Inside Match (all overlay the board, Esc-able unless noted):
//!   Board --Select--> Actions --Select/B--> Board
//!   Board --Start--> StartMenu --B/Start--> Board
//!   Board --R--> CardDetail --A/B/L/R/Start--> Board
//!   (engine prompt) ChoiceGrid --A--> Match / --B--> Match (cancel/skip)
//!   ChoiceGrid --SL--> board overlay --any--> ChoiceGrid
//!   ChoiceGrid --L--> ability-source detail --any--> ChoiceGrid
//!   ChoiceGrid --R--> cursor-card detail --any--> ChoiceGrid
//! ```
//!
//! Button map per screen:
//!
//! | Screen       | Up/Down         | Left/Right         | A            | B            | Start        | Select | L            | R            |
//! |--------------|-----------------|--------------------|--------------|--------------|--------------|--------|--------------|--------------|
//! | ModeSelect   | move cursor     | -                  | confirm      | -            | confirm      | -      | detail text  | detail text  |
//! | DeckSelect   | move cursor     | -                  | confirm      | -            | confirm      | -      | detail text  | detail text  |
//! | Board        | prev/next action| move hand/stage cursor | run action | -        | Start menu   | Actions view | cycle focus Hand->Own->Opp | card detail |
//! | Actions      | prev/next action| -                  | run action   | back to Board | Start menu  | Board view | full text | card stats |
//! | StartMenu    | move cursor     | -                  | confirm/sub  | back/close   | back/close   | -      | -            | -            |
//! | CardDetail   | scroll text     | -                  | close        | close        | close        | -      | close        | close        |
//! | ChoiceGrid   | wrap incl. pages| wrap incl. pages   | pick         | back/skip    | (inert)      | board overlay | choice hint + src | cursor card |
//! | Result       | -               | -                  | continue     | -            | continue     | -      | -            | -            |

/// Every screen the GBA port can show. Variants are documentation-first:
/// the engine still renders Mode/Deck/Result lists, but `bin/rabuka_gba.rs`
/// visits them in exactly the order below, so "what menu goes where" is
/// answered here, not scattered across callbacks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    /// Match setup: VS AI / 2 Player / AI vs AI. A/Start confirms.
    ModeSelect,
    /// Match setup: player 1's deck. A/Start confirms.
    DeckSelectP1,
    /// Match setup (2-player only): player 2's deck. Skipped otherwise
    /// (the AI gets a random deck).
    DeckSelectP2,
    /// In-match default: graphical board, bottom action bar.
    Board,
    /// In-match full-screen action list (toggled with Select).
    Actions,
    /// In-match Start overlay: stats, then Game Log / Cards submenus.
    StartMenu,
    /// Card art + stats popup (R on the focused card, A in zone lists).
    CardDetail,
    /// Engine pending-choice prompt: 5x1 stage-size card grid with pagination.
    /// D-pad wraps (Up from the top row reaches the bottom row and vice
    /// versa); A picks, B backs out/skips, Start is inert, Select shows the
    /// board, L shows the ability-source card, R the cursor card.
    ChoiceGrid,
    /// Terminal match result. A/Start returns to [`Screen::ModeSelect`].
    Result,
}
