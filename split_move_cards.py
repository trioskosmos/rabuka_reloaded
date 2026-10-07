import os
import subprocess

MOVE_DIR = os.path.join("engine", "src", "ability", "effects", "move")
MOD_RS = os.path.join(MOVE_DIR, "mod.rs")
SOURCE_RS = os.path.join(MOVE_DIR, "source.rs")
HANDLERS_RS = os.path.join(MOVE_DIR, "handlers.rs")

def main():
    # Read original mod.rs from git HEAD or disk
    try:
        content = subprocess.check_output(
            ["git", "show", "HEAD:engine/src/ability/effects/move/mod.rs"],
            text=True, encoding="utf-8"
        )
    except Exception:
        with open(MOD_RS, "r", encoding="utf-8") as f:
            content = f.read()

    lines = content.splitlines(keepends=True)

    # 1. source.rs
    # lines 255:1784 (0-indexed 254 to 1784)
    # Starts at "impl AbilityResolver {" (line 255)
    # Ends before "    /// Parse the 1-based deck position" (line 1785)
    source_body = lines[254:1784]

    # Make resolve_cards_from_source and resolve_cost_limit_reference pub(crate) so mod.rs can call them
    source_body_str = "".join(source_body)
    source_body_str = source_body_str.replace(
        "    fn resolve_cards_from_source(",
        "    pub(crate) fn resolve_cards_from_source(",
        1
    ).replace(
        "    fn resolve_cost_limit_reference(",
        "    pub(crate) fn resolve_cost_limit_reference(",
        1
    )

    source_header = """use super::super::super::enums::Zone;
use super::super::super::resolver::AbilityResolver;
use super::super::super::types::{Choice, ExecutionContext};
use super::super::super::util;
use crate::card::{AbilityEffect, Operator};
use crate::core::constants::CountCast;
use crate::game_state::GameState;
use super::transport::{drain_under_cards_to_energy_zone, matching_waitroom_indices, remove_card_from_any_zone};
use super::{MoveRoute, MoveSourceContext, SelectionSpec};
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

"""

    with open(SOURCE_RS, "w", encoding="utf-8", newline="\n") as f:
        f.write(source_header + source_body_str + "\n}\n")

    # 2. handlers.rs
    # Includes:
    # - numeric_deck_position & SelectionFilter (lines 122:253)
    # - handle_select_position (lines 2351:2513)
    # - execute_selected_cards_from_zone through execute_selected_energy_zone_cards (lines 2677:3563)
    handlers_helpers = "".join(lines[122:253])
    handle_select_position_code = "".join(lines[2351:2513])
    choice_handlers_code = "".join(lines[2677:3563])

    handlers_header = """use super::super::super::enums::Zone;
use super::super::super::resolver::AbilityResolver;
use super::super::super::types::{Choice, ExecutionContext, LookAndSelectStep};
use super::super::super::util;
use crate::card::{AbilityEffect, CardDatabase, Operator, PlacementOrder};
use crate::core::constants::CountCast;
use crate::game_state::GameState;
use crate::HashMap;
use super::placement::log_move_result;
use super::DeckOrderHint;
#[cfg(feature = "no_std")]
use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use smallvec::SmallVec;

"""

    handlers_content = (
        handlers_header
        + handlers_helpers
        + "\nimpl AbilityResolver {\n"
        + handle_select_position_code
        + "\n"
        + choice_handlers_code
        + "\n}\n"
    )

    with open(HANDLERS_RS, "w", encoding="utf-8", newline="\n") as f:
        f.write(handlers_content)

    # 3. mod.rs
    # Header: lines 0:122 (up to MoveSourceContext impl)
    mod_top = "".join(lines[0:122])

    # Insert submodules and make MoveRoute / SelectionSpec pub(crate)
    mod_top = mod_top.replace(
        "mod placement;\nmod selection;\nmod transport;",
        "mod handlers;\nmod placement;\nmod selection;\npub(crate) mod source;\nmod transport;"
    ).replace(
        "struct MoveRoute<'a> {",
        "pub(crate) struct MoveRoute<'a> {"
    ).replace(
        "struct SelectionSpec {",
        "pub(crate) struct SelectionSpec {"
    ).replace(
        "use crate::HashMap;\n",
        ""
    ).replace(
        "use smallvec::SmallVec;\n",
        ""
    ).replace(
        "use transport::{matching_waitroom_indices, remove_card_from_any_zone};\n",
        ""
    )

    # Middle: lines 1784:2351
    # - parse_effect_deck_pos
    # - resolve_or_card_type
    # - resolve_name_fragments
    # - maybe_prompt_deck_order
    # - place_taken_cards
    # - execute_move_cards
    mod_middle = "".join(lines[1784:2351])

    # Finalize & side effects: lines 2513:2677
    # Make finalize_card_movement and fire_debut_side_effects pub(crate) so handlers.rs can call them
    finalize_block = "".join(lines[2513:2677])
    finalize_block = finalize_block.replace(
        "    fn finalize_card_movement(",
        "    pub(crate) fn finalize_card_movement(",
        1
    ).replace(
        "    fn fire_debut_side_effects(",
        "    pub(crate) fn fire_debut_side_effects(",
        1
    )

    # Tail: lines 3563:end
    # - execute_move_cards_both
    # - execute_discard_until_count
    # - execute_logged_move
    mod_tail = "".join(lines[3563:])

    mod_content = (
        mod_top
        + "\nimpl AbilityResolver {\n"
        + mod_middle
        + "\n"
        + finalize_block
        + "\n"
        + mod_tail
    )

    with open(MOD_RS, "w", encoding="utf-8", newline="\n") as f:
        f.write(mod_content)

    print("Successfully split move/mod.rs into source.rs, handlers.rs, and mod.rs")

if __name__ == "__main__":
    main()
