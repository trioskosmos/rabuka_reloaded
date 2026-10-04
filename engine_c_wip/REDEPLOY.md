# REDEPLOYMENT MAP

Ground truth for what is actually open, and who owns each file.
Companion to `RECONCILIATION.md`. Derived mechanically from the live tree at
**2026-10-03 20:41**, not from `PARITY_CHECKLIST.md`.

**Ownership rule used here** (as instructed):
- **FREE** — the file's owner produced a report in this repo, so the agent is presumed done.
  Proof of report = an `engine_c_wip/AUDIT_*.md` file, or an inline cluster section
  in `PARITY_CHECKLIST.md`, or a named `*.md` deliverable in `git status --porcelain`.
- **CONTESTED** — the file's owner **never reported**. These are the nine agents that
  died or never started. Assume an agent may still be mid-flight in these files.
- **CONTESTED-LIVE** — a file being written *right now*. Do not dispatch here at all.

---

## 0. Which agents actually reported — established from the tree, not from claims

| agent | owned files | reported? | proof |
|---|---|---|---|
| T1 | `vm.c resolver.c enums.c types.c` | **YES** | `AUDIT_vm_resolver_enums.md` |
| T2 | `state.c ability.c ability_effects.c score.c` | **YES** | `AUDIT_effects_state.md` |
| T3 | `zones.c game_state_core.c data.c stats_pipeline.c tracking.c alloc.c` | **YES** | `AUDIT_core_state.md` |
| T4 | `ability_queue.c triggers.c` | **YES** | `AUDIT_queue_triggers.md` |
| T5 | `describe.c compound.c choice_renderer.c deck_parser.c deck_builder.c` | **YES** | `AUDIT_describe_display.md` |
| T6 | `engine.c phase.c live.c` | **YES** | `AUDIT_actions_setup.md` |
| T7 | `vm.c resolver.c enums.c types.c` | **YES** | `AUDIT_decoders_resource.md` |
| G+H | `card.c util.c game_state_abilities.c` | **YES** (inline) | `PARITY_CHECKLIST.md:9-31` |
| I | `live.c phase.c engine.c web_server.c` | **YES** (inline) | cluster I section |
| J | `tests/**` | **YES** (inline) | cluster J section |
| K | `Makefile tools/** platforms/** generated/**` | **YES** (inline) | cluster K section |
| N (nozomi) | `tests/test_parity_search_reorder.c` | **YES** | `NOZOMI_ROOT_CAUSE.md` |
| **T8** | `test_game.c test_game.h tests/test_harness_self.c` | **YES, LIVE NOW** | `ORPHANS_AND_SCOPE.md` (20:33) |
| **harness** | test harness fixes | partial | `HARNESS_FIXES.md` (19:09) |
| **B** | `choice.c` | **NO** | file is **CLEAN in git**; no doc |
| **C** | `cost.c rules.c misc.c` | **NO** | all three **CLEAN in git**; no doc |
| **D** | `condition.c` | **NO** | **CLEAN in git**; no doc |
| **E** | `modifiers.c` | **NO** | **CLEAN in git**, mtime `10-03 01:49`; no doc |
| **F** | `custom.c executor.c draw.c move.c dynamic_count.c` | **NO** | 4 of 5 **CLEAN in git**; no doc |
| **unblock sweep** | `rules.c game_state_abilities.c condition.c util.c` | **NO** | no doc |

**The decisive fact: `src/ability/choice.c`, `src/ability/cost.c`,
`src/ability/condition.c`, `src/core/modifiers.c`, `src/ability/effects/misc.c`,
`src/ability/effects/move.c`, `src/ability/effects/draw.c`,
`src/ability/effects/executor.c` are all CLEAN in git.** Those are precisely the
files the never-reporting agents (B, C, D, E, F, sweep) were given. Clusters B, C, D
and E wrote **nothing at all**. Cluster F landed only the `custom.c` hunk.

---

## 1. The map — file → open items → ownership

| # | file | open items | owner status | cluster items |
|---|---|---|---|---|
| 1 | `src/core/modifiers.c` | **19** | **CONTESTED** (E never reported) | E1 E2 E3 E4 E5 E6 E9 E10 E11 E12 E14 E15 E16 E17, T3-6 (clear/re-apply half), T4-12, T6-5, D-refs |
| 2 | `src/ability/condition.c` | **10** | **CONTESTED** (D never reported) | D1 D2 D3 D4 D5 D6 D7 D8, T2-12, I3's `count_surplus_heart` gate |
| 3 | `include/rabuka.h` | **9** | FREE (main) | A4-wiring, A17 5 unwritten carriers, T1-7, T2-14 field, T3-6 3 arrays, T5-1 display types, B5 2 hoists, C5 4 decls, E15 decl |
| 4 | `src/core/card.c` | **5** | FREE (G+H reported) | A17/H1 producer (6 carriers), T2-11, T1-9, E16, G4 follow-up |
| 5 | `src/turn/phase.c` | **5** | FREE (T6/I reported) | T6-2, T6-3 2nd half, T4-10 2nd half, I2 2nd half |
| 6 | `src/turn/triggers.c` | **5** | FREE (T4 reported) | T4-10, T4-11, T4-13, T2-15 2nd half, T7-l |
| 7 | `src/ability/ability_queue.c` | **4** | FREE (T4 reported) | T4-7, T4-8, T4-9 2nd half, T4-12 |
| 8 | `src/core/game_state_abilities.c` | **4** | **CONTESTED** (G+H reported, but the sweep also claimed it and never reported) | E17, I6, T4-9, T2-10 2nd half |
| 9 | `src/ability/cost.c` | **3** | **CONTESTED** (C never reported) | C3, C5, C6 |
| 10 | `src/ability/effects/misc.c` | **3** | **CONTESTED** (C and F both claimed it; neither reported) | F3, T2-10, T2-15 1st half, F1 residue |
| 11 | `src/ability/vm.c` | **3** | FREE (T1/T7 reported) | T1-4, T1-5, T7-k |
| 12 | `src/ability/compound.c` | **3** | FREE (T5 reported) | T4-8 (unblocks it), T1-8, T5-2 |
| 13 | `src/ability/effects/move.c` | **2** | **CONTESTED** (F never reported) | F6, F7 (both already landed — needs verification only), F8 |
| 14 | `src/ability/effects/draw.c` | **2** | **CONTESTED** (F never reported) | F6 (seat bug), T3-9 seat half |
| 15 | `src/ability/choice.c` | **2** | **CONTESTED** (B never reported) | B2, B3 |
| 16 | `src/ability/describe.c` | **3** | FREE (T5 reported) | T5-3, T5-4, T5-6 |
| 17 | `src/core/game_state_core.c` | **1** | FREE (T3 reported) | T3-2 (rewire `rb_gsc_push_rule_log` to the new `g->rule_log`) |
| 18 | `src/ability/effects/executor.c` | **1** | **CONTESTED** (F never reported) | T7-l (`watches_ability_resolution` consumer) |
| 19 | `src/ability/enums.c` | **1** | FREE (T1 reported) | T1-6 |
| 20 | `src/ability/resolver.c` + `compound.c` | **1** | **CONTESTED** (neither reported; `compound.c` is T5's/FREE, `resolver.c` is T1's/FREE) | N1/N6 duplicate verdict-cache triad |
| 21 | `src/game/game_setup.c` | **1** | **CONTESTED** (C never reported) | C1 |
| 22 | `src/ability/effects/state.c` | **1** | FREE (T2 reported) | T3-4 1st half (correct path; the checklist's `src/core/state.c` does not exist) |
| 23 | `src/core/zones.c` | **1** | FREE (T3 reported) | T3-4 2nd half (`GameState*` in the signature) |
| 24 | `src/turn/live.c` | **3** | FREE (I/T6 reported) | I2 1st half, I4 (`live_spare` writer), T4-11 blocked-on |
| 25 | `src/game/deck_builder.c` | **1** | FREE (T5 reported) | T5-3 |
| 26 | `src/ability/effects/custom.c` | **1** | **CONTESTED** (F, dirty) | F1 residue (delete `h_custom` in misc.c) |
| 27 | `src/ability/effects/dynamic_count.c` | **1** | **CONTESTED** (F never reported) | verify F4 arms only |
| 28 | `Makefile` / `tools/**` / `cards/**` | **2** | FREE (K landed) | K7, K8 |
| 29 | `src/core/alloc_counter.c` | **1** | FREE (quarantine decision) | orphan, fails on every `-std` |
| 30 | `tests/**` | **4** | **CONTESTED-LIVE** (T8 active) | T5-7, T2-13, T3-1b residue, H-2 residue |
| 31 | `src/test_game.c`, `include/test_game.h`, `tests/test_harness_self.c` | **2** | **CONTESTED-LIVE — DO NOT DISPATCH** | `sizeof(GameState)` Makefile comment, mojibake |
| 32 | `src/ability/effects/look.c` | **1** | **CONTESTED-LIVE** — edited 20:52:20, +683 B, H-3's blocked-on half; compiles clean under `-fsyntax-only` | do not dispatch until the 20:52 edit's owner reports |

**Total genuinely open items: 92** (69 confirmed OPEN + 13 PARTIAL + 3 NOT-VERIFIED
+ 5 T6 numbered + the 2 K items), across **31 files**.

---

## 2. Ordered dispatch list

Ordered by (a) unblocking leverage, (b) cluster E's size, (c) safety.
Each line names **exactly one file**.

### Wave 0 — must happen before anything else is trustworthy (2 files)

| # | item | file | ownership |
|---|---|---|---|
| 0.1 | T3-2: rewire `rb_gsc_push_rule_log`/`_fmt` (`game_state_core.c:712-722`) to write `g->rule_log[]`/`rule_log_len[]` instead of `rb_log_push_verdict(text,"rule_log",1)`. Today the 500-entry buffer main added has **0 writers** and every rule-log line still lands in the 32-slot verdict ring, evicting condition verdicts. | `src/core/game_state_core.c` | FREE (1 item) |
| 0.2 | Fix `Makefile:3-8`: it claims `sizeof(GameState)` = 885912 and that `test_harness_self.c` §A fails if it moves. Real value **1113736**; §A asserts only a *ratio* (`<= 16777216/15 = 1118481`). **Headroom 4,745 B.** Either pin the absolute value in §A or delete the false claim in the Makefile. | `Makefile` | FREE (1 item) |

### Wave 1 — the two dead header fields main's bill created (1 file, 3 items)

| # | item | file | ownership |
|---|---|---|---|
| 1.1 | `resolution_owners[]`/`n_resolution_owners` (`rabuka.h:1617-1618`) have **0 writers and 0 readers**, and `rb_resolution_add_for_owner` (`rabuka.h:2639`) is declared with **no definition anywhere**. Write the body and the call site, or delete all three. | `include/rabuka.h` + `src/game/game_setup.c` | FREE (2 items) |
| 1.2 | `live_face_up[]` (`rabuka.h:1801`) has **0 writers and 0 readers**. It is the field T6-open4 needs for the `rb_check_invalid_live_cards` `face_up` gate. Either wire it or delete it. | `include/rabuka.h` | FREE (1 item) |

### Wave 2 — cluster E, the largest untouched surface (1 file, 19 items)

`src/core/modifiers.c` — **CONTESTED**, and the highest-value single file in the repo.
Nothing was attempted: E1–E6, E9–E12 are absent (four of them have **zero occurrences
tree-wide**), E7 and E13 are half-wired, E14/E15 are dead-but-present, E16 is blocked
on `card.c`, E17 needs `game_state_abilities.c`.
Expect **19 items in this one file**. Do not split it across two agents.

### Wave 3 — cluster D, the second-largest (1 file, 10 items)

`src/ability/condition.c` — **CONTESTED**. D1 was marked `[~] "agent running"` and
nothing landed. D2/D3 are half-present with a wrong negation shape. D4–D8 are absent.
Also carries T2-12's `activated_this_turn` reader and I3's missing
`live_surplus_ready_this_turn` gate. Expect **10 items**.

### Wave 4 — the CardFilter producer, which unblocks 4 items in 3 other clusters (1 file, 5 items)

`src/core/card.c` — FREE (G+H). `card_filter_from_effect` writes **none** of A17's six
carriers. This one edit makes util.c's `has_original_heart_total` reader live (H1),
closes T2-11 and J9-M, unblocks E16's `cause_card_id` writer (A9), and gives T1-9's
`placement_target()` a home. Expect **5 items**.

### Wave 5 — the never-reporting clusters' remaining files (5 files, 11 items)

| file | items | ownership |
|---|---|---|
| `src/ability/choice.c` | B2, B3 | CONTESTED (B) |
| `src/ability/cost.c` | C3, C5, C6 | CONTESTED (C) |
| `src/game/game_setup.c` | C1 | CONTESTED (C) |
| `src/ability/effects/misc.c` | F3, T2-10, T2-15, F1 residue | CONTESTED (C+F+sweep, three claimants) |
| `src/ability/effects/executor.c` | T7-l | CONTESTED (F) |

`misc.c` has the **worst ownership in the repo**: cluster C, cluster F and the unblock
sweep were each told it was theirs. Quiescence-check before dispatch.

### Wave 6 — queue/trigger dead code, all FREE (3 files, 12 items)

`src/ability/ability_queue.c` (4), `src/turn/triggers.c` (5),
`src/ability/compound.c` (3, and landing T4-8 unblocks T4-8 itself).

### Wave 7 — phase/live, all FREE (3 files, 9 items)

`src/turn/phase.c` (5), `src/turn/live.c` (3), `src/core/zones.c` (1).

### Wave 8 — duplicates and verification-only (3 files, 6 items)

| item | file | ownership |
|---|---|---|
| N1/N6 collapse `cached_condition_verdict` + `cond_cache_key` + `store_condition_verdict` (2 live static copies each, divergent cacheability test) | `src/ability/resolver.c` and `src/ability/compound.c` — **two files, decide which keeps the owner** | both FREE |
| N2 collapse `card_total_hearts` (2 divergent copies, 4 live call sites) | `src/ability/condition.c` (owner) — but wave 3 already has this file | see wave 3 |
| N3 collapse `card_appeared_this_turn` (one copy has the `cid < 0` guard, the other does not) | `src/ability/effects/misc.c` — but wave 5 already has it | see wave 5 |
| N4 collapse `player_prefix` (3 copies, signature split) | `src/ability/cost.c` / `misc.c` / `resolver.c` — 3 files | CONTESTED |
| N5/N7 collapse `effect_extra` (2) and `eff_extra` (5) | 5 files | mixed |
| N8/N9 delete the dead `move_missing.c` copies (`remove_card_from_any_zone`, `cmf_extra`, `extra_true`) — gcc reports one `defined but not used` | `src/ability/effects/move_missing.c` | CONTESTED |

### Wave 9 — data (2 items)

K7 `bash tools/regen_all.sh` and K8. Both FREE (K landed). K7 is the highest-leverage
single command in the repo: it is the root cause of T5-5 and it is why Rust and C
disagree on abilities 325 and 805.

### Wave 10 — tests, LAST and carefully (4 items)

`tests/**` is **CONTESTED-LIVE**: T8 is writing `tests/test_harness_self.c`,
`src/test_game.c` and `include/test_game.h` *right now*. Do not dispatch a test agent
until T8 reports. The four items (T5-7, T2-13, T3-1b residue, H-2 residue) are all
low-risk and can be bundled into one agent afterwards.

---

## 3. Free vs contested, condensed

**FREE (12 files, 62 items)** — `include/rabuka.h` 9 · `src/turn/phase.c` 5 ·
`src/turn/triggers.c` 5 · `src/core/card.c` 5 · `src/ability/ability_queue.c` 4 ·
`src/ability/vm.c` 3 · `src/ability/compound.c` 3 · `src/ability/describe.c` 3 ·
`src/turn/live.c` 3 · `src/core/game_state_core.c` 1 · `src/core/zones.c` 1 ·
`src/ability/enums.c` 1 · `src/ability/effects/state.c` 1 ·
`src/game/deck_builder.c` 1 · `Makefile`+`tools/**`+`cards/**` 2 ·
`src/core/alloc_counter.c` 1

**CONTESTED (12 files, 25 items)** — `src/core/modifiers.c` **19** ·
`src/ability/condition.c` **10** · `src/core/game_state_abilities.c` 4 ·
`src/ability/cost.c` 3 · `src/ability/effects/misc.c` 3 ·
`src/ability/choice.c` 2 · `src/ability/effects/move.c` 2 ·
`src/ability/effects/draw.c` 2 · `src/ability/effects/executor.c` 1 ·
`src/game/game_setup.c` 1 · `src/ability/effects/custom.c` 1 ·
`src/ability/effects/dynamic_count.c` 1

**CONTESTED-LIVE (4 files, 6 items — do not dispatch yet)** —
`src/test_game.c`, `include/test_game.h`, `tests/test_harness_self.c`,
`tests/**`

---

## 4. Items the checklist says are OPEN but are not — do not re-dispatch these

| id | why it looks open | actual |
|---|---|---|
| B1 | `[~] "(agent running)"` | done — `choice.c:4831, :5187` |
| B4 | `[ ]` "wire or delete with Rust proof" | already wired — `choice.c:333` def, `:600` call |
| C2 | `[ ]` | done — `engine.c:1187` |
| C4 | `[ ]` | done — `cost.c:1418-1419` |
| E8 | `[ ]` | done on arrival — `game_state_abilities.c:253, :302` |
| F1 | `[ ]` premise "misc.c h_custom is live" | **inverted** — `executor.c:408` now calls custom.c |
| F2 | `[ ]` | obsolete — 0 of 985 |
| F5 | `[ ]` | done — `live.c:82` + 3 callers |
| F7 | `[ ]` | done — `move.c:1529, :1543, :1577` |
| T5-8 | `[ ]` "in NO build target" | done — `Makefile:25, :28`; suite GREEN |
| H-1 | `[ ]` "FAIL with no colon" | done — it emits `FAIL:` now |
| H-2 | `[ ]` 3 rows red | obsolete — A2/A3/A4b all `ok`; new row A7 |
| H-3 | `[ ]` "aborts mid-file" | obsolete — completes, 3/146 failed |
| T3-1b | `[ ]` "line 780 should read centre" | obsolete — line 780 was rewritten |
| T6-1 | numbered "open" | done — `engine.c:1187` |

## 5. Items the checklist says are CLOSED but are not — these are the real gaps

| id | claimed | what is actually missing |
|---|---|---|
| **A4** | `[x]` | `rule_log[500][256]` has **0 writers, 0 readers**; the writer still targets the 32-slot ring |
| **A9** | `[x]` | `cause_card_id` has **0 writers, 0 readers**; E16 therefore cannot fire |
| **A17** | `[x]` | 6 carriers declared, **0 produced** |
| **G3** | `[x]` | the header declaration was *not* deleted — a `static inline` wrapper sits under a tombstone that says it was |
| **H1** | `[x]` | matcher landed, **producer absent** — every number it quotes is from dead code |
| **H2** | `[x]` | 4 local twins still present |
| **I2** | `[x]` | `rb_rules_clear_turn_state` declared twice, **defined nowhere, called nowhere** |
| **I6** | `[x]` | reader never swapped; its own comment is now false |
| **J9** | `[x]` | rows F and M still open |
| **T3-2** | `[ ]` | field landed, writer never rewired |
| **T3-5** | `[ ]` | fields landed, 0 writers, 0 readers, and the count field was never declared |

## 6. Constraints for whoever deploys

1. **`git stash@{0}` = `c458ab6f85ce5559041defd25165789b62cd8f21` holds live
   uncommitted work and was destroyed 3×.** No `stash pop` / `apply` /
   `checkout --` / `restore` / `reset --hard`. Read-only git only.
   Note the stash is dated **2026-10-02 12:15**, i.e. *yesterday* — it predates every
   cluster's work and applying it would clobber today's tree, not recover it.
2. Peers made unauthorised commits **`3cffff64`** and **`f152a50f`** (`HEAD`).
   Noted, not reverted.
3. `make t T=<stem>` and `make run-all-tests` both depend on `make data`, which is
   **correctly RED** while K7 is open. To run a suite, compile it directly against
   a built `OBJ_LIB`:
   `gcc -std=c11 -O2 -Wall -Iinclude -Isrc -Isrc/core/generated -Wl,--stack,16777216 -o t.exe $OBJ_LIB tests/test_<stem>.c`
4. Always `make clean` before any measurement; `make t` links pre-existing `.o`.
   `Copy-Item` preserves `LastWriteTime`, so **delete the `.o`** for any A/B.
5. Count failures with `^FAIL` (no colon), **case-sensitively** (`-cmatch`), on
   stdout **and** stderr. A suite emitting no output is BROKEN.
6. Never edit a source file through PowerShell string cmdlets
   (`Get-Content -Raw | Set-Content`, `-replace`). It has already corrupted
   `src/engine.c` once (99943 → 106372 B) and has just mojibake'd
   `tests/test_harness_self.c` and `ORPHANS_AND_SCOPE.md`.
7. `src/abilities_strings.bin` is correct at 726831 B / `89204df0bbde08a0…`.
   `cards/build/abilities_strings.bin` is 726649 and stale, and `Makefile:63`'s
   data rule points at it — so **run NO generator**. Verify the blob size and hash
   before and after every build.
