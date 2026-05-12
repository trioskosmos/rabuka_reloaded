# Test Fixes Needed

## Priority 1: Always-pass / No-assertion tests (MUST FIX)

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| 1 | `awake_test.rs` | 48-55 | `if score_mod == 1 { eprintln!(...) }` — no assertion | Add `assert_eq!(score_mod, 1)` |
| 2 | `mifune_test.rs` | 40 | `assert!(score < 10 \|\| true, ...)` — `\|\| true` always passes | Remove `\|\| true`, assert actual value |
| 3 | `dazzling_test.rs` | 58 | `assert!(true, "no crash")` — tests nothing | Assert both members received blade |
| 4 | `strawberry_trapper_test.rs` | 82 | `assert!(score_mod == 0 \|\| score_mod == 2)` — accepts any outcome | Test should be deterministic |
| 5 | `smile_test.rs` | 45-51 | Score fetched but never asserted | Add `assert_eq!(score_mod, 1)` |
| 6 | `miracle_wave_test.rs` | 50-56 | Score fetched but never asserted | Add `assert_eq!(mod_val, 4)` |
| 7 | `kanon_invalidate_test.rs` | 31-35 | Recovery checked but never asserted | Add `assert!(recovered)` |
| 8 | `yoshiko_debug_test.rs` | 50 | `// Don't assert anything` | Add assertions or remove test |

## Priority 2: Weak / wrong assertions

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| 9 | `solitude_test.rs` | 45 | `assert!(score_mod >= 0)` — always true | Change to `== 1` |
| 10 | `himeno_test.rs` | 98 | `assert!(true, "no crash")` | Remove or add real check |
| 11 | `b9_more_test.rs` | 64-79 | Recovery not asserted | Check card moved |
| 12 | `b8_live_timing_test.rs` | 34-36 | `\|\|` accepts two different phases | Check exact phase |
| 13 | `gameplay_test.rs` | 173 | Only checks "no more choices", not blade applied | Add blade check |
| 14 | `yoshiko_fixed_test.rs` | 51 | `hand.len() < 1` — wrong bound | Change to `== 0` |
| 15 | `start_true_dreams_test.rs` | 47-50 | "should verify" but no assertion | Add assertion |
| 16 | `love_u_test.rs` | 47-51 | "verify no crash" — no assertion | Add assertion |
| 17 | `cannot_baton_touch_test.rs` | 9 | `assert!(true, "compilation successful")` | Add runtime check |
| 18 | `sumire_auto_test.rs` | 49-51 | Only negative tests, no positive test | Add test for when condition IS met |
| 19 | `wien_n_test.rs` | 46-47 | Same — only negative tests | Add positive test |

## Priority 3: Cleanup (cosmetic / comments)

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| 20 | `energy_and_member_under_test.rs` | multiple | "FIXED:" in assertion messages | Remove "FIXED:" prefix |
| 21 | `energy_and_member_under_test.rs` | 127-130 | "GAP:" comments for known gaps | Document or fix |
| 22 | `strawberry_trapper_test.rs` | 80-83 | "GAP:" comment + OR assertion | Fix OR assertion |
| 23 | `kanon_test.rs` | 62-63 | `\|\|` accepts 0 or 6 | Fix to exact value |

## Priority 4: Weak bounds (should be exact values)

| # | File | Line | Issue | Fix |
|---|------|------|-------|-----|
| 24 | `shizuku_pb1_test.rs` | 40-41 | `active_energy_count <= 13` — too loose | Use exact value |
| 25 | `shizuku_pb1_test.rs` | 44-45 | `len < deck_before` — only checks change | Check exact delta |
| 26 | `chisato_move_test.rs` | 48-49 | `energy_after > energy_before` — too loose | Check exact delta |
| 27 | `draw_phase_fix.rs` | 28-29 | `<=` / `>=` bound checks | Check exact values |
| 28 | `setsuna_bp5_test.rs` | 77-78 | No assertion on heart modifier | Add assertion |
