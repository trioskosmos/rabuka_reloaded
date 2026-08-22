# Engine Issues Found

_Audited 2026-08-22 against the current tree. The previous list (2026-06-16) had 9 entries;
7 were verified fixed and are kept below as closed records._

## Open

### 1. Exit Code 1 on Successful Build/Test — UNVERIFIED
- **Issue**: `cargo build/test/run` reportedly completed successfully but returned exit code 1,
  breaking CI.
- **Status**: Not re-verified since the original report; no recent evidence of recurrence.
  If CI fails mysteriously, check this first.

## Closed (verified fixed 2026-08-22)

| # | Issue | Where it was | Evidence of fix |
|---|-------|--------------|-----------------|
| 2 | Unused variable `group` | `src/ability/util.rs:166` | Line is now a `.count()` call; the pattern is gone |
| 3 | Invalid `drop()` of reference | `src/ability/effects/state.rs:98` | No `drop(player)` anywhere in engine |
| 4 | Unused imports in 8 test files (mirai_ticket, yoshiko_*) | various tests | Imports cleaned |
| 5 | Unused vars `filler`/`live`/`center` in tests | himeno_test, mirai_ticket_test | Defined-and-used now |
| 6 | Unused fn `advance_to_live_start` | performance_pipeline_test.rs:22 | Zero occurrences remain |
| 7 | Missing `Default` for `CardDatabase` | `src/core/card.rs` | `impl Default for CardDatabase` at card.rs:355 |
| 8 | Missing `Default` for `GameModifiers` | `src/core/game_modifiers.rs` | `impl Default` at game_modifiers.rs:120 |
| 9 | `or_insert_with(Vec::new)` instead of `.or_default()` | `src/core/card_loader.rs` | Now uses `.or_default()` (lines ~133, ~159) |

For refactor opportunities and verified-dead code, see [docs/REFACTOR_BACKLOG.md](../docs/REFACTOR_BACKLOG.md).
