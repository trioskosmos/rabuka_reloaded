# Parser Refactoring — Status

## Done this session
- P1: `_try_per_unit` 350→25 lines (3 helpers)
- P2: `parse_action` reduced by 130 lines (3 helpers)
- P1-7 earlier: deduplicate, unify, break up, remove dead code
- bp6-003 fix: Rust engine respect card_type from parsed effect
- bp7-007: 0 custom actions, 7 edge case gameplay tests
- Validation: merged into single _validate_semantic

## Remaining — the fundamental issues

### 1. `_fill_defaults` re-extracts fields `parse_action` already set
Lines 5952-6211 (256 lines). Re-extracts source, destination, cost_limit,
optional, max, position, group_names, heart_colors — all already extracted
by parse_action. Reader can't tell which function sets which field.

**Fix**: Move all field extraction into `parse_action`. `_fill_defaults` should
only set action-type-specific defaults (draw→deck/hand, shuffle→move_cards).

### 2. `_fill_defaults_move_cards` is 131 lines of source inference
Lines 5818-5949. Complex source→destination inference that should be
in the dispatch table or parse_action itself.

### 3. `_walk` + `_propagate_context` = two full tree walks
`_walk` (11 sub-walkers) runs during normalization.
`_propagate_context` (240 lines) runs AFTER _process_pre_fix.
They do different but overlapping work. Merging requires understanding
the timing dependency (propagate_context needs _process_pre_fix output).

### 4. `_process_pre_fix` is 340 lines of compensating patches
18+ FIX blocks that fix handler output. Each should be fixed in the
handler that produced the wrong output.

### 5. Double/triple extraction of same fields
`extract_source`, `extract_destination`, `extract_card_type`, etc. called
3-4 times on the same text across parse_action, _fill_defaults, _walk.
