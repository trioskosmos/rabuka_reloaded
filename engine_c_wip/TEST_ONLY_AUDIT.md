# TEST-ONLY FUNCTION AUDIT — the class the global census named and nobody examined

## 0. Which HEAD this is measured against — READ THIS FIRST

**HEAD at the start of this audit: `cf315a9ac5dedd1ca02caeadbfb13f0f74919e5c`** ("Merge windows worktree: rust engine, shared assets, cards JSON, C port v2"), with **152 modified files in the working tree** (peers editing live). Primary snapshot `…/Temp/kilo/to/to26/`.

**HEAD moved under me mid-audit to `5ce311e6` ("wip: engine/c parity audits, bot v8 live work, test inventory regen, new parity tests and audit docs").** That commit swept up the 152 dirty files. I re-snapshotted to `…/Temp/kilo/to/to26b/`, rebuilt all 54 `src/**/*.c` and all **166** `tests/*.c` (was 163), and re-ran the whole census. **Every number and every table below is the `to26b` / `5ce311e6` measurement.** The `to26` numbers are given only where they differ.

Five files changed between the two measurements: `src/web_server.c`, `src/ability/effects/rules.c`, `src/core/game_state_abilities.c`, `src/core/modifiers.c`, `src/turn/triggers.c`. `src/core/game_state_abilities.c` was still being edited *while I re-measured* (a function I read at `:1526` had moved to `:1507` between two commands ten minutes apart), so treat any `game_state_abilities.c` line number in §8 as ±20 lines and re-derive it.

**Headline drift between the two HEADs — this is the fifth time this project has been bitten by exactly this, so it is worth stating precisely:**

| | `cf315a9`+dirty | `5ce311e6` |
|---|---|---|
| definitions in `src/**/*.c` | 2 569 | **2 576** |
| WIRED | 1 845 | **1 856** |
| **TEST-ONLY** | **261 (+2 header statics = 263)** | **261 (+2 header statics = 263)** |
| DEAD | 463 | **459** |

The *count* is stable, the *membership* is not: `rb_queue_push_constant_context` and `rb_queue_pop_constant_context` became WIRED (a peer wired them from `src/core/modifiers.o`), and two brand-new test-only exports arrived — `rb_inject_choice_ability_context` (`game_state_abilities.c:1526`, reached only by `tests/test_parity_choice_routing.c`) and `rb_rules_execute_restriction` (`rules.c:1865`, reached only by `tests/test_parity_q68_cannot_live.c`). **The class is being actively grown by the peers landing test-only exports.** The second of those is ORPHANED LOGIC O3 below and is the most consequential thing in this document. Twelve line numbers moved inside `modifiers.c` (+6), `game_state_abilities.c` and `triggers.c` (+17).

**All four classification counts, the 9 dead census entries, and all three ORPHANED LOGIC cases are identical at both HEADs.** Only the churn listed above and the line numbers moved.

---

## 1. Method (three axes, all compiler-grounded)

The published census says its method is "the union of `nm` and `gcc` negatives". I rebuilt it independently and found the union is necessary but **not sufficient**, so I added a third axis.

| axis | how | what it catches |
|---|---|---|
| **A. cross-TU** | `nm -u -f posix` over the 54 `src/**/*.o`; a defined symbol absent from the union of undefined sets has no caller in another TU | the 1 445 `nm` false positives (an intra-TU `static` call is never an undefined symbol) |
| **B. intra-TU** | comment- and string-stripped scan of the defining file, classified by **brace depth**: an occurrence at depth ≥ 1 is a real use (`call` if followed by `(`, `addr` if preceded by `&`, `bare` otherwise = table initialiser); an occurrence at depth 0 is a prototype or the definition and is *not* a use | intra-TU calls, **function pointers, and table initialisers** |
| **C. reachability** | a symbol defined by ≥ 1 `tests/*.o` as well is test-reachable; a symbol *defined* by a test object is shadowed and proves nothing | a test-local function sharing a name with a production one |

Definition sites come from **`gcc -aux-info`** (`NF` = definition, `NC` = declaration), not a regex, so definition lines and prototype lines are compiler-identified. Toolchain `c:\devkitPro\msys2\usr\bin\gcc.exe`, `-std=c11 -O0 -fno-inline -w`; all 54 src and 166 test TUs compiled with **0 failures**.

### Two errors I made and fixed, because they would have produced the same false positives the brief warned about

1. **Brace depth, not "followed by `;`".** My first intra-TU axis treated `rb_execute_performance_phase(g, x);` as a *prototype* because a `;` follows the parameter list. That silently promoted 53 genuinely-wired functions into TEST-ONLY, including `rb_execute_performance_phase` (called from `rb_advance_phase`, `src/turn/phase.c:335`) and all 7 `rb_probe_*`. Brace depth is the only sound discriminator: a prototype is at depth 0.
2. **`src/test_game.c` is not a production path.** It is a `src/**/*.c` file and it *is* linked into `rb_engine` (`OBJ_LIB` ends `src/test_game.o`), so counting it as a caller marks harness code as production. I re-ran with it excluded. **Delta: 0 functions** — no symbol is called from `test_game.c` and from `tests/` but not from production. Worth stating because it was a live risk, not a theoretical one.

### The published 262 versus my 263 — the count agreeing is a coincidence

| | published | measured at `5ce311e6` |
|---|---|---|
| definitions in `src/**/*.c` | 2 604 | **2 576** |
| WIRED | 1 853 | **1 856** |
| TEST-ONLY | 262 | **263** |
| DEAD | 489 | **459** |

(The definition-count gap is header accounting: the census counts 5 header definitions, my `gcc -aux-info` pass reports 14 distinct `NF` names in `include/*.h` when each header is compiled standalone; 13 symbols defined in more than one TU are excluded from all three buckets because a per-symbol bucket is ambiguous for them.)

**11 of the published 262 are NOT test-only at `5ce311e6` (9 census OVERCOUNT + 2 overtaken by the peer commit):**

| symbol | location | why it is not test-only | `ntestTU` |
|---|---|---|---|
| `card_name` | `src/ability/effects/misc.c:2271` | production `static const char *card_name(int,char*,size_t)`. The census's 14 "test call sites" are `tests/test_parity_look_per_card.c:343` — **the test's own** `static const char *card_name(int cid)`, different signature. `gcc -Wunused-function` proves the production one has no user. **DEAD**, not test-only. | 0 |
| `stage_area_of` | `src/ability/effects/move.c:423` | production `static int stage_area_of(RbPlayer*,int)`. **Three** test-local copies with different signatures: `test_parity_deploy_multi.c:294 (TestGame*,int)`, `test_parity_int_gaps.c:396 (TestGame*,int,int)`, `test_parity_jidou_move_yell.c:342`. **DEAD**, not test-only. | 0 |
| `rb_probe_nkeys`, `rb_probe_key`, `rb_probe_keycount`, `rb_probe_reset`, `rb_probe_ndropped_keys`, `rb_probe_dropped_key`, `rb_probe_dropped_tag` | `src/ability/vm.c:657-672` | `nm -u` over all 166 test objects: **zero references** (`ntestTU=0` for all seven). The census's "2/1/1/1/3/3/3 test call sites" are matches inside comments — the block is labelled "TEMPORARY PROBE INSTRUMENTATION (removed before hand-off)". **DEAD**, not test-only. | 0 |

This is a new instance of the trap the brief names: matching a function *by name* across the `src/`/`tests/` boundary without checking that the test side is calling the same function.

**Plus 2 more the census listed that a later commit legitimately re-wired** — `rb_queue_push_constant_context` and `rb_queue_pop_constant_context` were TEST-ONLY at `cf315a9` and are WIRED at `5ce311e6` (`src/core/modifiers.o` now calls both). Not a census error: the census was right about the tree it measured, which is precisely why this document states its HEAD. **10 genuinely test-only functions the census MISSED (census UNDERCOUNT — its "WIRED" bucket absorbed them), plus 2 that arrived test-only with the `5ce311e6` commit itself:**

| symbol | location | why the census missed it |
|---|---|---|
| `rb_resolution_add`, `rb_resolver_pending_choice`, `rb_resolver_get_pending_choice`, `rb_resolver_fmt_ids`, `rb_resolver_card_matches_type`, `rb_resolver_card_matches_cost_limit`, `rb_resolver_fmt_card`, `rb_resolver_merge_group_names`, `rb_resolver_zone_for_card` | `src/ability/resolver.c:53,55,1106,1210,1226,1243,1252,1310,1338` | each has a **forward declaration at the top of its own file** (`resolver.c:27-55`, `:1100-1110`). The census's intra-TU call map counted the prototype as a call. The real discriminator is `gcc -aux-info`: `NF` at the definition, no use anywhere at brace depth ≥ 1, `nm -u` empty. |
| `test_flat_verdict_failed` | `src/test_game.c:1297` | reached only through the **macro** `#define FLAT_VERDICT() test_flat_verdict_failed(__func__)` at `include/test_game.h:378`. Neither a call-syntax grep nor `nm -u` can see a macro expansion. |
| `rb_rules_execute_restriction` | `src/ability/effects/rules.c:1865` | **landed by the `5ce311e6` peer commit itself**, test-only on arrival. See §4/O3 — it is ORPHANED LOGIC, not a miss. |
| `rb_inject_choice_ability_context` | `src/core/game_state_abilities.c:1526` | **landed by the `5ce311e6` peer commit itself**, test-only on arrival. See §4/O2. |

Arithmetic closes exactly: **262 − 11 + 12 = 263.**

---

## 2. Did the census undercount in the "actually production-reachable" direction? **NO — 0 cases.**

The brief asks me to shout if I find one. I did not, and I checked three ways:

1. **Axis A** — no member of the 263 appears in the undefined set of any `src/**/*.o` other than `src/test_game.o` (and re-running without it changed nothing).
2. **Axis B** — no member has a `call`, `addr` or `bare` occurrence at brace depth ≥ 1 in its own translation unit.
3. **Macro / table-initialiser sweep** — for each of the 263 I swept every occurrence across `src/`, `include/`, `tools/`, `platforms/` and separated **prototypes** from real uses by walking the parameter list and checking for a trailing `;`. Every cross-file hit resolves to a prototype in `include/rabuka.h` / `include/test_game.h`, with two exceptions, both benign:
   * `rb_card_no_scratch_bytes`, `rb_card_no_scratch_layout_ok` → `tools/audit_w_gate.c:222,226`. A host tool, not `rb_engine`.
   * `rb_decode_fallback_count` → `tools/scratch/test_zzprobe.c`. A host tool.

`nm -u` also proves nothing is shadowed: 25 names are *defined* by a test object as well as a src object, and all 25 are excluded from the 263 before classification.

**So the defect the brief was hunting — a green test attesting to a function the shipped binary never reaches, where that function is actually reachable and therefore fine — does not exist here. The census did not undercount in that direction.**

---

## 3. Classification of all 263

| class | n | meaning |
|---|---|---|
| **SHIPPED-ELSEWHERE** | **168** | the behaviour runs in production through a different function or an inlined field access; the test-only export is a thin, usually one-line, wrapper |
| **HARNESS** | **75** | all of `src/test_game.c` — the fixture API declared in `include/test_game.h`. Not engine behaviour at all; it is in the census's class only because it is a `src/**/*.c` definition |
| **ORPHANED LOGIC** | **16** | the behaviour exists only on the test path while Rust reaches it in production (§4) |
| **DEAD-IN-RUST-TOO** | **4** | Rust has no production caller either; the test is exercising a convenience API that neither engine ships |
| **VERIFIED** | **0** | see §2 |

Key: `SE` = SHIPPED-ELSEWHERE, `DRT` = DEAD-IN-RUST-TOO, `HARN` = harness, `**ORPH**` = ORPHANED LOGIC, `**NO**` in the "in the published 262?" column = the census missed it. The table in §8 classifies all 263: 261 with a definition in a `src/**/*.c` file plus the 2 `static inline` header shims (`rb_saturate_i16`, `rb_card_filter_subset`) that `nm -u` cannot see because each test object emits its own local copy.
---

## 4. ORPHANED LOGIC, ranked by severity

### O3 — `rb_rules_execute_restriction` — `src/ability/effects/rules.c:1865` — **the only producer of a live, wired reader's input has zero callers**  ⭐ WORST

*This case arrived in the `5ce311e6` commit. It is the largest reachability of the three and the tree already half-knows about it.*

**The behaviour.** Rust has ONE restriction handler. `AbilityResolver::execute_restriction` (`engine/src/ability/effects/.../restriction.rs:26`) records, among other things:

* `GameState.wait_immune_members: Vec<(i16, PlayerId)>` — per-member WAIT immunity, **with a blade-count filter** (`rules.c:1962-1967`), and
* `GameState.cannot_activate_members[2]` (`rules.c:1916`).

The C has **two** handlers, and `src/ability/effects/misc.c:1049-1067` documents the divergence in its own words:

```
1052: THE HANDLER FAMILY IS TRIPLICATED and this file's copy is the one that runs:
1053:   1. misc.c h_restriction                 -- LIVE (executor.c:370, triggers.c:1207)
1054:   2. rules.c rb_rules_execute_restriction -- 0 callers
1055:   3. (the C's restriction engine tests)   -- fixtures, not a handler
1056: The three are NOT interchangeable, so delegating 1 to 2 would change
1057: behaviour: 2 records `st->wait_immune` per member with the blade-limit filter
1058: (:1881-1924) and `st->cannot_activate_members` (:1870), while 1 records
1059: `g->cannot_active_cards` (:1055-1060) and none of rules.c's wait_immune list.
1060: What was safe to lift was the ONE missing arm, and it is now in both.
```

**The readers are production-WIRED:**

```
src/ability/effects/rules.c:2279  rb_rules_is_wait_immune()   -- READER
src/ability/effects/state.c:391   if(!rb_rules_is_wait_immune(g, cands[i], controller))   <- candidate filter
src/ability/effects/state.c:492   if(!rb_rules_is_wait_immune(g, cands[i], controller))   <- candidate filter
src/ability/effects/rules.c:2300  rb_rules_cannot_activate_member()  <- reads st->cannot_activate_members[pl]
src/turn/phase.c:439,509,543      CLEARS cannot_activate_members[2] per turn
src/turn/phase.c:560              CLEARS wait_immune per turn
```

**The only writers are inside the uncalled twin:**

```
src/ability/effects/rules.c:1916   st->cannot_activate_members[pl] = 1;         <- inside rb_rules_execute_restriction
src/ability/effects/rules.c:1964-1967 st->wait_immune[...].member_id/owner_pl    <- inside rb_rules_execute_restriction
```

**Consequence in `rb_engine`:** `n_wait_immune` is permanently 0, so `rb_rules_is_wait_immune` always answers "not immune" and the two `state.c` candidate filters at `:391` and `:492` never remove anything. `cannot_activate_members[2]` is permanently 0 and `phase.c:439/509/543` clear a set nobody fills. **This is a check that can never fail** — the exact defect class the brief names, and it is the mirror image of the shape the tree already fixed once for `changed_state_members` (`misc.c:1040-1043`: *"What was safe to lift was the ONE missing arm, and it is now in both."* The other two arms were *"left alone on purpose because changing them is a behaviour change no Rust line demands"* — but a Rust line does demand them, because Rust has only the one handler.)

**Corpus reachability.** `state.c:373-376` cites Rust's `candidates.retain(|(_, cid)| !gs.wait_immune_members…`, i.e. the reader is the filter that chooses **which members an opponent may be forced into WAIT**. Broad upper bound over the 985 unique abilities: **48** abilities place a member in WAIT from an activated/auto ability. The ones that actually need `wait_immune` are the opponent-targeting, blade/heart-conditioned ones — the shape is on `PL!SP-bp4-011-P` / `-P＋` / `-R＋` / `-SEC`:

> {{自動}}このメンバーが登場か、エリアを移動したとき、**相手の**ステージにいる元々持つ{{ブレード}}の数が**3つ以下のメンバー1人**を…ウェイトにする

— a blade-count-gated forced-wait, i.e. exactly the arm `rules.c:1962-1967` implements with `st->wait_immune` and the blade-limit filter, and exactly what `h_restriction` does not record. Also in the 48: `PL!S-bp3-001-P/R＋/SEC`, `PL!S-bp3-006-P/R＋/SEC`, `PL!N-bp3-008-P/R＋/SEC`, `PL!SP-bp5-001-AR/P/R＋/SEC`, `PL!SP-bp5-002-AR/P/R＋/SEC`, `PL!-bp3-008/009-*`, `PL!N-…`, `PL!-PR-012-PR`, `PL!S-PR-038-PR`, `PL!SP-PR-017-PR`, `PL!SP-sd1-011-P/SD/SD2`.

**The test that asserts it:** `tests/test_parity_q68_cannot_live.c` — the only test TU whose object references the symbol. Green, and attesting to a producer the engine never calls.

**Rust citation:** `engine/src/core/game_state/mod.rs:149` declares `wait_immune_members`; `rules.rs`/`restriction.rs:26` is the single writer; `state.rs` the single reader. The C's `state.c:18-19` and `:373-376` already quote all three.

**Fix — lift the two missing arms into the LIVE handler, exactly the pattern already used for `changed_state_members`. Do NOT delegate `h_restriction` to `rb_rules_execute_restriction`; `misc.c:1056` is right that the two are not interchangeable.** Both `src/ability/effects/misc.c` and `src/ability/effects/rules.c` are on another agent's write list *right now*, so this is reported, not landed. Patch shape (the two blocks to copy into `misc.c`'s `h_restriction`, before its final `return 1;` at `misc.c:1075`):

```c
/* --- src/ability/effects/misc.c, inside static int h_restriction(...),
 *     immediately before its final `return 1;` (misc.c:1075) ---
 *
 * Lift the two arms that only the UNCALLED twin records. Without this,
 * st->n_wait_immune and st->cannot_activate_members are never written, so:
 *   - rb_rules_is_wait_immune (rules.c:2279) always answers "not immune", and the
 *     two candidate filters at state.c:391 and state.c:492 never remove anything;
 *   - rb_rules_cannot_activate_member (rules.c:2300) always answers 0, and
 *     phase.c:439/:509/:543 clear a set nobody fills.
 * Both are "a check that can never fail". Rust has ONE handler
 * (AbilityResolver::execute_restriction, restriction.rs:26) and records both.
 *
 * Copy, verbatim, from src/ability/effects/rules.c:
 *   :1916   st->cannot_activate_members[pl] = 1;                 (the cannot_activate arm)
 *   :1962-1967  the per-member wait_immune push, INCLUDING the
 *               blade-count filter that h_restriction has no equivalent of.
 *
 * The mirror-image readers already exist and are wired:
 *   rb_rules_is_wait_immune          rules.c:2279  <- state.c:391, state.c:492
 *   rb_rules_cannot_activate_member  rules.c:2300
 * The per-turn clears already exist and are wired:
 *   wait_immune_members.clear()       phase.c:560   (Rust phases.rs:281)
 *   cannot_activate_members.clear()   phase.c:439, :509, :543  (Rust phases.rs:279-280)
 *
 * REACHABILITY: 48 of the 985 corpus abilities force a member into WAIT from an
 * activated/auto ability; the shape that needs this arm is PL!SP-bp4-011-{P,P＋,R＋,SEC}
 * (blade-count-gated forced wait on the opponent's stage). See
 * engine_c_wip/TEST_ONLY_AUDIT.md O3.
 *
 * This needs RbRulesState. misc.c already names the rules.c file-static typedef
 * through the accessor pair `rb_rules_take_changed_state_members` /
 * `rb_rules_state`; add a matching `rb_rules_record_wait_immune(g, member_id,
 * owner_pl)` and `rb_rules_record_cannot_activate(g, pl)` writer-pair in
 * rules.c and call those from misc.c, so no struct has to be exposed.
 */
```

**Landing note.** This one *adds* prohibitions, so unlike O1 it should not make currently-green parity cases go red except where a case was passing because the restriction was silently not recorded. Quoting the suite delta is mandatory. Coordinate with whoever owns `misc.c`/`rules.c` — they are mid-edit.

---

### O1 — `rb_reset_keyword_tracking` — `src/core/tracking.c:38` — **a real missing engine path**

**The behaviour.** Per-turn (re)initialisation of the keyword-scoped trackers, called at the top of the Active phase. Body:

```
g->n_auto_ability_trigger_counts = 0;        /* :10  */
clear_area_placement_tracking(g);            /* :21  -> n_areas_placed_this_turn = 0 */
rb_reset_change_flags(g);                    /* WIRED */
g->live_success[0] = 0; g->live_success[1] = 0;   /* :56-57 */
g->cheer_check_completed = 0;                /* :58  */
rb_reset_loop_detection(g);                  /* WIRED */
rb_clear_baton_touch_tracking(g);            /* WIRED */
g->n_turn1_abilities_played = 0;             /* :40  */
g->n_turn2_abilities_played = 0;             /* :41  */
g->player1_cheer_blade_heart_count = 0;       /* :42  */
g->player2_cheer_blade_heart_count = 0;       /* :43  */
g->p[0].deck_refreshed_this_turn = 0;        /* :44  */
g->p[1].deck_refreshed_this_turn = 0;        /* :45  */
g->last_resolution_cards_p1.n = 0;           /* :46  */
g->last_resolution_cards_p2.n = 0;           /* :47  */
```

**Does Rust reach it? YES — in production.** `engine/src/core/game_state/tracking.rs:23` `pub fn reset_keyword_tracking`, called from **`engine/src/turn/phases.rs:62` and `engine/src/turn/phases.rs:98`** — the `Phase::Active` arm of `advance_phase`, twice per round, every round.

**Does the C have a production call site? NO.** `nm -u` on all 54 `src/**/*.o`: zero. Brace-depth ≥ 1 occurrence of the name in `tracking.c`: zero. The source file says so itself, at `tracking.c:27-30`:

> *"The C turn rollover in `src/turn/phase.c` has no equivalent call; see the report for the exact insertion point."*

**The test that asserts it:** `tests/test_parity_ability_watch.c` — the only test TU whose object references the symbol. Green, and attesting to a rollover that never happens in `rb_engine`.

**Reachability over the real corpus.** The consequence is not one field, it is **six trackers that never reset**. Measured over `cards/abilities.json` (985 unique abilities / 2116 pairs / 2639 cards):

| tracker cleared | reader in the C engine | producer in the C engine | corpus abilities | pairs | cards |
|---|---|---|---|---|---|
| `p[].deck_refreshed_this_turn` | `condition.c:542` (`temporal=this_turn`, `location=deck`) and `condition.c:2155` | **7 producers** (`draw.c:321`, `move.c:1091`, `rules.c:2034`, `live.c:154`, `phase.c:659`, `zones.c:134`, `tracking.c:127`) and **zero resets in production** | 4 | 7 | 7 |
| `player1/2_cheer_blade_heart_count` | `condition.c:2566` — `score_threshold_condition` | `game_state_core.c:439-440`; **zero resets in production** | 1 card in the printed shape (`PL!SP-pb2-048-L`, documented at `condition.c:2558`) | — | 1 |
| `live_success[0..1]` | `condition.c:2676`, `:2681-2682`, `triggers.c:462` | `live.c:717`, `:797`, `:1692` (monotonic); **zero resets in production** | 3 | 4 | 4 |
| `n_turn1/2_abilities_played` | none | **NO PRODUCER ANYWHERE** — the only write in the whole tree is the reset at `tracking.c:40-41` | 2 (`{{turn1.png|ターン1回}}`) | 6 | 6 |
| `n_auto_ability_trigger_counts` | none | **NO PRODUCER ANYWHERE** — only `tracking.c:10` | 2 | 5 | 5 |
| `n_areas_placed_this_turn` | none | **NO PRODUCER ANYWHERE** — only `tracking.c:21` | 11 (broad shape) | 24 | 24 |
| `last_resolution_cards_p1/p2` | none (`live.c:1950` says `g->revealed_cards` is the mirror) | **NO PRODUCER ANYWHERE** — only `tracking.c:46-47` | — | — | — |

Three of these are the exact defect shape the brief names — **a field with no producer** — and `include/rabuka.h:1755-1757` already carries `BLOCKED-ON: src/core/tracking.c:41 -- delete g->n_turn2_abilities_played = 0;`, i.e. a previous audit found the no-producer fact and deferred it.

**The single cleanest divergence, with its card number:**

> `PL!S-bp2-022-L` — 「{{live_success.png|ライブ成功時}}このターン、自分のデッキがリフレッシュしていた場合、このカードのスコアを＋２する。」
>
> `deck_refreshed_this_turn` is set by seven production sites and cleared by none. So after the player's **first** deck refresh of the game this card pays +2 for the rest of the game, including on every turn where nothing was refreshed. Rust clears it twice a round; C never clears it at all.

Other named cards on the latched `live_success[]` path: `PL!S-pb1-021-L` (「…**このターン**、相手が余剰のハートを持たずにライブを成功させていた場合」 — the exact 「このターン」 scoping the missing reset owns), `PL!S-bp3-005-P`/`-R`, `PL!S-bp6-024-L`. Cards on the no-excess path: `PL!N-bp5-007-*`, `PL!N-bp5-010-*`, `PL!-bp3-025-L`/`-SRL`, `PL!-bp4-023-L`/`-SRL`, `PL!HS-bp6-028-L`, `PL!N-bp3-027-L`, `PL!S-bp3-019-L`, `PL!S-bp5-020-L`, `PL!S-bp6-024-L`, `PL!S-pb1-021-L` (10 abilities / 17 pairs / 17 cards).

**Broad upper bound:** the string 「このターン」 appears in **40 of 985** unique abilities (**69 of 2116** pairs, **65 of 2639** cards). Not all 40 read a tracker this function clears, but 40 abilities are scoped by a keyword whose per-turn reset the C never performs.

**Rust citation is by content** because the oracle moves: in this tree `reset_keyword_tracking` exists at `tracking.rs:23` and is called at `phases.rs:62` and `phases.rs:98`. If your Rust checkout differs, re-check those two call sites before landing the C patch — a Rust tree where the calls are gone would make this DEAD-IN-RUST-TOO instead.

**Fix — insert into the Active-phase rollover in `src/turn/phase.c`.** The C's phase machine has one Active rollover site; `phase.c:655-670` (the block that already sets `P->deck_refreshed_this_turn = 1` at `:659` is the tail of the same phase).

```c
/* --- src/turn/phase.c, in the PHASE_ACTIVE rollover, immediately BEFORE the
 *     deck/shuffle bookkeeping at phase.c:655 -- ---
 *
 * Mirror engine/src/core/game_state/tracking.rs:23 `reset_keyword_tracking`,
 * called from the Phase::Active arm of advance_phase at engine/src/turn/phases.rs:62
 * and :98. Without this the six per-turn trackers below latch for the whole game:
 *   p[].deck_refreshed_this_turn          -> condition.c:542 and :2155 read it
 *   player1/2_cheer_blade_heart_count      -> condition.c:2566 reads it
 *   live_success[0..1]                     -> condition.c:2676, :2681, triggers.c:462
 *   n_turn1/2_abilities_played             -> no producer either (dead field)
 *   n_auto_ability_trigger_counts          -> no producer either (dead field)
 *   n_areas_placed_this_turn               -> no producer either (dead field)
 * Measured: PL!S-bp2-022-L pays its +2 from the first deck refresh of the game
 * onward; 40 of the 985 corpus abilities are 「このターン」-scoped.
 * See engine_c_wip/TEST_ONLY_AUDIT.md O1.
 */
rb_reset_keyword_tracking(g);
```

Owner: whoever owns `src/turn/phase.c`. **Do not edit `src/core/tracking.c`** — another agent owns it *and* is editing it right now; only add the call.

**Landing note / risk.** This is not behaviour-neutral: it will turn several currently-TRUE conditions FALSE on turns where nothing happened. That is the point (Rust already does it), but it means parity suites will move. Expect `tests/test_parity_ability_watch.c` to keep passing (it already assumes the reset) and expect some `test_parity_live*` / `test_parity_per_card*` cases that currently pass *because* the field is latched to flip to RED. **Land with the suite deltas quoted, not silently.**

---

### O2 — `rb_inject_choice_ability_context` — `src/core/game_state_abilities.c:1526` — **the choice-ability JSON context is built for nobody** (new at `5ce311e6`)

**The behaviour.** Serialises the current queue entry's `card_no` / `ability_text` / `card_name` into a JSON buffer for the UI. Its own comment at `:1516` names the reason it exists:

> *"The `_` arm passes the empty string through UNCHANGED, which matters: the dummy entry `queue.rs:367-409` pushes has an empty `player_id` on purpose, and the old C body substituted a hard "p1" there, which would have satisfied the leak filter at `engine/src/game/web_server.rs:1243` for the WRONG seat."*

**Rust reaches it in production**: `engine/src/game/web_server.rs:1243` region — the Rust web server builds this context and runs a leak filter over it.

**The C does not.** `src/web_server.c` has no `rb_rend_*`, no `rb_inject_choice_ability_context`, no choice-ability JSON; it serialises only `count`/`allow_skip`/`actor` at `:193-194`. `nm -u` over all 54 `src/**/*.o`: zero.

**The test that asserts it:** `tests/test_parity_choice_routing.c` — the only test TU whose object references it.

**Reachability:** not corpus-gated — it is the JSON the UI would show for *any* card ability that triggers a choice, i.e. every choice in the game. Its absence is user-visible (the C web UI cannot show which card/ability is asking).

**Fix:** call it from `src/web_server.c`'s pending-choice handler and run the seat leak filter Rust runs at `web_server.rs:1243`. `src/web_server.c` was one of the five files that changed under me and is not on my write list. **Needs an owner; reported, not landed.**

---

### O2b — `src/ability/choice_renderer.c` (14 functions) — **the card-choice grid never reaches a player**

**The behaviour.** `rb_rend_render_card_choice_grid` (`choice_renderer.c:1368`) plus the 13 `rb_rend_text_*` / `rb_rend_render_choice` entry points. `rb_rend_render_card_choice_grid` is a one-line forward to a file-local helper, so it is SHIPPED-ELSEWHERE *inside the file*; the defect is that **the file has no consumer at all**.

**Rust reaches it in production**: `engine/src/choice_renderer.rs:48 pub fn render_card_choice_grid`, called from **`engine/src/game/platform_ui.rs:243`**.

**The C does not.** Zero references from any `src/**/*.o` other than test objects, zero from `rb_web_server`, zero from `platforms/cdi/build_cdi.sh`, `platforms/genesis/build_genesis.sh`, `platforms/Makefile`. The Makefile already says it, at `:49-61` and `:246-249`:

> *"`src/ability/choice_renderer.c` … was in NO build target until 2026-10-03 … 1821 of its lines were compiled into nothing and no compiler had ever seen it. It is here now so that stops."*
> *"The C's nearest file, `src/ability/choice_renderer.c`, is an ORPHAN — zero references in OBJ_LIB, platforms/cdi/build_cdi.sh, platforms/genesis/build_genesis.sh or platforms/Makefile — so a suite could not link against it."*

**The test that asserts it:** `tests/test_choice_renderer.c` (Makefile:60). Green, and attesting to a renderer no player sees.

**Severity: product, not engine.** Engine choice *semantics* are covered elsewhere (`tests/test_parity_cce_gaps.c` and the rest of `test_parity_*`). What is unproven is that the C's actual UI — `rb_web_server` — presents a card-choice grid the way Rust's `platform_ui` does.

**Fix, two options, pick one and record which:**

*Option A (recommended — keep the file compiled, correct the claim):* leave `choice_renderer.c` in `SRC`/`OBJ_LIB` (the Makefile explicitly forbids removing it) and change `tests/test_choice_renderer.c`'s header comment to state the truth: this suite pins the *renderer library*, which no shipped binary currently calls; the player-visible parity claim for card choices is `test_parity_cce_gaps.c`, not this file. A comment — no assertion changes, no deletion.

*Option B (the real fix, larger):* wire `rb_rend_render_card_choice_grid` into `src/web_server.c`'s choice endpoint. **Out of scope here** — needs an owner and a UI spec. Pairs naturally with O2.

---

## 5. DEAD-IN-RUST-TOO — the correct close is a test comment, not a patch

All four are faithful C ports of a Rust API that **Rust itself never calls in production**. The C test is not lying about the C: it is exercising an API neither engine ships. **Do not add a production call site.** What each needs is one sentence in the test header saying so.

| symbol | C | Rust | Rust callers | test |
|---|---|---|---|---|
| `rb_perform_cheer_check` | `src/core/tracking.c:109` (6 test TUs) | `engine/src/core/game_state/tracking.rs:62 pub fn perform_cheer_check` | **0 in `engine/src`**; only `engine/tests/…` (5 files) | `test_parity_ability_mod_extra.c`, `test_parity_cost_discount.c`, `test_parity_jidou_extra.c`, `test_parity_jidou_watch.c`, `test_parity_jidou_watch_clusters.c`, `test_parity_restriction_invalidate.c` |
| `rb_need_satisfied` | `src/core/stats_pipeline.c:243` | `engine/src/core/stats_pipeline.rs:258` | **0 in `engine/src` AND 0 in `engine/tests`** | `test_parity_stats_pipeline.c` |
| `rb_stage_can_place_card` | `src/core/zones.c:371` — Rule 8.2.2, only non-live cards on the stage | `engine/src/core/zones.rs:458 Stage::can_place_card`, `!card.is_live()` | **0 in `engine/src`**; the live gate is `AbilityResolver::can_place_card_in_zone` (`abilities.rs:2937`, called from `effects/move/placement.rs:279`) — exactly the C's `rb_can_place_card_in_zone` | `test_parity_zones_position.c` |
| `rb_live_can_place_card` | `src/core/zones.c:600` — `return 1` unconditionally | `engine/src/core/zones.rs:518 LiveCardZone::can_place_card` — also `true` unconditionally | called from `zones.rs:524 add_card`, so it *is* reached, but the call is a no-op in both engines | `test_audit_zones.c` |

`rb_perform_cheer_check` deserves one extra sentence because it looks like the biggest one and is not. Its body is the **only** writer in the entire `src/` tree of `cheer_checks_done`, `cheer_check_completed` and `cheer_checks_required`:

```
src/core/tracking.c:112-118   the ONLY assignments of cheer_checks_done / cheer_check_completed / cheer_checks_required
```

and its only producer of resolution-zone entries is `rb_resolution_add_for_owner` (`zones.c:730`), which IS wired — but production reaches it from nowhere on this path. In `rb_engine` the yell is `do_yell` (`src/turn/live.c:180`, static, wired), which writes `yell_cards[]` and the snapshot and **never** populates `g->resolution` from the tracking path. So `g->cheer_checks_required` stays 0 and the gate `cheer_checks_done < cheer_checks_required` (`tracking.c:196-197`, inside `rb_check_required_hearts`, itself also uncalled) is `0 < 0` → satisfied. **The gate is vacuous in production — but it is vacuous in Rust too**, which is why this is a close, not a patch.

`rb_need_satisfied` is already documented correctly at `src/core/stats_pipeline.c:21-24` ("The Rust twin of `need_satisfied` has ZERO callers in `engine/src` AND in `engine/tests`"). Verified: true in this tree. The header of `tests/test_parity_stats_pipeline.c` should say the same so the next audit does not re-open it.

---

## 6. Defects found WITHOUT fixing (read-only on my side)

**D1 — `src/test_game.c` ships in `rb_engine`.** All 75 harness functions are linked into the shipped binary (`OBJ_LIB` ends `src/test_game.o`), and the census classes them as "test-only" only because nothing in `rb_engine` calls them. 75 dead functions are in the product. This is deliberate — the Makefile comment on `choice_renderer.c` and the `test_flat_verdict_failed` macro at `include/test_game.h:378` show the tree's position is "keep it compiled so the port stays a port". Recording it as a **decision that should be written down**, not a bug to fix.

**D2 — three copies of `card_total_hearts`.** `rb_card_total_hearts` (`src/ability/effects/state.c:2044`, test-only) is byte-equivalent to `static int card_total_hearts` at `src/ability/condition.c:4011` (WIRED, 8 call sites) **and** at `src/ability/util.c:863` (WIRED). Three copies of a 10-line rule, two live, one exported for tests. Any fix to the Q149/Q172 base-vs-need fallback has to land three times. Consolidate into one exported function and delete the two statics — but `condition.c` and `util.c` are on another agent's write list right now.

**D3 — `rb_ability_debug_set` (`src/ability/debug.c:12`) has no production writer, so `g_ability_debug` is `false` in every shipped binary.** 19 test TUs call the setter. Legitimate for tests, but it means the C engine has **no way to enable ability tracing in production at all** — Rust's `debug.rs` reads a build/`env` switch. If tracing is an operational requirement, the C is missing an env reader. Not a test-assertion defect; flagging for an owner.

**D4 — `include/rabuka.h:2716` has a `#define rb_card_filter_subset(e,out) rb_effect_filter_subset((e),(out))` with a `static inline int rb_card_filter_subset(...)` of the same name at `:2724`.** Harmless today — the `#define` sits inside the `/* … */` block that opens at `:2715` and closes at `:2723`. But a macro and a static inline sharing a name in one header is a trap: moving one line destroys the other. Recommend deleting the dead `#define`.

**D5 — `engine/src/game/platform_ui.rs:243` is a Rust production consumer with no C counterpart at all.** Beyond O2b, this means the C has no equivalent of Rust's `card_ability_text` either (Makefile:242-249 moved `card_info_show`/`card_info_parity` to `SUITE_ABSENT` for exactly this reason). Recorded so it is not rediscovered as a parity gap.

**D6 — the class is growing.** `5ce311e6` shipped two new test-only exports (`rb_inject_choice_ability_context`, `rb_rules_execute_restriction`) and one of them (O3) hides the only producer of a live reader's input. Whatever process lands test-only exports should run axis-A/B/C on each one, because "add an export so a test can reach it" is exactly how O3 happened.

---

## 7. Fix list, ordered

| # | kind | file | what | who |
|---|---|---|---|---|
| 1 | **production call site** | `src/ability/effects/misc.c` + `src/ability/effects/rules.c` | O3: lift the `wait_immune` + `cannot_activate_members` recording into the LIVE `h_restriction`. Patch in §4/O3. Do **not** delegate `h_restriction` to `rb_rules_execute_restriction` | blocked — both mid-edit by another agent |
| 2 | **production call site** | `src/turn/phase.c` (Active rollover, before `:655`) | O1: add `rb_reset_keyword_tracking(g);` — patch in §4/O1. Do **not** edit `src/core/tracking.c` | `phase.c` owner |
| 3 | **production call site** | `src/web_server.c` | O2: call `rb_inject_choice_ability_context` from the pending-choice handler and run Rust's seat leak filter (`web_server.rs:1243`) | needs an owner |
| 4 | test comment | `tests/test_parity_q68_cannot_live.c` | say that `rb_rules_execute_restriction` has 0 production callers and the live handler is `misc.c h_restriction`, so the `wait_immune` / `cannot_activate_members` arms are unrecorded until fix #1 lands | test owner |
| 5 | test comment | `tests/test_parity_ability_watch.c` | say the C has no Active-phase rollover call, per `tracking.c:27-30`, and that the suite is asserting the call site fix #2 adds | test owner |
| 6 | test comment | `tests/test_parity_stats_pipeline.c` | state that `need_satisfied` has zero Rust callers (`stats_pipeline.rs:258`) as well as zero C ones; `stats_pipeline.c:21-24` already says it | test owner |
| 7 | test comment | `tests/test_parity_zones_position.c` (the `stage_can_place_card` case) | state that Rust `Stage::can_place_card` (`zones.rs:458`) has zero `engine/src` callers and the live gate is `abilities.rs:2937` | test owner |
| 8 | test comment | the 6 `perform_cheer_check` suites | state that `perform_cheer_check` is Rust-test-only too; production yells via `do_yell` (`live.c:180`) and the `cheer_checks_*` gate is vacuous in both engines | test owner |
| 9 | test comment | `tests/test_choice_renderer.c` | Option A from §4/O2b: the suite pins the renderer *library*, which no shipped binary calls | test owner |
| 10 | cleanup | `src/ability/condition.c:4011`, `src/ability/util.c:863`, `src/ability/effects/state.c:2044` | D2: keep one `rb_card_total_hearts`, delete the two statics | blocked — both on another agent's list |
| 11 | cleanup | `include/rabuka.h:2716` | D4: delete the dead `#define rb_card_filter_subset` | coordinator (`include/rabuka.h` is theirs) |
| 12 | decision | `Makefile`, `src/test_game.c` | D1: write down that `src/test_game.c` is deliberately shipped, or split it out of `OBJ_LIB` | coordinator (Makefile is theirs) |
| 13 | process | — | D6: run axes A/B/C on every new test-only export before landing it | whoever reviews exports |

**No test was deleted, and no assertion was weakened.** Items 4-9 are additive comments only; items 1-3 are the only changes that move engine behaviour.

---

## 8. Full enumeration — all 263

Reached-from lists the `tests/*.c` translation units whose **compiled object** carries an undefined reference to the symbol (`nm -u`), which is linkage-level proof, not a text match.

### `include/rabuka.h`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 186 | `rb_saturate_i16` | SE | yes | — (header inline) | a saturating i16 clamp; the C never needs an i16 window - `rb_saturate_u8` (rabuka.h:181) and `rb_saturate_i32` (rabuka.h:196) cover the live conversions. Reached only from `tests/test_describe_golden.c:368-369` |
| 2724 | `rb_card_filter_subset` | SE | yes | — (header inline) | a one-line forward to `rb_effect_filter_subset` (include/rabuka.h:2724-2726). Reached only from `tests/test_ported_generated.c` |

### `src/ability/ability_queue.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 103 | `rb_queue_init` | SE | yes | `test_audit_triggers`, `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 151 | `rb_queue_is_idle` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 163 | `rb_queue_is_waiting_for_choice` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 197 | `rb_queue_len` | SE | yes | `test_audit_triggers`, `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 201 | `rb_queue_is_empty` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 205 | `rb_queue_has_pending` | SE | yes | `test_parity_per_card2` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 209 | `rb_queue_iter` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 326 | `rb_queue_start_next` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 626 | `rb_queue_pause_for_auto_ability_choice` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 650 | `rb_queue_resume_with_choice` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 756 | `rb_queue_promote_entry` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 787 | `rb_queue_promote_entry_by_abs` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 804 | `rb_queue_set_current_entry` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 1073 | `rb_queue_take_resolver` | SE | yes | `test_p1_helpers`, `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 1080 | `rb_queue_set_resolver` | SE | yes | `test_p1_helpers`, `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 1085 | `rb_queue_has_resolver` | SE | yes | `test_p1_helpers`, `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 1103 | `rb_queue_has_entry_with_id` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 1113 | `rb_queue_pending_entries` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 1132 | `rb_queue_dump_state` | SE | yes | `test_parity_queue_resume` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |

### `src/ability/choice.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 5673 | `rb_choice_allow_skip` | SE | yes | `test_parity_cce_gaps` | ch->allow_skip read directly at choice.c:1140,:1146,:1521,:1594,:2025 |

### `src/ability/choice_renderer.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 1368 | `rb_rend_render_card_choice_grid` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1608 | `rb_rend_text_new` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1664 | `rb_rend_text_free` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1675 | `rb_rend_text_script` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1695 | `rb_rend_text_stuck` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1697 | `rb_rend_text_n_frames` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1698 | `rb_rend_text_n_lines` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1699 | `rb_rend_text_n_draws` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1700 | `rb_rend_text_overflowed` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1701 | `rb_rend_text_line` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1705 | `rb_rend_text_frame_line` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1713 | `rb_rend_text_draw` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1726 | `rb_rend_text_frame_draws` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |
| 1865 | `rb_rend_render_choice` | **ORPH** | yes | `test_choice_renderer` | Rust calls it in production (`platform_ui.rs:243`); C has no consumer in any target |

### `src/ability/compound.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 366 | `rb_route_conditional_branch` | SE | yes | `test_parity_cost_compound` | route_conditional_branch_ptr (src/ability/compound.c:358), called at :743,:748 |

### `src/ability/cost.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 1956 | `rb_validate_cost` | SE | yes | `test_parity_cost_compound`, `test_parity_fw_cost_cardtype` | validate_cost (src/ability/cost.c, static) via rb_validate_mandatory_cost (:1963); engine.c:1187 |

### `src/ability/debug.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 12 | `rb_ability_debug_set` | SE | yes | `test_cross_player_interaction`, `test_opponent_live_success_flow`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_conditional_gated`, `test_parity_cost_discount`, `test_parity_draw_until_count`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_jidou_state`, `test_parity_recover_score_state_delta`, `test_parity_required_hearts`, `test_parity_rules_conditions2`, `test_parity_rules_conditions`, `test_parity_rules_phases`, `test_parity_small_clusters`, `test_parity_zone_source` | no production writer - the flag is compile-time off; see DEFECT D3 |

### `src/ability/describe.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 1776 | `rb_desc_cost_en` | SE | yes | `test_describe_golden` | rb_desc_cost (src/ability/describe.c, WIRED) |
| 1780 | `rb_desc_cost_ja` | SE | yes | `test_describe_golden` | rb_desc_cost (src/ability/describe.c, WIRED) |
| 1818 | `rb_desc_sequential_cost_en` | SE | yes | `test_describe_golden` | rb_desc_cost over a sequential list (src/ability/describe.c) |
| 1823 | `rb_desc_sequential_cost_ja` | SE | yes | `test_describe_golden` | rb_desc_cost over a sequential list (src/ability/describe.c) |
| 1832 | `rb_desc_zone_label` | SE | yes | `test_describe_golden` | d_zone_label (src/ability/describe.c, WIRED) |
| 1836 | `rb_desc_zone_label_ja` | SE | yes | `test_describe_golden` | d_zone_label (src/ability/describe.c, WIRED) |
| 1840 | `rb_desc_card_type_label` | SE | yes | `test_describe_golden` | d_cond_card_type_label (src/ability/describe.c, WIRED) |
| 1844 | `rb_desc_card_type_label_ja` | SE | yes | `test_describe_golden` | d_cond_card_type_label (src/ability/describe.c, WIRED) |
| 1848 | `rb_desc_state_verb` | SE | yes | `test_describe_golden` | rb_desc_state_verb_ja (src/ability/describe.c, WIRED) |
| 1856 | `rb_desc_resource_label` | SE | yes | `test_describe_golden` | d_resource_label (src/ability/describe.c, WIRED) |
| 1860 | `rb_desc_resource_label_ja` | SE | yes | `test_describe_golden` | d_resource_label (src/ability/describe.c, WIRED) |
| 1864 | `rb_desc_duration_label` | SE | yes | `test_describe_golden` | d_duration_label (src/ability/describe.c, WIRED) |
| 1874 | `rb_desc_duration_label_ja` | SE | yes | `test_describe_golden` | d_duration_label (src/ability/describe.c, WIRED) |
| 2946 | `rb_desc_translate_choice_prompt_en_to_ja` | SE | yes | `test_describe_golden` | the JA prompt table in rb_desc_choice (src/ability/describe.c) |

### `src/ability/effects/rules.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 1865 | `rb_rules_execute_restriction` | **ORPH** | **NO** | `test_parity_q68_cannot_live` | the UNCALLED twin holds the ONLY producer of st->wait_immune[] (rules.c:1964-1967) and st->cannot_activate_members[] (rules.c:1916). The LIVE handler is static h_restriction (misc.c:1068), reached from executor.c:370 and triggers.c:1207, and misc.c:1049-1060 states it records "none of rules.c wait_immune list". The reader rb_rules_is_wait_immune (rules.c:2279) IS wired - state.c:391,:492 - so the filter never removes anything in rb_engine. See ORPH O3. |

### `src/ability/effects/state.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 2044 | `rb_card_total_hearts` | SE | yes | `test_parity_card_identity`, `test_parity_full_round3` | — |

### `src/ability/enums.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 33 | `rb_target_player_from_str` | SE | yes | `test_parity_enums` | gsc_id_is / the target resolver in src/core/game_state_core.c |
| 52 | `rb_target_player_as_str` | SE | yes | `test_parity_enums` | rb_target_player_to_str (src/ability/enums.c, WIRED) |
| 57 | `rb_action_type_from_str` | SE | yes | `test_audit_actions`, `test_parity_char2`, `test_parity_characterization`, `test_parity_enums`, `test_parity_rules_extra`, `test_parity_rules` | the action dispatch strcmp chain in src/ability/effects/dispatch equivalent (executor.c) |
| 212 | `rb_action_type_label` | SE | yes | `test_audit_actions`, `test_parity_enums` | rb_action_type_to_str (src/ability/enums.c, WIRED) |
| 291 | `rb_condition_type_from_str` | SE | yes | `test_parity_enums`, `test_parity_rules_conditions` | rb_condition_type_from_str_at / the decoder switch (src/ability/vm.c) |
| 329 | `rb_condition_type_to_str` | SE | yes | `test_parity_enums` | the condition-type label switch in describe.c:2265 |
| 390 | `rb_select_target_kind_to_str` | SE | yes | `test_parity_enums` | rb_select_target_kind_from_str (src/ability/enums.c, WIRED) |
| 431 | `rb_effect_card_type_default` | SE | yes | `test_parity_enums` | RB_EFFECT_CARD_TYPE_MEMBER default arm in the executor |
| 436 | `rb_effect_state_from_str` | SE | yes | `test_parity_enums` | the state-effect action dispatch (src/ability/effects/state.c) |
| 443 | `rb_effect_state_as_str` | SE | yes | `test_parity_enums` | the state-effect label switch in describe.c |
| 452 | `rb_effect_state_default` | SE | yes | `test_parity_enums` | the default arm of the state-effect dispatch |

### `src/ability/log.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 51 | `rb_log_set_enabled` | SE | yes | `test_audit_actions`, `test_log_drain`, `test_parity_char2`, `test_parity_cost_discount`, `test_parity_executor_dispatch`, `test_parity_required_hearts`, `test_target_selection` | — |
| 223 | `rb_log_push_verdict_child` | SE | yes | `test_log_drain` | rb_log_push_verdict_effect (src/ability/log.c, WIRED) |
| 269 | `rb_log_clear_verdicts` | SE | yes | `test_log_drain`, `test_parity_char2`, `test_parity_executor_dispatch`, `test_target_selection` | rb_log_drain_verdicts_since (src/ability/log.c, WIRED) |

### `src/ability/resolver.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 253 | `rb_resolver_zone_for_card` | SE | **NO** | `test_parity_resolver` | the zone resolver in rb_resolver_zone (src/ability/resolver.c) |
| 279 | `rb_resolver_get_pending_choice` | SE | **NO** | `test_parity_resolver` | g->queue.pending read directly (src/ability/choice.c:963,:2025) |
| 283 | `rb_resolver_pending_choice` | SE | **NO** | `test_parity_resolver` | g->queue.pending read directly (src/ability/choice.c:963,:2025) |
| 298 | `rb_resolver_cached_condition_verdict` | SE | yes | `test_parity_resolver`, `test_target_selection` | the verdict cache read inside rb_condition_eval (src/ability/condition.c) |
| 340 | `rb_resolver_store_condition_verdict` | SE | yes | `test_parity_resolver`, `test_target_selection` | the verdict cache write inside rb_condition_eval (src/ability/condition.c) |
| 679 | `rb_resolver_card_matches_type` | SE | **NO** | `test_parity_resolver` | rb_card_matches_type (src/ability/util.c, WIRED) |
| 682 | `rb_resolver_card_matches_cost_limit` | SE | **NO** | `test_parity_resolver` | rb_card_cost_limit_cmp (src/ability/resolver.c) |
| 686 | `rb_resolver_fmt_card` | SE | **NO** | `test_parity_resolver` | rb_card_name (src/core/card.c, WIRED) |
| 695 | `rb_resolver_fmt_ids` | SE | **NO** | `test_parity_resolver` | the id-list format built inline in the resolver messages |
| 705 | `rb_resolver_merge_group_names` | SE | **NO** | `test_parity_resolver` | rb_effect_group_names_any (src/ability/resolver.c, WIRED) |
| 1210 | `rb_resolve_ability` | SE | yes | `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_state_per_card`, `test_parity_zone_source`, `test_target_selection` | resolve_ability_body (src/ability/resolver.c, WIRED) |
| 1223 | `rb_resolver_drain_verdicts` | SE | yes | `test_target_selection` | rb_resolver_drain_verdicts_since (src/ability/resolver.c, WIRED) |
| 1235 | `rb_resolver_push_verdict` | SE | yes | `test_target_selection` | rb_log_push_verdict (src/ability/log.c, WIRED) |

### `src/ability/types.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 101 | `rb_value_ref_init_literal` | SE | yes | `test_condition_evaluation`, `test_vm_decode` | the literal arm of rb_value_ref_make / rb_value_ref_copy (src/ability/types.c) |
| 107 | `rb_value_ref_init_step` | SE | yes | `test_condition_evaluation`, `test_vm_decode` | the step arm of the same switch |
| 113 | `rb_value_ref_init_accepted` | SE | yes | `test_condition_evaluation`, `test_vm_decode` | the accepted arm of the same switch |
| 119 | `rb_value_ref_init_offset` | SE | yes | `test_condition_evaluation`, `test_vm_decode` | the offset arm of the same switch |
| 136 | `rb_value_ref_kind_str` | SE | yes | `test_vm_decode` | rb_value_ref_kind_to_str (src/ability/types.c, WIRED) |
| 140 | `rb_value_ref_kind_from_str` | SE | yes | `test_vm_decode` | the kind parse in rb_value_ref_decode (src/ability/types.c) |
| 149 | `rb_value_ref_resolve` | SE | yes | `test_condition_evaluation`, `test_vm_decode` | the step/offset resolution inside rb_value_ref_eval (src/ability/types.c) |
| 174 | `rb_value_ref_is_literal` | SE | yes | `test_vm_decode` | ref->kind == RB_VR_LITERAL checked inline by the resolvers |
| 738 | `rb_choice_builder_new` | SE | yes | `test_parity_target_player` | rb_full_choice_new_select_target / rb_choice_new (src/ability/choice.c) |
| 751 | `rb_choice_builder_build` | SE | yes | `test_parity_target_player` | rb_full_choice_set_options / rb_choice_finalize (src/ability/choice.c) |
| 854 | `rb_choice_builder_picker` | SE | yes | `test_parity_target_player` | ch->picker assigned inline by the choice builders |

### `src/ability/util.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 331 | `rb_card_has_blade_heart_strict` | SE | yes | `test_parity_blades2`, `test_parity_card_identity`, `test_parity_yell`, `test_ported_generated` | rb_card_has_blade_heart (src/ability/util.c:325) - laxer superset |
| 939 | `rb_card_matches_name_constraint` | SE | yes | `test_parity_deploy_recover` | rb_card_matches_name_fragments (src/ability/util.c, WIRED) |
| 1129 | `rb_get_zone_card_count` | SE | yes | `test_audit_zones` | rb_count_in_zone (src/ability/util.c:1146) - byte-identical zone branches, and rb_get_zone_card_count falls through to it |

### `src/ability/vm.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 1279 | `rb_decode_keywords` | SE | yes | `test_vm_decode` | decode_keywords_value (src/ability/vm.c, WIRED) |
| 1295 | `rb_count_empty_bytecode_abilities` | SE | yes | `test_parity_characterization`, `test_vm_decode` | rb_count_abilities (src/ability/vm.c, WIRED) |
| 1350 | `rb_get_ability` | SE | yes | `test_describe_golden`, `test_parity_characterization` | rb_decode_card_ability (src/ability/vm.c, WIRED) |
| 2753 | `decode_condition_direct` | SE | yes | `test_parity_t7_condition_decoder` | decode_condition_field (src/ability/vm.c, WIRED) |
| 2911 | `rb_decode_fallback_count` | SE | yes | `test_decode_fallback`, `test_parity_characterization`, `test_parity_full_round3`, `test_parity_t7_condition_decoder` | rb_note_decode_fallback (src/ability/vm.c, WIRED) |
| 2917 | `rb_decode_fallback_abilities` | SE | yes | `test_decode_fallback`, `test_parity_characterization` | rb_note_decode_fallback (src/ability/vm.c, WIRED) |
| 3077 | `rb_verify_generated_tables` | SE | yes | `test_artifacts_consistent`, `test_decode_skew`, `test_parity_char2` | the gen_data.c self-check the loader runs (src/core/generated/gen_data.c) |

### `src/core/card.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 21 | `rb_card_type_from_str` | SE | yes | `test_parity_card_identity` | rb_condition_card_type_from_str (src/core/card.c) |
| 29 | `rb_card_type_str` | SE | yes | `test_parity_card_identity` | rb_card_type_to_str / d_cond_card_type_label (src/core/card.c, src/ability/describe.c) |
| 147 | `rb_card_database_create_copy` | SE | yes | `test_ported_generated` | rb_register_card_copy (src/core/data.c, WIRED) |
| 378 | `rb_map_series_to_group` | SE | yes | `test_parity_card_identity`, `test_parity_gain_delta` | rb_card_series_to_group (src/core/card.c) |
| 589 | `rb_card_short_label` | SE | yes | `test_parity_card_identity` | rb_card_name / the _ja label path in src/core/card.c describe helpers |
| 731 | `rb_effect_per_unit_heart_colors_any` | SE | yes | `test_ported_generated` | rb_effect_heart_colors_any (src/core/card.c) |
| 734 | `rb_effect_options_any` | SE | yes | `test_ported_generated` | e->options walked inline in the effect dispatchers (src/ability/effects/*.c) |
| 753 | `rb_effect_destination_any` | SE | yes | `test_ported_generated` | e->destination read inline; see rb_effect_action_by_any |
| 758 | `rb_effect_is_under_self` | SE | yes | `test_ported_generated` | e->destination == "under_self" read inline in the move effects (src/ability/effects/move.c) |
| 765 | `rb_effect_action_by` | SE | yes | `test_ported_generated` | rb_effect_action_by_any (src/core/card.c) |
| 768 | `rb_effect_picker_any` | SE | yes | `test_parity_target_player`, `test_ported_generated` | e->picker read inline in the choice builders (src/ability/choice.c) |
| 771 | `rb_effect_non_stackable_any` | SE | yes | `test_ported_generated` | e->non_stackable read inline by the state effects |
| 813 | `rb_card_get_card` | SE | yes | `test_parity_card_identity` | rb_decode_card_by_index / rb_card_record (src/core/card.c) |
| 817 | `rb_card_has_trigger` | SE | yes | `test_parity_card_identity` | rb_ability_has_trigger (src/core/card.c, WIRED - triggers.c:540,:560) |
| 846 | `rb_card_triggerless_text` | SE | yes | `test_parity_card_identity` | rb_ability_triggerless_text (src/core/card.c, WIRED) |
| 1258 | `rb_card_fires_on_opponent_effects` | SE | yes | `test_parity_card_identity` | rb_card_has_draw_icon / the trigger scan in triggers.c |
| 1305 | `rb_card_energy_cost_total` | SE | yes | `test_parity_card_identity` | the cost pipeline in src/ability/cost.c reads ab.cost directly |
| 1312 | `rb_card_has_optional_payment` | SE | yes | `test_parity_card_identity` | rb_cost_optional_skipped / rb_cost_is_optional in src/ability/cost.c |
| 1319 | `rb_card_effective_energy_cost_total` | SE | yes | `test_parity_card_identity` | rb_mods_get_cost_set consumers in src/core/modifiers.c |
| 1342 | `rb_heartmap_is_empty` | SE | yes | `test_parity_card_identity` | rb_heartmap_len / rb_heartmap_value_at (src/core/card.c, WIRED) |
| 1346 | `rb_heartmap_values_sum` | SE | yes | `test_parity_card_identity` | rb_heartmap_value_at (src/core/card.c, WIRED) |
| 1364 | `rb_heartmap_contains_key` | SE | yes | `test_parity_card_identity` | rb_heartmap_key_at (src/core/card.c, WIRED) |
| 1400 | `rb_heartmap_remove` | SE | yes | `test_parity_card_identity` | rb_heartmap_set / rb_heartmap_clear_at (src/core/card.c) |
| 1421 | `rb_heartmap_clear` | SE | yes | `test_parity_card_identity` | rb_heartmap_clear_at (src/core/card.c) |
| 1432 | `rb_heartmap_entry_or_default` | SE | yes | `test_parity_card_identity` | rb_heartmap_value_at default arm (src/core/card.c) |

### `src/core/data.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 331 | `rb_card_stridx` | SE | yes | `test_audit_zones` | gen_data.c direct table access (src/core/generated/gen_data.c) |
| 530 | `rb_card_no_scratch_layout_ok` | SE | yes | `test_audit_warnings` | rb_card_no_scratch_bytes (src/core/data.c) - used by tools/audit_w_gate.c, not rb_engine |
| 915 | `rb_zone_id_from_ability_zone` | SE | yes | `test_parity_rules`, `test_zone_filter_parity` | the zone-id table in src/core/data.c read inline |
| 922 | `rb_zone_id_to_ability_zone` | SE | yes | `test_parity_rules`, `test_zone_filter_parity` | the zone-id table in src/core/data.c read inline |
| 1129 | `rb_score_to_i32` | SE | yes | `test_describe_golden` | the inline saturating casts in src/core/card.c / src/ability/util.c |
| 1144 | `rb_score_to_i32_rounded` | SE | yes | `test_describe_golden` | the inline saturating casts in src/core/card.c / src/ability/util.c |
| 1167 | `rb_overridden_stat` | SE | yes | `test_describe_golden` | rb_effective_stat (src/core/data.c) |
| 1178 | `rb_floored_cost` | SE | yes | `test_describe_golden` | rb_mods_apply_cost / rb_floored_cost inline in src/ability/cost.c |
| 1186 | `rb_count_u32` | SE | yes | `test_describe_golden` | rb_saturate_u32 (include/rabuka.h) |
| 1195 | `rb_id_u32` | SE | yes | `test_describe_golden` | rb_saturate_id (include/rabuka.h) |
| 1204 | `rb_f32_count` | SE | yes | `test_describe_golden` | rb_saturate_f32 (include/rabuka.h) |

### `src/core/game_state_abilities.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 67 | `rb_ability_has_remaining_uses` | SE | yes | `test_parity_auto_abilities`, `test_parity_recover_live`, `test_parity_search_reorder`, `test_tp_turn_limited_activation`, `test_trigger_paths` | rb_use_limit_reached (src/ability/ability_queue.c:1191) at triggers.c:591; also engine.c:1273,:1352 and resolver.c:270,:1000 |
| 1445 | `rb_is_loop_detected` | SE | yes | `test_parity_rules_extra` | g->loop_detected is read in production by rb_generate_state_hash (gsa.c:1423); set by rb_check_permanent_loop (gsa.c:1432) |
| 1482 | `rb_set_opponent_live_success` | SE | yes | `test_parity_per_card3` | inlined at src/turn/live.c:1113-1115, :1061-1066, :1692-1694 |
| 1526 | `rb_inject_choice_ability_context` | SE | **NO** | `test_parity_choice_routing` | no C consumer. Its own comment (game_state_abilities.c:1516) cites Rust engine/src/game/web_server.rs:1243 leak-filter; the C src/web_server.c builds no choice-ability JSON context. Test-only export shipped by the 5ce311e6 commit. |
| 1972 | `rb_trigger_auto_ability_by_index` | SE | yes | `test_parity_auto_abilities` | rb_trigger_auto_ability (gsa.c:1813) / rb_push_owned_entry (triggers.c:96) |
| 3506 | `rb_queue_reset` | SE | yes | `test_parity_auto_abilities` | the in-place reset at the top of rb_drain_ability_queue (src/ability/ability_queue.c) |
| 3614 | `rb_queue_set_pending_choice` | SE | yes | `test_harness_self`, `test_p1_helpers` | g->queue.pending assigned inline (src/ability/choice.c:1068,:1441) |

### `src/core/modifiers.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 182 | `rb_mods_get_cost_set` | SE | yes | `test_parity_cost_discount` | the cost-set field read inline in src/ability/cost.c |
| 264 | `rb_mods_set_heart_override` | SE | yes | `test_parity_characterization_extra`, `test_parity_stats_pipeline` | rb_mods_set_need_heart (src/core/modifiers.c, WIRED) |
| 274 | `rb_mods_get_heart_override` | SE | yes | `test_parity_rules_conditions` | rb_mods_get_need_heart (src/core/modifiers.c, WIRED) |
| 322 | `rb_mods_trace_len` | SE | yes | `test_condition_evaluation`, `test_parity_conditions`, `test_parity_fw_effects` | — |
| 703 | `rb_get_baton_touch_count` | SE | yes | `test_baton_touch`, `test_harness_self`, `test_parity_deploy_recover`, `test_parity_full_round`, `test_parity_rules_phases` | baton_touch_count_for (src/ability/condition.c:1781) and the direct field reads at condition.c:579, misc.c:1335 |
| 932 | `rb_clear_card_movement_tracking` | SE | yes | `test_parity_deploy_recover`, `test_parity_foreign_cause` | rb_clear_movement_tracking (src/core/game_state_abilities.c:601) |
| 1201 | `rb_mods_weighted_zone_count` | SE | yes | `test_parity_cluster_e` | rb_mods_count_as_multiple_weight (src/core/modifiers.c, WIRED) |
| 1266 | `rb_mods_replacement_appear_wait_applies` | SE | yes | `test_parity_cluster_e` | the appearance-wait check inside rb_add_replacement_effect callers |

### `src/core/stats_pipeline.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 243 | `rb_need_satisfied` | DRT | yes | `test_parity_stats_pipeline` | Rust stats_pipeline::need_satisfied is only a definition (engine/src/core/stats_pipeline.rs:258); zero callers in engine/src AND engine/tests. stats_pipeline.c:21-24 already says so. |

### `src/core/tracking.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 38 | `rb_reset_keyword_tracking` | **ORPH** | yes | `test_parity_ability_watch` | Rust GameState::reset_keyword_tracking IS called in production: engine/src/turn/phases.rs:98, the Phase::Active arm of advance_phase (twice per round). The C has NO call site; src/core/tracking.c:27-30 admits it. |
| 109 | `rb_perform_cheer_check` | DRT | yes | `test_parity_ability_mod_extra`, `test_parity_cost_discount`, `test_parity_jidou_extra`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_restriction_invalidate` | Rust GameState::perform_cheer_check (engine/src/core/game_state/tracking.rs:62) has ZERO callers in engine/src; only engine/tests call it (5 test files). The C production yell is do_yell (live.c:180). |

### `src/core/zones.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 12 | `rb_member_area_to_str` | SE | yes | `test_parity_zones_position` | AREA_* -> name table in src/core/zones.c used by the display/describe path |
| 18 | `rb_member_area_front` | SE | yes | `test_parity_zones_position` | rb_member_area_to_index front arm (src/core/zones.c, WIRED) |
| 39 | `rb_zone_invariant` | SE | yes | `test_audit_zones` | the two rules it checks are enforced by rb_stage_place_under_card / rb_live_add_card bounds; see zones.c:39-116 |
| 118 | `rb_zone_refresh` | SE | yes | `test_audit_zones` | rb_yell_refresh_deck (src/turn/live.c:135, static, WIRED); also rules.c:2024 and phase.c:530 rb_player_refresh. zones.c:112-116 names all three |
| 138 | `rb_member_area_to_tag` | SE | yes | `test_parity_zones_position` | tag encode inline in the move/position effects |
| 146 | `rb_member_area_from_tag` | SE | yes | `test_parity_zones_position` | tag decode inline (rb_stage_position_change_with_bags) |
| 186 | `rb_stage_get_area` | SE | yes | `test_parity_zones_position` | P->stage[area] read inline in the move effects |
| 190 | `rb_stage_set_area` | SE | yes | `test_parity_zones_position` | rb_stage_place_under_card / rb_stage_move_with_bags (src/core/zones.c) |
| 271 | `rb_stage_position_change` | SE | yes | `test_parity_zones_position` | rb_stage_position_change_with_bags (src/core/zones.c) |
| 371 | `rb_stage_can_place_card` | DRT | yes | `test_parity_zones_position` | Rust Stage::can_place_card (zones.rs:458) has ZERO callers in engine/src; the live gate is AbilityResolver::can_place_card_in_zone (abilities.rs:2937), the same function the C uses. |
| 413 | `rb_stage_formation_change` | SE | yes | `test_parity_zones_position` | rb_misc_finalize_formation_change (src/ability/effects/misc.c:1861) |
| 569 | `rb_energy_set_active_count` | SE | yes | `replay`, `test_parity_ability_watch`, `test_parity_hearts_constants`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_wave1b`, `test_parity_wave1c` | P->energy.active_count assigned inline by the energy effects (src/ability/effects/energy.c) |
| 600 | `rb_live_can_place_card` | DRT | yes | `test_audit_zones` | Rust LiveCardZone::can_place_card (zones.rs:518) IS called - from add_card (zones.rs:524) - but is `true` unconditionally, so the call is a no-op in both engines. |
| 752 | `rb_resolution_add` | SE | **NO** | `test_parity_rules_conditions` | rb_resolution_add_for_owner (src/core/zones.c:730) - WIRED |
| 756 | `rb_resolution_clear` | SE | yes | `test_parity_jidou_extra` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 765 | `rb_resolution_len` | SE | yes | `test_parity_rules_conditions` | `g->queue.*`/`g->resolution` fields read+written inline by the production drain (`ability_queue.c:180-714,1330-1500`; resolution drain `phase.c:770-800`) |
| 805 | `rb_member_area_from_index` | SE | yes | `test_parity_zones_position` | rb_member_area_to_index (src/core/zones.c, WIRED) |
| 842 | `rb_energy_deck_is_empty` | SE | yes | `test_parity_rules` | P->energy_deck.n == 0 read inline in the energy effects |
| 885 | `rb_live_calculate_score` | SE | yes | `test_audit_zones`, `test_mechanics` | the score sum in rb_populate_live / allocate_and_verdict (src/turn/live.c:790-800) |
| 959 | `rb_exclusion_add` | SE | yes | `test_audit_zones` | rb_waitroom_add / rb_deck_add (src/core/zones.c, WIRED) |
| 1154 | `rb_player_draw_card` | SE | yes | `test_audit_zones` | rb_zone_draw (src/core/zones.c) |

### `src/engine.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 22 | `rb_rng_checkpoint` | SE | yes | `replay` | rb_rng_state (src/engine.c) |
| 23 | `rb_rng_restore` | SE | yes | `replay` | rb_rng_set_state (src/engine.c) |
| 26 | `rb_lcg_new` | SE | yes | `replay` | rb_lcg_next_u64 (src/engine.c) + rb_rng_seed/rng_range (src/engine.c:10-40) |
| 34 | `rb_lcg_range` | SE | yes | `replay` | rng_range (src/engine.c) |
| 388 | `rb_execute_effect` | SE | yes | `replay`, `test_parity_int_gaps` | rb_execute_effect_ex (src/engine.c:263) - WIRED; n_recently_moved cleared at engine.c:774,:1214,:1403-1404 and ability_queue.c:1420,:1569 |

### `src/game/deck_builder.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 121 | `rb_build_deck_from_list` | SE | yes | `test_deck_builder` | rb_build_deck_from_card_numbers (src/game/deck_builder.c) |

### `src/game/deck_parser.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 330 | `rb_deck_status_str` | SE | yes | `test_deck_parser` | RbDeckStatus printed inline in src/game/deck_parser.c:1-60 |
| 343 | `rb_deck_list_to_card_numbers` | SE | yes | `test_deck_parser` | rb_build_deck_from_card_numbers (src/game/deck_builder.c) |

### `src/test_game.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 22 | `test_id` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_ability_effects_crash`, `test_ability_effects`, `test_audit_actions`, `test_audit_warnings`, `test_audit_zones`, `test_b8_live_timing`, `test_baton_touch`, `test_condition_evaluation`, `test_cross_player_interaction`, `test_draw_phase_fix`, `test_harness_self`, `test_live_allocator`, `test_live_card_zone_movement`, `test_live_success_rules`, `test_live_zone_slot_validation`, `test_mechanics`, `test_member_activation_debut_live_start`, `test_opponent_live_success_flow`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_auto_abilities`, `test_parity_blades2`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_deploy_move_missing`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_full_round2`, `test_parity_full_round`, `test_parity_gain_delta`, `test_parity_hearts_constants`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_per_card`, `test_parity_queue_resume`, `test_parity_realcard_walkthrough`, `test_parity_recover_to_hand`, `test_parity_resolver`, `test_parity_restriction`, `test_parity_reveal_extra`, `test_parity_rules_conditions`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_effects`, `test_parity_sequential`, `test_parity_state_family`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_parity_support`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_yell`, `test_parity_zone_source`, `test_parity_zones_position`, `test_performance_phase_rules`, `test_phase_machine_rules`, `test_ported_generated`, `test_position_and_source`, `test_restriction_mechanics`, `test_surplus_heart`, `test_survey_probe`, `test_target_selection`, `test_tp_activation_offer`, `test_tp_joint_card_live_start`, `test_tp_turn_limited_activation`, `test_tp_zone_change_gate`, `test_trigger_paths`, `test_zone_filter_parity` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 26 | `test_new_id` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_ability_effects_crash`, `test_ability_effects`, `test_audit_triggers`, `test_audit_warnings`, `test_condition_evaluation`, `test_harness_self`, `test_live_card_zone_movement`, `test_live_zone_slot_validation`, `test_look_select`, `test_member_activation_debut_live_start`, `test_move_cards`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_auto_abilities`, `test_parity_blade_under`, `test_parity_blades2`, `test_parity_blades_live_start`, `test_parity_bp7`, `test_parity_card_score`, `test_parity_cce_gaps`, `test_parity_char2`, `test_parity_characterization_extra`, `test_parity_cluster_e`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_deck_order`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_delta`, `test_parity_draw_flat`, `test_parity_energy_position_delta`, `test_parity_energy_remainder`, `test_parity_executor_dispatch`, `test_parity_foreign_cause`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_full_round`, `test_parity_fw_cost_cardtype`, `test_parity_fw_effects`, `test_parity_fx_fixers`, `test_parity_gain_delta`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_look_reveal`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_per_card`, `test_parity_queue_resume`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_score_state_delta`, `test_parity_recover_to_hand`, `test_parity_required_hearts`, `test_parity_resolver`, `test_parity_restriction_invalidate`, `test_parity_restriction`, `test_parity_reveal_extra`, `test_parity_rules_conditions2`, `test_parity_rules_conditions`, `test_parity_rules_extra`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_effects`, `test_parity_score_per_card2`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_parity_support`, `test_parity_t6_action_setup`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_wave1b`, `test_parity_wave1c`, `test_parity_x_class`, `test_parity_yell`, `test_parity_zone_source`, `test_parity_zones_position`, `test_performance_phase_rules`, `test_position_and_source`, `test_survey_probe`, `test_target_selection`, `test_tp_joint_card_live_start`, `test_tp_turn_limited_activation`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 58 | `test_card_copy_pool_reset` | HARN | yes | `test_audit_warnings`, `test_harness_self`, `test_parity_char2`, `test_parity_draw_delta`, `test_parity_energy_position_delta`, `test_parity_fw_cost_cardtype`, `test_parity_fw_effects`, `test_parity_live_total2`, `test_parity_score_per_card2`, `test_parity_t6_action_setup` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 86 | `test_id_ref` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 103 | `test_id_exact` | HARN | yes | `test_audit_conditions`, `test_audit_warnings`, `test_harness_self`, `test_parity_char2`, `test_parity_full_round3`, `test_parity_jidou_life`, `test_parity_jidou_watch_clusters`, `test_parity_live_total2`, `test_parity_q68_cannot_live`, `test_parity_rules_conditions2`, `test_parity_score_per_card2` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 122 | `test_add_to_hand` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_audit_triggers`, `test_audit_warnings`, `test_b8_live_timing`, `test_condition_evaluation`, `test_cross_player_interaction`, `test_harness_self`, `test_live_success_rules`, `test_member_activation_debut_live_start`, `test_move_cards`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_blade_under`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_char2`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_custom_misc`, `test_parity_deck_order`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_delta`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_energy_position_delta`, `test_parity_energy_remainder`, `test_parity_executor_dispatch`, `test_parity_full_round`, `test_parity_fw_cost_cardtype`, `test_parity_fw_effects`, `test_parity_gain_delta`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_look_per_card`, `test_parity_look_reveal`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_per_card`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_score_state_delta`, `test_parity_recover_to_hand`, `test_parity_required_hearts`, `test_parity_restriction`, `test_parity_reveal_extra`, `test_parity_rules_conditions2`, `test_parity_rules_conditions`, `test_parity_rules_extra`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_effects`, `test_parity_score_per_card2`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_parity_support`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_x_class`, `test_parity_yell`, `test_parity_zone_source`, `test_parity_zones_position`, `test_performance_phase_rules`, `test_phase_machine_rules`, `test_ported_generated`, `test_restriction_mechanics`, `test_surplus_heart`, `test_survey_probe`, `test_target_selection`, `test_tp_joint_card_live_start`, `test_tp_turn_limited_activation`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 123 | `test_add_to_discard` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_audit_zones`, `test_live_card_zone_movement`, `test_live_success_rules`, `test_look_select`, `test_move_cards`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_blade_under`, `test_parity_blades_live_start`, `test_parity_characterization_extra`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_cost_discount`, `test_parity_deck_order`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_executor_dispatch`, `test_parity_full_round`, `test_parity_gain_delta`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_look_reveal`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_queue_resume`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_to_hand`, `test_parity_rules_conditions2`, `test_parity_rules_phases`, `test_parity_score_per_card2`, `test_parity_sequential`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_zone_source`, `test_tp_activation_offer`, `test_tp_turn_limited_activation`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 127 | `test_add_to_stage` | HARN | yes | `replay`, `test_ability_effects_crash`, `test_ability_effects`, `test_audit_actions`, `test_audit_conditions`, `test_audit_triggers`, `test_audit_warnings`, `test_audit_zones`, `test_harness_self`, `test_live_allocator`, `test_live_success_rules`, `test_mechanics`, `test_member_activation_debut_live_start`, `test_move_cards`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_auto_abilities`, `test_parity_blades_live_start`, `test_parity_characterization_extra`, `test_parity_cluster_e`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_custom_misc`, `test_parity_deploy_move_missing`, `test_parity_draw_flat`, `test_parity_executor_dispatch`, `test_parity_fw_cost_cardtype`, `test_parity_fw_effects`, `test_parity_fx_fixers`, `test_parity_hearts_constants`, `test_parity_integration`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_per_card`, `test_parity_q68_cannot_live`, `test_parity_recover_score_state_delta`, `test_parity_restriction`, `test_parity_rules`, `test_parity_score_per_card2`, `test_parity_support`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_position_and_source`, `test_restriction_mechanics`, `test_target_selection`, `test_tp_activation_offer`, `test_tp_joint_card_live_start`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 132 | `test_place_under` | HARN | yes | `test_audit_zones`, `test_p1_helpers`, `test_parity_ability_mod`, `test_parity_blade_under`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_deploy_move_missing`, `test_parity_gain_delta`, `test_parity_hearts_constants`, `test_parity_live_total`, `test_parity_look_per_card`, `test_parity_parser_e2e_extra`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_zone_source`, `test_parity_zones_position` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 140 | `test_add_to_success` | HARN | yes | `replay`, `test_audit_actions`, `test_audit_zones`, `test_live_success_rules`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_condition_and_effect`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_deploy_recover`, `test_parity_draw_delta`, `test_parity_draw_flat`, `test_parity_executor_dispatch`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_integration`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_parser_e2e_extra`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_required_hearts`, `test_parity_score_effects`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_wave1a`, `test_parity_wave1c`, `test_parity_x_class`, `test_parity_zone_source` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 154 | `test_add_to_deck` | HARN | yes | `replay`, `test_condition_evaluation`, `test_live_success_rules`, `test_look_select`, `test_mechanics`, `test_p1_helpers`, `test_parity_area_move`, `test_parity_card_score`, `test_parity_condition_and_effect`, `test_parity_conditions`, `test_parity_custom_misc`, `test_parity_deck_order`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_executor_dispatch`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_look_reveal`, `test_parity_queue_resume`, `test_parity_recover_to_hand`, `test_parity_score_per_card2`, `test_parity_search_reorder`, `test_parity_small_clusters`, `test_parity_stats_pipeline`, `test_parity_support`, `test_parity_trigger_paths`, `test_parity_x_class`, `test_parity_zones_position`, `test_ported_generated`, `test_restriction_mechanics`, `test_surplus_heart`, `test_survey_probe`, `test_target_selection` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 158 | `test_add_to_deck_pl` | HARN | yes | `test_audit_warnings`, `test_audit_zones`, `test_b8_live_timing`, `test_condition_evaluation`, `test_cross_player_interaction`, `test_harness_self`, `test_live_card_zone_movement`, `test_live_success_rules`, `test_live_zone_slot_validation`, `test_look_select`, `test_mechanics`, `test_member_activation_debut_live_start`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_blade_under`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_cce_gaps`, `test_parity_char2`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_draw_flat`, `test_parity_energy_remainder`, `test_parity_full_round3`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_mill_deck`, `test_parity_per_card`, `test_parity_recover_q79`, `test_parity_required_hearts`, `test_parity_rules_extra`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_per_card2`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_parity_support`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_yell`, `test_parity_zones_position`, `test_performance_phase_rules`, `test_phase_machine_rules`, `test_ported_generated`, `test_surplus_heart`, `test_survey_probe`, `test_tp_joint_card_live_start`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 164 | `test_add_to_energy_deck` | HARN | yes | `test_audit_zones`, `test_live_success_rules`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_compound_per_card`, `test_parity_cost_and_effect`, `test_parity_custom_misc`, `test_parity_deploy_move_missing`, `test_parity_executor_dispatch`, `test_parity_full_round3`, `test_parity_integration_extra`, `test_parity_jidou_extra`, `test_parity_jidou_move_yell`, `test_parity_jidou`, `test_parity_live_total`, `test_parity_per_card2`, `test_parity_per_card`, `test_parity_rules`, `test_parity_state_per_card`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_zone_source`, `test_parity_zones_position`, `test_phase_machine_rules`, `test_surplus_heart`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 172 | `test_insert_deck_top` | HARN | yes | `test_harness_self`, `test_p1_helpers`, `test_parity_abilitymod_conditional`, `test_parity_blades2`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_full_round3`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_state`, `test_parity_jidou_watch`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_per_card`, `test_parity_recover_to_hand`, `test_parity_reveal_extra`, `test_parity_search_reorder`, `test_parity_state_per_card`, `test_parity_support`, `test_parity_yell`, `test_performance_phase_rules`, `test_phase_machine_rules` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 180 | `test_add_to_energy` | HARN | yes | `test_audit_zones`, `test_p1_helpers`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_energy_remainder`, `test_parity_jidou_extra`, `test_parity_live_total`, `test_parity_recover_q79`, `test_parity_rules_conditions`, `test_parity_score_per_card2`, `test_parity_under_member` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 191 | `test_set_energy_active` | HARN | yes | `replay`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_compound_per_card`, `test_parity_cost_discount`, `test_parity_energy_remainder`, `test_parity_full_round3`, `test_parity_fw_cost_cardtype`, `test_parity_live_total`, `test_parity_mill_deck`, `test_parity_recover_q79`, `test_parity_score_per_card2`, `test_parity_state_per_card`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 195 | `test_add_to_revealed` | HARN | yes | `test_audit_conditions`, `test_parity_ability_mod_extra`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_custom_misc`, `test_parity_integration_rounds`, `test_parity_live_total2`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_score_per_card2`, `test_parity_under_member`, `test_parity_zone_source` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 199 | `test_give_opp_energy` | HARN | yes | `test_p1_helpers`, `test_parity_blades_live_start`, `test_parity_jidou_watch_clusters`, `test_parity_stats_pipeline`, `test_parity_target_player` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 218 | `test_give_energy` | HARN | yes | `replay`, `test_ability_effects_crash`, `test_baton_touch`, `test_cross_player_interaction`, `test_harness_self`, `test_live_success_rules`, `test_member_activation_debut_live_start`, `test_opponent_live_success_flow`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_blade_under`, `test_parity_blades2`, `test_parity_blades_live_start`, `test_parity_bp7`, `test_parity_card_score`, `test_parity_cce_gaps`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_custom_misc`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_delta`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_energy_position_delta`, `test_parity_energy_remainder`, `test_parity_executor_dispatch`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_full_round`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_per_card`, `test_parity_queue_resume`, `test_parity_realcard_walkthrough`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_score_state_delta`, `test_parity_recover_to_hand`, `test_parity_required_hearts`, `test_parity_restriction_invalidate`, `test_parity_restriction`, `test_parity_reveal_extra`, `test_parity_rules_conditions2`, `test_parity_rules_conditions`, `test_parity_rules_extra`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_per_card2`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_parity_support`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_wave1b`, `test_parity_yell`, `test_parity_zone_source`, `test_parity_zones_position`, `test_ported_generated`, `test_restriction_mechanics`, `test_target_selection`, `test_tp_activation_offer`, `test_tp_joint_card_live_start`, `test_tp_turn_limited_activation`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 253 | `test_play_to_stage` | HARN | yes | `replay`, `test_baton_touch`, `test_cross_player_interaction`, `test_harness_self`, `test_live_success_rules`, `test_member_activation_debut_live_start`, `test_parity_ability_mod_extra`, `test_parity_ability_watch`, `test_parity_area_move`, `test_parity_blade_under`, `test_parity_blades_live_start`, `test_parity_bp7`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_delta`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_energy_position_delta`, `test_parity_energy_remainder`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_full_round`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_per_card`, `test_parity_queue_resume`, `test_parity_realcard_walkthrough`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_score_state_delta`, `test_parity_recover_to_hand`, `test_parity_required_hearts`, `test_parity_reveal_extra`, `test_parity_rules_conditions`, `test_parity_rules_extra`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_support`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_wave1b`, `test_parity_wave1c`, `test_parity_zone_source`, `test_parity_zones_position`, `test_ported_generated`, `test_target_selection`, `test_tp_joint_card_live_start`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 309 | `test_try_play_to_stage` | HARN | yes | `test_harness_self`, `test_parity_bp7`, `test_parity_cost_discount`, `test_parity_deploy_multi`, `test_parity_draw_flat`, `test_parity_parser_e2e_extra`, `test_parity_rules_conditions2`, `test_parity_rules`, `test_parity_trigger_paths`, `test_parity_zones_position`, `test_restriction_mechanics` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 336 | `test_play_to_stage_baton` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 360 | `test_play_to_stage_double_baton` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 368 | `test_recalc` | HARN | yes | `replay`, `test_ability_effects_crash`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_blade_under`, `test_parity_blades2`, `test_parity_blades_live_start`, `test_parity_bp7`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_deploy_recover`, `test_parity_fw_cost_cardtype`, `test_parity_fw_effects`, `test_parity_gain_delta`, `test_parity_hearts_constants`, `test_parity_int_gaps`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_move_yell`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_per_card`, `test_parity_recover_hand_a`, `test_parity_recover_score_state_delta`, `test_parity_restriction`, `test_parity_score_per_card2`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_wait_activation`, `test_parity_wave1b`, `test_parity_zone_source`, `test_restriction_mechanics`, `test_tp_activation_offer` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 369 | `test_clear_mods_for_card` | HARN | yes | `test_parity_card_score`, `test_parity_cost_compound`, `test_parity_cost_discount` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 370 | `test_set_opp_stage` | HARN | yes | `replay`, `test_audit_conditions`, `test_live_success_rules`, `test_mechanics`, `test_parity_cost_and_effect`, `test_parity_hearts_constants`, `test_parity_int_gaps`, `test_parity_integration`, `test_parity_live_total`, `test_parity_parser_e2e_extra`, `test_parity_q68_cannot_live`, `test_parity_under_member`, `test_tp_joint_card_live_start`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 375 | `test_add_to_opp_live` | HARN | yes | `replay`, `test_live_success_rules`, `test_mechanics`, `test_parity_custom_misc`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_live_total`, `test_parity_score_effects`, `test_tp_zone_change_gate` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 376 | `test_add_to_opp_success` | HARN | yes | `replay`, `test_ability_effects_crash`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_condition_and_effect`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_integration`, `test_parity_live_total`, `test_parity_recover_hand_a` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 380 | `test_fire_debut` | HARN | yes | `test_audit_warnings`, `test_parity_abilitymod_conditional`, `test_parity_conditional_gated`, `test_parity_full_round2`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 382 | `test_activate_ability` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_member_activation_debut_live_start`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_area_move`, `test_parity_blade_under`, `test_parity_blades2`, `test_parity_bp7`, `test_parity_cce_gaps`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_energy_position_delta`, `test_parity_energy_remainder`, `test_parity_full_round3`, `test_parity_full_round`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou`, `test_parity_live_total`, `test_parity_mill_deck`, `test_parity_per_card2`, `test_parity_per_card`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_to_hand`, `test_parity_restriction`, `test_parity_rules`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_support`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_wave1b`, `test_parity_wave1c`, `test_parity_zone_source`, `test_parity_zones_position`, `test_tp_activation_offer`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 402 | `test_spend_energy` | HARN | yes | `test_harness_self`, `test_p1_helpers`, `test_parity_hearts_constants`, `test_parity_score_per_card2` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 407 | `test_drain_auto_choices` | HARN | yes | `replay`, `test_cross_player_interaction`, `test_harness_self`, `test_parity_conditional_gated`, `test_parity_required_hearts`, `test_parity_restriction`, `test_parity_zone_source` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 418 | `test_resume_choice` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_harness_self`, `test_live_card_zone_movement`, `test_live_success_rules`, `test_mechanics`, `test_move_cards`, `test_opponent_live_success_flow`, `test_parity_card_score`, `test_parity_char2`, `test_parity_deploy_move_missing`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_score_per_card2`, `test_parity_trigger_paths`, `test_ported_generated`, `test_surplus_heart`, `test_tp_activation_offer`, `test_tp_joint_card_live_start`, `test_tp_turn_limited_activation`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 431 | `test_select_indices` | HARN | yes | `replay`, `test_ability_effects_crash`, `test_baton_touch`, `test_cross_player_interaction`, `test_harness_self`, `test_member_activation_debut_live_start`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_draw_flat`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_integration_rounds`, `test_parity_jidou_move_yell`, `test_parity_live_total`, `test_parity_realcard_walkthrough`, `test_parity_rules_phases`, `test_parity_sequential`, `test_parity_trigger_paths`, `test_parity_yell`, `test_phase_machine_rules`, `test_restriction_mechanics`, `test_surplus_heart`, `test_target_selection`, `test_tp_joint_card_live_start`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 436 | `test_has_pending_choice` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `test_ability_effects_crash`, `test_audit_warnings`, `test_baton_touch`, `test_condition_evaluation`, `test_cross_player_interaction`, `test_harness_self`, `test_live_card_zone_movement`, `test_live_success_rules`, `test_mechanics`, `test_member_activation_debut_live_start`, `test_move_cards`, `test_opponent_live_success_flow`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_auto_abilities`, `test_parity_blade_under`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_cce_gaps`, `test_parity_char2`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_cost_discount`, `test_parity_deploy_move_missing`, `test_parity_draw_delta`, `test_parity_draw_flat`, `test_parity_draw_until_count`, `test_parity_energy_position_delta`, `test_parity_energy_remainder`, `test_parity_hearts_live_start`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_per_card`, `test_parity_recover_score_state_delta`, `test_parity_required_hearts`, `test_parity_rules_conditions`, `test_parity_rules_extra`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_per_card2`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_stats_pipeline`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_zone_source`, `test_performance_phase_rules`, `test_phase_machine_rules`, `test_ported_generated`, `test_restriction_mechanics`, `test_surplus_heart`, `test_target_selection`, `test_tp_activation_offer`, `test_tp_joint_card_live_start`, `test_tp_turn_limited_activation`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 467 | `test_pending_choice_count` | HARN | yes | `test_harness_self`, `test_p1_helpers`, `test_parity_characterization_extra`, `test_parity_cost_and_effect`, `test_parity_live_total2`, `test_parity_look_per_card`, `test_parity_score_per_card2` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 474 | `test_set_live_card` | HARN | yes | `replay`, `test_harness_self`, `test_live_success_rules`, `test_member_activation_debut_live_start`, `test_opponent_live_success_flow`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_abilitymod_conditional`, `test_parity_blade_under`, `test_parity_blades2`, `test_parity_blades_live_start`, `test_parity_bp7`, `test_parity_card_score`, `test_parity_cce_gaps`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_cost_discount`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_delta`, `test_parity_full_round2`, `test_parity_full_round`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_integration_rounds`, `test_parity_integration`, `test_parity_jidou_life`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_live_total2`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_per_card`, `test_parity_realcard_walkthrough`, `test_parity_required_hearts`, `test_parity_restriction_invalidate`, `test_parity_restriction`, `test_parity_reveal_extra`, `test_parity_rules_conditions`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_score_per_card2`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_support`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_under_member`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_wave1b`, `test_parity_yell`, `test_parity_zone_source`, `test_phase_machine_rules`, `test_ported_generated`, `test_survey_probe`, `test_tp_joint_card_live_start` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 508 | `test_insert_live_card_at` | HARN | yes | `test_p1_helpers`, `test_parity_per_card` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 563 | `test_card_name` | HARN | yes | `test_harness_self`, `test_parity_ability_mod_extra`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_deploy_recover`, `test_parity_energy_remainder`, `test_parity_full_round3`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_integration_extra`, `test_parity_jidou`, `test_parity_live_total2`, `test_parity_required_hearts`, `test_parity_rules_phases`, `test_parity_score_per_card2`, `test_parity_under_member`, `test_tp_activation_offer`, `test_tp_turn_limited_activation` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 578 | `test_stage_has` | HARN | yes | `test_audit_triggers`, `test_member_activation_debut_live_start`, `test_p1_helpers`, `test_parity_card_score`, `test_parity_live_total2`, `test_parity_score_per_card2`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 582 | `test_hand_has` | HARN | yes | `test_live_success_rules`, `test_member_activation_debut_live_start`, `test_move_cards`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_abilitymod_conditional`, `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_deploy_move_missing`, `test_parity_draw_flat`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_rules_conditions2`, `test_parity_support`, `test_parity_zone_source`, `test_tp_joint_card_live_start` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 587 | `test_print_board` | HARN | yes | `test_parity_recover_to_hand` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 593 | `test_pending_choice_type` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `replay`, `test_condition_evaluation`, `test_cross_player_interaction`, `test_harness_self`, `test_live_card_zone_movement`, `test_live_success_rules`, `test_member_activation_debut_live_start`, `test_p1_helpers`, `test_parity_ability_mod`, `test_parity_ability_watch`, `test_parity_abilitymod_conditional`, `test_parity_card_score`, `test_parity_cce_gaps`, `test_parity_char2`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_energy_position_delta`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_full_round`, `test_parity_hearts_live_start`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_life`, `test_parity_jidou_state`, `test_parity_jidou_watch2`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_jidou`, `test_parity_live_total`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_realcard_walkthrough`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_score_state_delta`, `test_parity_recover_to_hand`, `test_parity_restriction`, `test_parity_reveal_extra`, `test_parity_rules_conditions`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_search_reorder`, `test_parity_small_clusters`, `test_parity_support`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_zone_source`, `test_performance_phase_rules`, `test_phase_machine_rules`, `test_restriction_mechanics`, `test_surplus_heart`, `test_survey_probe`, `test_target_selection`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 607 | `test_get_blade_modifier` | HARN | yes | `replay`, `test_condition_evaluation`, `test_cross_player_interaction`, `test_parity_ability_mod_extra`, `test_parity_blades_live_start`, `test_parity_characterization_extra`, `test_parity_condition_and_effect`, `test_parity_conditional_gated`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_full_round3`, `test_parity_hearts_live_start`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_jidou_extra`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_jidou`, `test_parity_parser_e2e_extra`, `test_parity_per_card`, `test_parity_rules_conditions`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_wait_activation`, `test_parity_zone_source`, `test_restriction_mechanics`, `test_tp_joint_card_live_start`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 608 | `test_get_score_modifier` | HARN | yes | `replay`, `test_ability_effects`, `test_parity_ability_mod_extra`, `test_parity_abilitymod_conditional`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_characterization_extra`, `test_parity_condition_and_effect`, `test_parity_conditions`, `test_parity_cost_and_effect`, `test_parity_deploy_recover`, `test_parity_full_round3`, `test_parity_integration`, `test_parity_jidou_move_yell`, `test_parity_jidou_state`, `test_parity_live_total2`, `test_parity_live_total`, `test_parity_parser_e2e_extra`, `test_parity_per_card`, `test_parity_recover_q79`, `test_parity_rules_conditions`, `test_parity_score_effects`, `test_parity_score_per_card2`, `test_parity_state_per_card`, `test_parity_stats_pipeline`, `test_restriction_mechanics` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 609 | `test_get_cost_modifier` | HARN | yes | `replay`, `test_parity_ability_mod_extra`, `test_parity_area_move`, `test_parity_characterization_extra`, `test_parity_conditional_gated`, `test_parity_cost_compound`, `test_parity_integration`, `test_parity_jidou_move_yell`, `test_parity_under_member`, `test_parity_zone_source` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 625 | `test_get_heart_modifier` | HARN | yes | `replay`, `test_cross_player_interaction`, `test_parity_blades_live_start`, `test_parity_card_score`, `test_parity_cost_and_effect`, `test_parity_jidou_extra`, `test_parity_jidou`, `test_parity_rules`, `test_parity_support`, `test_performance_phase_rules`, `test_ported_generated`, `test_target_selection`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 630 | `test_answer_play_cost_choice` | HARN | yes | `test_parity_conditional_gated`, `test_parity_cost_discount`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_parser_e2e_extra`, `test_parity_per_card2`, `test_parity_per_card3`, `test_parity_recover_hand_a`, `test_parity_recover_live`, `test_parity_recover_q79`, `test_parity_recover_to_hand`, `test_parity_reveal_extra`, `test_parity_search_reorder`, `test_parity_target_player`, `test_parity_zone_source` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 640 | `test_filler_hand` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 662 | `test_fill_decks` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 684 | `test_fill_energy_deck` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 713 | `test_stage_occupant` | HARN | yes | `test_harness_self` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 768 | `test_zone_ids` | HARN | yes | `test_p1_helpers`, `test_parity_draw_delta`, `test_parity_full_round3`, `test_parity_look_per_card`, `test_parity_search_reorder`, `test_parity_sequential`, `test_parity_support` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 778 | `test_zone_count_of_id` | HARN | yes | `test_harness_self`, `test_p1_helpers`, `test_parity_conditional_gated`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_integration_rounds`, `test_parity_support`, `test_parity_wave1c` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 787 | `test_total_card_count` | HARN | yes | `test_audit_actions`, `test_audit_zones`, `test_harness_self`, `test_p1_helpers`, `test_parity_blades2`, `test_parity_bp7`, `test_parity_char2`, `test_parity_full_round2`, `test_parity_full_round3`, `test_parity_jidou_life`, `test_parity_live_total2`, `test_parity_recover_q79`, `test_parity_recover_to_hand`, `test_parity_restriction_invalidate`, `test_parity_reveal_extra`, `test_parity_rules_conditions2`, `test_parity_score_per_card2`, `test_parity_sequential`, `test_parity_wave1a`, `test_parity_wave1b`, `test_parity_wave1c` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 833 | `test_zone_has_id` | HARN | yes | `ported/self_wait_discard_niji_live_recovery_test`, `test_baton_touch`, `test_live_success_rules`, `test_mechanics`, `test_move_cards`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_card_score`, `test_parity_char2`, `test_parity_characterization_extra`, `test_parity_conditional_gated`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_deploy_move_missing`, `test_parity_deploy_multi`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_full_round`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_jidou_life`, `test_parity_jidou_watch`, `test_parity_live_total`, `test_parity_support`, `test_parity_trigger_paths`, `test_parity_zone_source`, `test_performance_phase_rules`, `test_phase_machine_rules`, `test_restriction_mechanics`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 852 | `test_move_hand_to_waitroom` | HARN | yes | `test_harness_self`, `test_p1_helpers` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 864 | `test_move_ids_to_waitroom` | HARN | yes | `test_p1_helpers` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1083 | `test_advance_to_phase_named` | HARN | yes | `test_parity_jidou_life`, `test_parity_jidou_watch_clusters` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1118 | `test_advance_report` | HARN | yes | `test_parity_recover_q79` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1176 | `test_advance_to_phase` | HARN | yes | `test_parity_ability_mod_extra`, `test_parity_blades2`, `test_parity_characterization_extra`, `test_parity_full_round`, `test_parity_int_gaps`, `test_parity_jidou_move_yell`, `test_parity_per_card3`, `test_parity_required_hearts`, `test_parity_restriction`, `test_parity_rules_conditions`, `test_parity_rules_phases`, `test_parity_rules`, `test_parity_under_member`, `test_parity_wave1a`, `test_parity_wave1b`, `test_parity_yell`, `test_parity_zone_source` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1189 | `test_deck_len` | HARN | yes | `test_harness_self`, `test_p1_helpers`, `test_parity_condition_and_effect`, `test_parity_jidou_move_yell`, `test_parity_yell`, `test_tp_zone_change_gate` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1190 | `test_hand_len` | HARN | yes | `replay`, `test_audit_triggers`, `test_harness_self`, `test_p1_helpers`, `test_parity_condition_and_effect`, `test_parity_integration_rounds`, `test_parity_jidou_move_yell`, `test_parity_yell`, `test_tp_joint_card_live_start`, `test_tp_zone_change_gate` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1211 | `test_seat_id` | HARN | yes | `test_p1_helpers` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1224 | `test_queue_n_entries` | HARN | yes | `test_p1_helpers`, `test_parity_energy_position_delta`, `test_parity_integration_rounds`, `test_parity_per_card2`, `test_parity_rules_conditions`, `test_parity_sequential` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1244 | `test_queue_entry_player_id` | HARN | yes | `test_p1_helpers` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1251 | `test_queue_entry_choice_player_id` | HARN | yes | `test_p1_helpers`, `test_parity_target_player` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1258 | `test_queue_entry_owned_by_seat` | HARN | yes | `test_p1_helpers` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1262 | `test_queue_entry_choice_owned_by_seat` | HARN | yes | `test_p1_helpers` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1274 | `test_max_distinct_names` | HARN | yes | `test_p1_helpers`, `test_parity_full_round3`, `test_parity_required_hearts` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1503 | `test_fire_trigger_on_card_seat` | HARN | yes | `test_parity_blades2`, `test_parity_deploy_multi`, `test_parity_integration_extra`, `test_parity_integration`, `test_parity_parser_e2e_extra`, `test_parity_recover_hand_a`, `test_parity_reveal_extra`, `test_parity_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1508 | `test_fire_trigger_on_card` | HARN | yes | `test_ability_effects_crash`, `test_parity_ability_mod_extra`, `test_parity_ability_mod`, `test_parity_ability_watch`, `test_parity_blade_under`, `test_parity_blades2`, `test_parity_blades_live_start`, `test_parity_bp7`, `test_parity_cce_gaps`, `test_parity_condition_and_effect`, `test_parity_deploy_recover`, `test_parity_hearts_live_start`, `test_parity_int_gaps`, `test_parity_jidou_life`, `test_parity_jidou_watch_clusters`, `test_parity_jidou_watch`, `test_parity_look_and_filter`, `test_parity_look_per_card`, `test_parity_mill_deck`, `test_parity_per_card`, `test_parity_recover_live`, `test_parity_recover_to_hand`, `test_parity_restriction_invalidate`, `test_parity_reveal_extra`, `test_parity_search_reorder`, `test_parity_small_clusters`, `test_parity_state_per_card`, `test_parity_target_player`, `test_parity_trigger_paths`, `test_parity_wait_activation`, `test_parity_wave1a`, `test_parity_wave1b`, `test_restriction_mechanics`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1513 | `test_fire_trigger_on_card_nth` | HARN | yes | `test_parity_abilitymod_conditional`, `test_parity_gain_delta`, `test_parity_mill_deck`, `test_parity_required_hearts`, `test_parity_restriction`, `test_parity_stats_pipeline`, `test_restriction_mechanics` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1518 | `test_fire_trigger_at_index` | HARN | yes | `test_parity_state_per_card` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |
| 1567 | `test_flat_verdict_failed` | HARN | **NO** | `test_cross_player_interaction`, `test_live_card_zone_movement`, `test_live_zone_slot_validation`, `test_max_distinct_names`, `test_mechanics`, `test_move_cards`, `test_p1_helpers`, `test_parity_ability_mod_extra`, `test_parity_auto_abilities`, `test_parity_blades_live_start`, `test_parity_characterization`, `test_parity_choice_routing`, `test_parity_condition_and_effect`, `test_parity_cost_and_effect`, `test_parity_cost_compound`, `test_parity_deploy_move_missing`, `test_parity_deploy_recover`, `test_parity_draw_flat`, `test_parity_fw_effects`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_jidou_extra`, `test_parity_jidou`, `test_parity_q68_cannot_live`, `test_parity_recover_to_hand`, `test_parity_resolver`, `test_parity_reveal_extra`, `test_parity_rules`, `test_parity_score_effects`, `test_parity_wait_activation`, `test_parity_zones_position`, `test_performance_phase_rules`, `test_target_selection`, `test_tp_activation_offer`, `test_tp_joint_card_live_start`, `test_tp_zone_change_gate`, `test_trigger_paths` | test-harness entry point declared in `include/test_game.h`; ships in `rb_engine` because `OBJ_LIB` includes `src/test_game.o` |

### `src/turn/live.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 42 | `rb_calc_stage_hearts` | SE | yes | `replay`, `test_parity_blade_under`, `test_parity_characterization_extra`, `test_parity_compound_per_card`, `test_parity_full_round3`, `test_parity_hearts_constants`, `test_parity_hearts_live_start`, `test_parity_required_hearts`, `test_parity_rules_conditions` | rb_stage_hearts_pipeline (src/core/stats_pipeline.c:269); live.c:30-41 says so |

### `src/turn/triggers.c`

| line | symbol | class | in the published 262? | reached from | production twin / evidence |
|---|---|---|---|---|---|
| 182 | `rb_trigger_from_token` | SE | yes | `test_audit_triggers`, `test_parity_characterization` | inline token compares at gsa.c:1920, tracking.c:221, condition.c; RB_TSTR_* literals |

