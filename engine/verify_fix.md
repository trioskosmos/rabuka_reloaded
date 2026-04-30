# Live Set Card Drawing Fix Verification

## Problem
Cards placed in the live set zone were not being drawn to replace them during the live set phase.

## Root Cause
The new phased live set implementation (`LiveCardSetP1Turn` and `LiveCardSetP2Turn`) was missing the card drawing logic that existed in the legacy `handle_finish_live_card_set` function.

## Fix Applied
Modified the `Pass` action handling in `turn.rs` to draw cards equal to the number of cards placed in the live zone when players pass during live set phases.

## Code Changes
In `src/turn.rs`, added card drawing logic to the `Pass` action handler for:
- `Phase::LiveCardSetP1Turn` - draws cards for P1 when passing
- `Phase::LiveCardSetP2Turn` - draws cards for P2 when passing

## Verification
The fix ensures that when a player places live cards and then passes during the live set phase, they draw replacement cards equal to the number of cards they placed in the live zone.

This maintains the game balance where players don't lose hand size when setting live cards.
