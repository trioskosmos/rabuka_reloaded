# AUDIT — queue / triggers / drain (cluster T4)

Rust modules audited: `ability/queue.rs`, `turn/trigger_scan.rs`
(cross-check `ability/effects/ability_ops.rs` only).

C files owned by this audit: `src/ability/ability_queue.c`,
`src/turn/triggers.c`. Nothing else was edited.

**This is the first systematic wiring audit of either C file.** Every function in
both files is listed below with its definition line and *every* call site in the
tree. "Defined" is not "wired"; that distinction turned out to be the dominant
fact about this code.

## Ground rules used for the numbers

* A **production** caller is a call site in `src/**` or `include/**`. A call
  site inside a `/* ... */` comment is *not* a caller — several hits in the raw
  grep are prose mentions, and three of them (`engine.c:920`,
  `choice.c:4472`, `choice.c:4474`) read exactly like calls.
* `tests/**` callers are recorded but never make a function WIRED.
* Reachability is counted over the **compile input**, `cards/abilities.json`
  (985 `unique_abilities`, matching the pinned `gen_data` / `bytecode` table
  identity 985 / 6032 / 2116 and `statistics.total_abilities = 2116`), plus
  `src/abilities_strings.bin` (726831 B, sha256 `89204df0…`). No generator was
  run.
* Baseline = this snapshot with **only my own hunks reverted**, never git HEAD.
  The two files' pre-edit hashes are recorded in the attribution sweep at the
  bottom.

## Corpus census used throughout

```
unique abilities                         985
trigger tokens   ライブ開始時  285   (269 alone + 16 as "ライブ開始時, 登場")
                 登場          281   (265 alone + 16 as the pair above)
                 常時          128
                 ライブ成功時  124
                 起動          100
                 自動           81
use_limit non-null                     133   (values 1 and 2 only)
use_limit AND 自動                       53
use_limit AND 起動                       80
use_limit AND 登場                        0
use_limit AND ライブ開始時                0
use_limit AND ライブ成功時                0
use_limit AND 常時                        0
gain_ability / gain_ability_from_source  24  effect nodes
  ability_gain_trigger distinct values: 常時 (20), ライブ成功時 (2)
  → NO gain in the corpus grants a 登場 or ライブ開始時 ability
```

Three later facts fall out of this table and are load-bearing for the verdicts:

1. **No ability in the corpus is both use-limited and 登場 / ライブ開始時 /
   ライブ成功時.** So the enqueue-time `rb_use_limit_reached` calls in the three
   scans (see D-3) can never refuse anything the corpus can produce.
2. **No gain in the corpus grants a 登場 ability.** So the gained arm the 登場
   scan used to fire is `VERIFIED-NOOP: 0 of 985` (F-2, closed).
3. **Two gains DO grant a ライブ成功時 ability**, so the live-success gained arm
   is live and correctly present — unlike the live-start and debut gained arms.

---

# 1. WIRING CENSUS — `src/ability/ability_queue.c`

64 functions. **WIRED 41 · DEAD-AND-CORRECT 19 · DEAD-BECAUSE-E3 2 ·
DEAD-AND-WRONG 2 (both still open — T4-7, T4-8).**

Three divergences were closed inside **already-WIRED** functions, so they do not
show up in this classification: `rb_drain_ability_queue`'s use-limit key (T4-3)
and `rb_trigger_debut`'s enqueue payload (T4-2, whose producer is
`rb_push_owned_entry`) were WIRED-AND-WRONG, and the gained 登場 arm (T4-1) was a
live arm inside a WIRED function. Counting those, the file's real totals are
**38 WIRED-AND-CORRECT, 3 WIRED-AND-WRONG-fixed, 21 dead, 2 dead-and-wrong open.**

| def | function | production call sites | tests | verdict |
|---:|---|---|---|---|
| 103 | `rb_queue_init` | `effects/misc.c:433` | — | WIRED |
| 110 | `entry_clear_pending_actions` (static) | `:124`(doc) `:695` `:708` `:740` `:844` `:881` `:1037` | — | WIRED |
| 121 | `rb_queue_clear` | `compound.c:81` `compound.c:373` `game_setup.c:726` | — | WIRED |
| 151 | `rb_queue_is_idle` | **ZERO** (`choice.c:4472`/`:4474` are comments; the one in-file use at `:646` is inside dead `rb_queue_resume_with_choice`) | 5 | DEAD-CORRECT — substituted by the inline `state == AWAITING_CHOICE` test at `:1417`; Rust `is_idle` is live at `abilities.rs:1793` |
| 163 | `rb_queue_is_waiting_for_choice` | **ZERO** | 2 | DEAD-CORRECT — same substitution |
| 167 | `rb_queue_state` | `:1588` | 3 | WIRED |
| 171 | `rb_queue_set_state` | `:1588`×7 in-file, `choice.c:4837`, `turn/live.c:651` `:673` | 4 | WIRED |
| 179 | `rb_queue_current_entry` | `choice_renderer.c:1183` `cost.c:96` `game_state_core.c:493` `modifiers.c:819` | 8 | WIRED |
| 184 | `rb_queue_current_entry_mut` | `:1074` `:1081` `effects/rules.c:329` `:1488` `:1724` | 6 | WIRED |
| 192 | `rb_queue_get_entry` | `choice_renderer.c:1184` | 4 | WIRED |
| 197 | `rb_queue_len` | **ZERO** | 3 | DEAD-CORRECT |
| 201 | `rb_queue_is_empty` | **ZERO** | 2 | DEAD-CORRECT |
| 205 | `rb_queue_has_pending` | **ZERO** | 1 | DEAD-CORRECT |
| 209 | `rb_queue_iter` | **ZERO** | 3 | DEAD-CORRECT |
| 220 | `rb_queue_enqueue` | `:319` `triggers.c:75` | 30 | WIRED |
| 255 | `rb_queue_push` | `:312` (from `rb_queue_push_with_trigger`, itself dead) | 9 | DEAD-CORRECT — Rust's `push` is also only reachable through the API surface |
| 286 | `queue_set_entry_trigger` (static) | `:300` `:313` | — | WIRED |
| 300 | `rb_queue_set_entry_trigger` | `triggers.c:74` `effects/ability_effects.c:247` | 4 | WIRED |
| 307 | `rb_queue_push_with_trigger` | **ZERO** (`engine.c:920` is prose) | — | DEAD-AND-WRONG → **T4-OPEN-1** (leaves `player_id` empty; the owner-scoped drain cannot see such an entry — see its own header comment at `:55-65`) |
| 317 | `rb_queue_make_entry` | `effects/ability_effects.c:242` | 4 | WIRED |
| 326 | `rb_queue_start_next` | **ZERO** | 10 | DEAD-CORRECT — the C drain walks its own index instead of advancing through this (documented at `:1400-1415`); Rust `start_next` is live at `abilities.rs:1740`/`:1809`, `actions/mod.rs:1276`/`:1322` |
| 339 | `rb_queue_complete_current` | `choice.c:4831` `choice.c:5187` | 10 | WIRED — **B1 re-verified, see §4** |
| 401 | `queue_route_targets_opponent` (static) | `:569` | — | WIRED |
| 431 | `choice_routes_to_opponent` (static) | `:569` | — | WIRED |
| 456 | `rb_queue_set_choice_route_raw` | `effects/misc.c:1276` `:1331` | — | WIRED |
| 477 | `queue_mint_dummy_entry` (static) | `:602` | — | WIRED |
| 531 | `rb_queue_ensure_current_entry` | `effects/misc.c:1924` | — | WIRED |
| 537 | `rb_queue_pause_for_choice` | `state.c:1439` `live.c:698` `look.c:367` `misc.c:1869` `choice.c:3890` `engine.c:727` | 24 | WIRED |
| 626 | `rb_queue_pause_for_auto_ability_choice` | **ZERO** | 5 | DEAD-CORRECT *body*, but see **T4-OPEN-2**: Rust has a live production caller at `abilities.rs:1736` |
| 650 | `rb_queue_resume_with_choice` | **ZERO** | 2 | DEAD-CORRECT — the state flip Rust does here is inlined at `choice.c:4797` (documented in prose at `choice.c:4483-4487`) |
| 689 | `rb_queue_clear_completed` | `game_state_abilities.c:3203` `:3236` | 6 | WIRED — mirrors Rust's single reclamation site `abilities.rs:1901` |
| 717 | `rb_queue_push_constant_context` | **ZERO** | 3 | DEAD-BECAUSE-E3 — Rust's only caller is inside `evaluate_success_zone_constant_abilities` (`modifiers.rs:2337`), which has no C twin |
| 736 | `rb_queue_pop_constant_context` | **ZERO** | 2 | DEAD-BECAUSE-E3 (`modifiers.rs:2340`) |
| 756 | `rb_queue_promote_entry` | **ZERO** | 3 | DEAD-CORRECT — Rust's own doc says it was superseded by `promote_entry_by_abs` |
| 787 | `rb_queue_promote_entry_by_abs` | **ZERO** | 4 | DEAD-CORRECT *body*, but Rust has live callers `abilities.rs:1739`, `actions/mod.rs:1275` → **T4-OPEN-2** |
| 804 | `rb_queue_set_current_entry` | **ZERO** | 4 | DEAD-CORRECT *body*, but Rust has live callers `abilities.rs:1808`, `actions/mod.rs:1321` → **T4-OPEN-2** |
| 818 | `rb_queue_has_pending_actions` | `choice.c:688` `:709` `:3697` `:3709` `:4543` `cost.c:1958` `:2174` `:2241` | 8 | WIRED — matches Rust's 12 call sites in `cost/handlers.rs`, `choice.rs`, `actions/mod.rs` |
| 856 | `rb_queue_extend_pending_actions` | `:975` `:1019` `compound.c:335` | 9 | WIRED — Rust `extend_pending_actions` |
| 876 | `rb_queue_store_pending_actions` | `:947` `:954` `:957` `effects/misc.c:1956` `effects/move.c:2591` | 7 | WIRED — Rust `store_pending_actions` |
| 909 | `queue_resolve_continuation_effects` (static) | `:952` `:973` | — | WIRED |
| 942 | `rb_queue_set_pending_actions` | `choice.c:666` `:3908` `:3982` | 9 | WIRED |
| 967 | `rb_queue_save_pending_actions` | **ZERO** | — | DEAD-AND-WRONG → **T4-OPEN-3** (Rust calls it in production at `compound/conditional_on.rs:43`) |
| 986 | `rb_queue_take_pending_actions_owned` | `:1009` `choice.c:4545` | 2 | WIRED |
| 1007 | `rb_queue_take_pending_actions` | `choice.c:637` `:979` `:985` `:1588` `:2320` `:2499` `:2684` `:4823` `:5021` `:5439` `cost.c:598` `effects/move.c:2472` | 20 | WIRED |
| 1016 | `rb_queue_repark_pending_actions` | `choice.c:4557` | — | WIRED |
| 1025 | `rb_queue_resume_pending_actions` | `choice.c:5524` | 2 | WIRED — Rust `queue.rs:522-536` |
| 1073 | `rb_queue_take_resolver` | **ZERO** | 2 | DEAD-CORRECT — C's `RbAbilityQueue` has no `Option<Box<AbilityResolver>>` to move; Rust `abilities.rs:2053`, `actions/mod.rs:1340` |
| 1080 | `rb_queue_set_resolver` | **ZERO** | 1 | DEAD-CORRECT — same reason; Rust `abilities.rs:2225`, `actions/mod.rs:1475`/`:1607` |
| 1085 | `rb_queue_has_resolver` | **ZERO** (only `:616`, inside dead `rb_queue_pause_for_auto_ability_choice`) | 1 | DEAD-CORRECT — C keeps resume state in flat `g->queue.resume_*` fields instead |
| 1098 | `rb_queue_is_entry_available` | `game_state_abilities.c:2985` | 8 | WIRED — Rust `abilities.rs:1694`/`:1803` |
| 1103 | `rb_queue_has_entry_with_id` | **ZERO** | 2 | DEAD-CORRECT |
| 1111 | `rb_queue_pending_entries` | **ZERO** | 2 | DEAD-CORRECT — Rust's `pending_entries_iter` is live at `abilities.rs:1894`, substituted inline there |
| 1123 | `rb_queue_entry_player_id` | `game_state_abilities.c:2986` | 6 | WIRED |
| 1132 | `rb_queue_dump_state` | **ZERO** | 1 | DEAD-CORRECT — debug aid |
| 1187 | `rb_use_key` (static, **new**) | `:1199` `:1209` `:1225` | — | WIRED |
| 1191 | `rb_use_limit_reached` | `game_state_abilities.c:979` `:1955` `:2325` `triggers.c:181` `:255` `:334` `:381` `:554` `engine.c:1301` | 15 | WIRED |
| 1201 | `rb_record_use` | `game_state_abilities.c:55` `engine.c:1228` | 20 | WIRED |
| 1217 | `rb_use_count` | `game_state_abilities.c:2427` `engine.c:1298` | 9 | WIRED |
| 1229 | `rb_owner_of_card` | `condition.c:887` `effects/ability_effects.c:417` `turn/live.c:617` `triggers.c:560` | 12 | WIRED |
| 1290 | `rb_drain_seen_add` (static) | `:1383` `:1390` | — | WIRED |
| 1313 | `rb_drain_keep_alive` (static) | `:1510` `:1590` (`engine.c:1113` is prose) | — | WIRED |
| 1344 | `drain_record_ability_use` (static) | `:1469` `:1621` | — | WIRED |
| 1353 | `rb_drain_ability_queue` | 65 sites incl. `engine.c:414` `:469` `engine.c:1186` `turn/live.c` `turn/phase.c` `choice.c` `game_state_abilities.c:2670` | 40 | WIRED |
| 1637 | `rb_choice_set_route` | `effects/misc.c` `move.c` `move_missing.c` `state.c` `choice.c:4835` `engine.c:855` | — | WIRED |

## WIRING CENSUS — `src/turn/triggers.c`

35 functions. **WIRED 34 (3 of them WIRED-TRACE-ONLY — reachability helpers whose
only call sites are inside `fprintf(stderr, …)`) · DEAD 1
(`rb_trigger_to_texticon`, a UI-only twin) · DEAD-AND-WRONG 0.**

Like the queue file, the one divergence closed here (T4-1, the gained 登場 arm)
was a live arm inside a WIRED function — `rb_trigger_debut` — not a dead
function, so the file's real totals are **29 WIRED-AND-CORRECT / 1
WIRED-AND-WRONG-fixed.** `queue_gained_trigger` and `queue_gained_trigger_seen`
were deleted as a consequence and are absent from the table.

| def | function | production call sites | tests | verdict |
|---:|---|---|---|---|
| 32 | `effect_condition_baton_touch_trigger` (static) | `:236` | — | WIRED — Rust `:99-109` |
| 50 | `rb_trigger_is` | `effects/ability_effects.c:353` `game_state_abilities.c:1074` `engine.c:476` `test_game.c:589` `ability_queue.c:1388`/`:1395` | — | WIRED |
| 96 | `rb_push_owned_entry` (static) | `:239` `:389` `:434` `:607` `:613` | — | WIRED |
| 150 | `rb_trigger_seen_add` (static) | `:451` `:495` `:546` `:571` | — | WIRED |
| 165 | `rb_trigger_from_token` | internal only (was `queue_gained_trigger`, now gone) | — | WIRED (exported, used by tests) |
| 227 | `rb_trigger_debut` | `ability.c:424` `move.c:1230` | 8 | WIRED — Rust `trigger_debut_abilities`, both call sites `phases.rs:1076`/`:1317` |
| 277 | `rb_trigger_live_start` | `turn/phase.c:174` | 6 | WIRED — Rust `phases.rs:245`/`:246` |
| 425 | `rb_should_trigger_live_success` | `turn/live.c:717` `:1478` | 4 | WIRED, **but D-1** |
| 473 | `rb_trigger_live_success` | `turn/live.c:545` `:1485` | 5 | WIRED — Rust `live.rs:1211`/`:1226`/`:1241` |
| 576 | `rb_drain_live_success_choices` | `turn/live.c:767` | — | WIRED |
| 590 | `mirror_position` (static) | `:759` | — | WIRED |
| 597 | `card_at_position` (static) | `:763` | — | WIRED |
| 607 | `effect_is_live_end` (static) | `:743` | — | WIRED |
| 620 | `effect_position_matches` (static) | `:1389` `:1422` | — | WIRED |
| 643 | `card_on_stage` (static) | `:810` `:1142` | — | WIRED |
| 650 | `cextra` (static) | `:673` `:834` `:881` `:1236` `:1244` | — | WIRED |
| 664 | `cextra_first_entry` (static) | `:1014` | — | WIRED |
| 687 | `const_heart_color_list` (static) | `:917` `:1043` | — | WIRED |
| 711 | `ceff_per_unit` (static) | `:835` `:881` | — | WIRED |
| 726 | `constant_gain_per_unit_units` (static) | `:850` `:881` | — | WIRED |
| 734 | `apply_constant_effect` (static) | `:1228` `:1410` `:1428` | — | WIRED |
| 1236 | `rb_fire_debut` | `engine.c:590` `:1062` `test_game.c:236` | — | WIRED |
| 1246 | `remove_prohibition_text` (static) | `:1297` | — | WIRED |
| 1264 | `rb_check_expired_effects` | `ability.c:508` `:509` `test_game.c:237` | — | WIRED |
| 1326 | `rb_recalc_constants` | 30 sites | — | WIRED |
| 1537 | `effect_suppresses_trigger` (static) | `:1546` `:1566` | — | WIRED |
| 1550 | `rb_is_trigger_suppressed` | `:190` `:315` | — | WIRED — Rust `trigger_scan.rs:195`/`:307` |
| 1577 | `effect_activation_position_any` (static) | `:241` `:418` `:503` `:524` | — | WIRED |
| 1605 | `rb_trigger_auto_abilities_for_player` | `state.c:637` `:638` `:647` `:648` `:847` `:848` `turn/live.c:1911` `:1912` | — | WIRED — Rust `trigger_scan.rs:351`, same 8 caller sites |
| 1613 | `rb_canonical_trigger` | `:497` `:508` `:509` `:566` `:567` — **all inside `fprintf(stderr, …)`** | — | WIRED-TRACE-ONLY |
| 1630 | `rb_trigger_zone_id` | same four `fprintf`s | — | WIRED-TRACE-ONLY |
| 1634 | `rb_trigger_zone_label` | same four `fprintf`s | — | WIRED-TRACE-ONLY |
| 1641 | `rb_trigger_to_texticon` | **ZERO** | — | DEAD — Rust `triggers.rs:139` is called from `game/display.rs:1599`, a UI path with no C twin |
| 1656 | `kind_of_token` (static) | `:1682` | — | WIRED |
| 1668 | `rb_parse_triggers` | `core/card.c:623` | — | WIRED |

**`[LIVE_START_SCAN]` fprintfs: NOT REMOVED.** They are unconditional and mirror
Rust's `log::trace!` family at `trigger_scan.rs:273-281`, which is also
unconditional. The one Rust trace that *is* gated (`trigger_scan.rs:188`,
inside an `ABILITY_DEBUG.load()`) has no C counterpart because the C drain's
first-push early-out at `ability_queue.c:1388` already collapses that case.
These are diagnostics, not debug gates.

---

# 2. RUST ENTRY POINTS WHOSE C TWIN IS DEFINED BUT NOT CALLED

This is the list that matters. Every row is a `pub fn` in the two Rust modules
whose C twin exists, compiles, has tests exercising it — and is never reached
from the C engine path.

| Rust `pub fn` | Rust production callers | C twin | C production callers | consequence |
|---|---|---|---|---|
| `queue.rs:416 pause_for_auto_ability_choice` | `abilities.rs:1736` (Rule 9.5.3 auto-ability ordering prompt) | `rb_queue_pause_for_auto_ability_choice` (`ability_queue.c:626`) | **ZERO** | the 「複数の自動能力が同時に発動しました」 ordering prompt does not exist in C |
| `queue.rs:592 promote_entry_by_abs` | `abilities.rs:1739`, `actions/mod.rs:1275` | `rb_queue_promote_entry_by_abs` (`:787`) | **ZERO** | the first auto-ability of the batch is not hoisted |
| `queue.rs:607 set_current_entry` | `abilities.rs:1808`, `actions/mod.rs:1321` | `rb_queue_set_current_entry` (`:804`) | **ZERO** | the §9.5.3.2→§9.5.3.1 depth-first loopback cannot target an index |
| `queue.rs:291 start_next` | `abilities.rs:1740`/`:1809`, `actions/mod.rs:1276`/`:1322` | `rb_queue_start_next` (`:326`) | **ZERO** | substituted: the C drain walks its own index (`:1400-1415`). Structurally fine, but it means **`g->queue.cur` is the C's single source of ordering truth and no caller may rely on `start_next`** |
| `queue.rs:539 save_pending_actions` | **`ability/compound/conditional_on.rs:43`** | `rb_queue_save_pending_actions` (`:967`) | **ZERO** | **T4-OPEN-3.** A `conditional_on` whose selected branch finishes must leave ONE retained action on the entry; C leaves none |
| `queue.rs:466 / :510 push_/pop_constant_context` | `modifiers.rs:2337`/`:2340`, inside `evaluate_success_zone_constant_abilities` | `rb_queue_push_constant_context` (`:717`) / `rb_queue_pop_constant_context` (`:736`) | **ZERO** | downstream of E3 — dead until the success-zone scan lands |
| `queue.rs:614/:619/:626 take_/set_/has_resolver` | `abilities.rs:2052`/`:2053`/`:2225`, `actions/mod.rs:1340`/`:1475`/`:1607` | `rb_queue_take_resolver` (`:1073`) / `set` (`:1080`) / `has` (`:1085`) | **ZERO** | C has no `Option<Box<AbilityResolver>>`; resume state is flat `g->queue.resume_*`. Not fixable without a type change. DEAD-CORRECT |
| `queue.rs:522 pending_entries_iter` | `abilities.rs:1894` | `rb_queue_pending_entries` (`:1111`) | **ZERO** | inlined at that Rust site. DEAD-CORRECT |
| `triggers.rs:139 trigger_to_texticon` | `game/display.rs:1599` | `rb_trigger_to_texticon` (`triggers.c:1641`) | **ZERO** | UI-only. DEAD |

**Everything else in `queue.rs` and `trigger_scan.rs` is called from the same
place in the control flow as its Rust original.** In particular all five Rust
`pub fn`s in `trigger_scan.rs` — `trigger_debut_abilities`,
`is_trigger_suppressed`, `trigger_live_start_abilities`,
`trigger_auto_abilities_for_player`, `trigger_live_success_abilities` — are
wired at their real Rust call sites, and `trigger_debut_abilities`' two callers
(`phases.rs:1076`, `:1317`) both pass `Some(card_id)`, which is why the C's
`card_id == None` fallback arm (Rust's `card_no` re-scan, `:59-64`) is
legitimately unreachable and needs no port.

---

# 3. QUEUE LIFECYCLE vs RUST

| stage | Rust | C | verdict |
|---|---|---|---|
| enqueue | `AbilityQueue::enqueue` (`queue.rs:344-383`) appends at `n_entries`; nothing is advanced, nothing recorded | `rb_queue_enqueue` (`ability_queue.c:220-254`) identical, including the `memset` of the whole entry | MATCH |
| dedup | none at enqueue — `enqueue_printed_ability` returns `Ok(())` and the caller ignores it (`abilities.rs:1284-1286`) | same; the ignore-check is in `rb_drain_ability_queue:1400` (`pending` key set) and in `pca_resolve_entry` | MATCH |
| `use_limit` gate | **not at enqueue.** `trigger_auto_ability` → `enqueue_printed_ability` has no gate at all. Gates live in the TAS scan (`abilities.rs:971 ability_has_remaining_uses`) and at resolution (`gates.rs:93-111 use_limit_gate`, wired through `EFFECT_GATES`) | no record at enqueue (`rb_push_owned_entry` documents why), the TAS scan gates at `game_state_abilities.c:979`, resolution-time records at `ability_queue.c:1469`/`:1621` and `game_state_abilities.c:2756`/`:2840`/`:2888`/`:2946`/`:2959` | MATCH, with D-3 below |
| completion / advance | `complete_current` (`queue.rs:437-451`): mark completed, `state = Idle`, `current_index += 1`. Nothing else | `rb_queue_complete_current` (`:339-366`) byte-for-byte the same three steps | MATCH |
| park & resume | `pause_for_choice` (`:349-381`) / `resume_pending_actions` (`:522-536`) / `resume_with_choice` (`:422-434`) | `rb_queue_pause_for_choice` (`:537`) WIRED, `rb_queue_resume_pending_actions` (`:1025`) WIRED, `rb_queue_resume_with_choice` (`:650`) DEAD but its state flip is inlined at `choice.c:4797` | MATCH |
| clear-on-overflow | `clear_completed` (`queue.rs:458-472`), and the queue is rebuilt by `retain` (`queue.rs:449-458`) | `rb_queue_clear_completed` (`:689-714`) identical, including the `_pos > _i` rebuild | MATCH |
| park-and-resume owner stamp | entries carry `player_id`; the drain is owner-scoped | `rb_push_owned_entry` stamps `p1`/`p2`; `game_state_abilities.c:2670-2680` clears the queue when no owner matches | MATCH |

## B1 re-verification — `choice.c` bare `cur++`

The prior finding named `choice.c:4907`. Lines have moved; the two sites are
now `choice.c:4831` and `choice.c:5187`. **The fix landed and is correct.**

```c
/* choice.c:5187, guarded by the rb_has_pending_choice test at :5189's twin */
rb_queue_complete_current(g);
```

The guard is real — the surrounding comment (`choice.c:5179-5186`) quotes
Rust's own refusal: `"skipping complete_current — pending choice exists"`
(`turn/actions/mod.rs:1552-1557`), which fires when re-running the parked
effect raises another prompt. The other site, `choice.c:4831`, is inside the
`success_replacement` branch and is guarded by the same `if
(g->queue.cur >= 0 && g->queue.cur < g->queue.n_entries)` bounds check, then
sets `RB_QUEUE_IDLE` and drains — i.e. the same three steps Rust takes. Neither
site calls `rb_queue_clear_completed`, and that is deliberate and documented at
`choice.c:5164-5177` (running it here was measured to cost +7 on
`parity_cce_gaps`).

---

# 4. TRIGGER SCAN ORDER

Verified against Rust line by line. **All three scans match.**

### 登場 (`rb_trigger_debut`, `triggers.c:227`)

Rust `trigger_scan.rs:42-47` builds `areas = [LeftSide, Center, RightSide]`
and `:48-57` skips any area whose card is not the debuting `card_id`, so with
`card_id = Some(..)` — the only shape either caller uses — at most one area
survives. The C derives `area` from a stage scan and `break`s at the first
match. Equivalent. Gate order `:90` → `:99` → `:121` (position, baton-touch,
use-limit) matches the C's `:239` → `:243` → `:249`. Invalidation at `:86-88`
precedes the trigger match in both.

### ライブ開始時 (`rb_trigger_live_start`, `triggers.c:277`)

Rust `:232-238`:
```rust
let live_cards = player.live_card_zone.cards.iter().map(...);
let stage_cards = player.stage.stage.iter().enumerate().map(...);
for (card_id, position) in live_cards.chain(stage_cards) {
```
**live-card zone FIRST, then stage.** C: `for (int i = 0; i < g->p[pl].live.n; i++)`
then `for (int s = 0; s < RB_STAGE_SIZE; s++)`. **MATCH**, including that both
pass one `player_id_clone` for the whole chain.

Rust `:282-286` inserts into `seen` and `:288-315` fires, with `check_effect_position`
inside the enqueue closure — i.e. **position gate, then `seen`, then enqueue**.
C: `rb_trigger_seen_add(...) && !rb_use_limit_reached(...)` after
`rb_check_effect_position`. **MATCH.**

Rust `:345-346` passes `None` for `moved`; C passes `NULL, 0`. **MATCH** (and
that matters — see F-1, where the 登場 scan was passing `NULL` too and should
not).

### ライブ成功時 (`rb_trigger_live_success`, `triggers.c:473`)

Rust `:386-395`:
```rust
let stage_cards = ...;
let live_cards = ...;
for (card_id, position) in stage_cards.chain(live_cards) {
```
**stage FIRST, then live zone** — the opposite chain from ライブ開始時. C:
`for (int zone = 0; zone < 2; zone++)` with `zone == 0` bound to
`g->p[pl].stage`. **MATCH.** Getting this backwards would hand each ability a
different board state, so it is worth stating explicitly.

Rust `:396-405` (suppression) → `:407` (`should_trigger_live_success`) →
`:411-423` (position gate) → `:424-428` (`seen` + enqueue) → `:462-514` (gained
loop, sharing the same `seen`) → `:516-527` (fire). C reproduces that order and
defers both arms through a `pending[]` array so the enqueue loop cannot mutate
the scan. **MATCH.**

---

# 5. DIVERGENCES — C returns a different value than Rust

Ordered by how much they cost.

### D-1 `rb_should_trigger_live_success` returns a cached flag; Rust recomputes

Rust `abilities.rs:2870-2915` (`should_trigger_live_success`) does three
things:

1. `if self.current_phase != Phase::LiveVictoryDetermination { return false; }`
   — **the C has no phase term at all.**
2. `if player.live_card_zone.cards.is_empty() { return false; }` — C has this
   (`g->p[pl].live.n == 0`).
3. it recomputes `stage_hearts` and **re-checks every live card's
   `need_heart`**, so the answer reflects the heart requirements *at this
   moment*.

The C returns the recorded `g->live_success[pl]`. Divergence in two windows:

* **phase**: any caller that runs the ライブ成功時 scan outside
  `LiveVictoryDetermination` gets `1` from C where Rust returns `0`. That is
  exactly what most of the suites do when they call `rb_trigger_live_success`
  directly, which is why this divergence is invisible to them.
* **heart requirement**: any change to the live card's `need_heart` after
  `g->live_success[pl]` was recorded — a `modify_required_hearts` effect, a
  `need_heart` modifier that expired or was restored — leaves C on the stale
  answer. Rust re-derives it. This is the same hole as checklist E3/E4, and it
  is why `rb_trigger_live_success` does **not** call
  `evaluate_success_zone_constant_abilities` / `restore_performance_need_heart_modifiers`
  the way `trigger_scan.rs:360` and `:365` do.

Do not "fix" the phase term: 20+ suites call the scan directly with
`live_success[pl]` preset and rely on the cached answer. Closing this means
fixing the *callers* (a suite-side change owned by cluster J) or landing E3/E4
first. Left open.

### D-2 the 登場 scan fires a gained 登場 ability Rust never fires — **CLOSED**

`rb_trigger_debut` used to end with `queued += queue_gained_trigger(g, pl,
card_id, "登場")`. Rust's `trigger_debut_abilities` (`trigger_scan.rs:78-139`)
iterates `card.abilities` and nothing else — no `gained_card_abilities` loop —
so `trigger_auto_ability_by_index_refs` is only ever reached with a printed
index. This is the 登場 twin of the ライブ開始時 divergence already removed at
`triggers.c:403-412`.

**Reachability: `VERIFIED-NOOP: 0 of 985`** — of the 24 `gain_ability` /
`gain_ability_from_source` nodes, the distinct `ability_gain_trigger` values
are 常時 (20) and ライブ成功時 (2). Never 登場. So the removal is
measurement-neutral by construction, and it stops C diverging the day the
parser does emit one.

Removed, along with the now-dead `queue_gained_trigger` /
`queue_gained_trigger_seen` helpers (the latter's only other caller had already
been deleted from the ライブ開始時 scan). Replaced by a comment block at
`triggers.c:206-226` recording the Rust citation and the census.

**Tests:** `tests/test_parity_queue_resume.c` —
`gained_debut_ability_is_never_fired_by_the_debut_scan`.

### D-3 enqueue-time `use_limit` gate that Rust does not have — `VERIFIED-NOOP: 0 of 985`

`rb_trigger_debut` (`:249`), `rb_trigger_live_start` (`:334`, `:381`) and
`rb_trigger_live_success` (`:554`) all refuse to enqueue when
`rb_use_limit_reached` fires. Rust's `trigger_scan.rs` never calls
`ability_has_remaining_uses` — that accessor has exactly three call sites in
the whole engine: the TAS scan (`abilities.rs:971`), the bot
(`bot/observation.rs:146`) and `use_limit_gate` at resolution
(`gates.rs:102-103`).

**Reachability: `VERIFIED-NOOP: 0 of 985`** — the cross-tab above shows
`use_limit AND {登場, ライブ開始時, ライブ成功時} = 0` for every one of the
three. Since the C sentinel is `limit = 99` for `use_limit < 0`, the gate can
only fire after 99 records of one key in one turn.

The gates are left in place rather than deleted: they are correct-in-principle,
they cost one integer compare, and removing them would collide with the
pending work that may land E3/E4 and start re-populating the affected sets.

### D-4 `use_limit` key truncated to a FOUR-BIT ability index — **CLOSED**

`rb_record_use` / `rb_use_limit_reached` / `rb_use_count` all keyed on
`(card_id << 4) | (ability_idx & 0xF)`. Rust keys on the full
`(i16, usize, u32)` triple (`abilities.rs:146-151`).

`RB_GAINED_ABILITY_INDEX_BASE` is `10000` (`rabuka.h:3028`) and
`10000 & 0xF == 0`, so a **gained** ability at index `10000 + g` hashed to the
same slot as the **printed** ability `#g` on the same card. Concretely, C could
refuse a printed 「{{ターン1回}}」 自動 ability because an unrelated gained
ability had resolved, and `rb_use_count` could report a non-zero count for an
ability that never resolved.

**Reachability today: `VERIFIED-NOOP: 0 of 985`** — the collision needs a gained
ability carrying 自動 or 起動, and the only two families that carry a
`use_limit` at all are 自動 (53) and 起動 (80); no gain in the corpus grants
either. But the gained arm *is* wired (two ライブ成功時 grants), and
`queue_gained_trigger_seen` called `rb_use_limit_reached` with exactly the
aliased index — a latent bug in a live path, which is the case the mandate says
to fix rather than document.

Fixed by `rb_use_key()` (`ability_queue.c:1187-1189`), used by all three
functions. Provably behaviour-neutral on the corpus: with no collisions in
existence, widening changes which slot a pair lands in, not which pairs compare
equal. `card_id < 2^15` (2639 cards) and the index is masked to 16 bits, so the
packed key stays inside a positive `int`; `RB_USE_TRACK` is untouched.

**Tests:** `tests/test_parity_queue_resume.c` —
`gained_and_printed_use_limit_keys_do_not_alias`.

### D-5 the 登場 enqueue carried no `moved` snapshot; Rust carries one — **CLOSED**

Rust `trigger_scan.rs:148`:
```rust
let moved_snapshot = game_state.recently_moved_cards.clone();
```
taken once before the loop and cloned into **every** enqueued 登場 ability
(`:156`, `:166`, `:180`, `:190`, `:205`, `:215`, `:221`). C's
`rb_push_owned_entry` passed `NULL, 0` for all three scans, so
`rb_queue_set_entry_trigger` wrote `n_trigger_moved_cards = 0`.

This is load-bearing, not decoration. `rb_entry_trigger_moved_cards`
(`game_state_abilities.c:1614-1624`) is the **first** source
`moved_card_pool` / `those_cards` conditions consult (`condition.c:1194-1200`,
`:3203-3210`) and it **prefers** the snapshot over the live global — and
`rb_drain_ability_queue` zeroes `g->n_recently_moved` at `ability_queue.c:1420`
before the cost runs. So by the time a 登場 effect asks "those cards", the live
global is already empty and the entry snapshot is the only surviving evidence.
With it empty, C resolved the question against an **empty set** where Rust sees
the batch.

**Reachability: n > 0.** Over the 281 登場 abilities:

* 3 have `source: "those_cards"` on their effect,
* 22 have `type: "movement_condition"` (20 as a top-level condition type).

The other two scans pass `None` in Rust (`:345-346`, `:525-526`) and still pass
`NULL, 0` in C — that part was already right. The 自動 / TAS scan passes the
batch and was already right too (`game_state_abilities.c:1694-1700`). Only the
登場 scan was wrong.

Fixed by threading `moved` / `n_moved` through `rb_push_owned_entry` and
passing `g->recently_moved` / `g->n_recently_moved` from `rb_trigger_debut`.

**Tests:** `tests/test_parity_queue_resume.c` —
`debut_entry_carries_the_enqueue_time_movement_snapshot` and
`live_start_entry_carries_no_movement_snapshot`.

### D-6 `rb_trigger_live_start` drains inside the scan; Rust does not

Rust `phases.rs:245-253`:
```
trigger_live_start_abilities(first)    // enqueue only
trigger_live_start_abilities(second)   // enqueue only
process_pending_auto_abilities(first)  // owner-scoped drain
if has_pending_choice() return
process_pending_auto_abilities(second)
```
Rust's scan never drains. C's `rb_trigger_live_start` calls
`rb_drain_ability_queue(g)` at `triggers.c:441-445` whenever it queued
anything — an un-owner-scoped drain from inside the scan, so the first seat's
entries resolve before the second seat's are even enqueued.

In practice the ordering comes out the same (p1 then p2) *because* the queue is
empty when `phases.rs:245` runs, which is why nothing measures. It is a real
divergence in the general case and a trap for anyone who later leaves entries
queued across that boundary.

Not changed: the drain is load-bearing for the phase machinery around it and
moving it to the caller means editing `turn/phase.c` (cluster J). Recorded as
**T4-OPEN-4**.

### D-7 `rb_is_trigger_suppressed` also consults `g->prohibition[]`

Rust `trigger_scan.rs:162-176` checks exactly three suppressors
(`trigger_moved_activation`, `on_activation`, `suppress_target`) across
`stage.stage` then `live_card_zone.cards`. C adds a fourth source — a
`"suppress:%d:%s"` entry in `g->prohibition[]` — that has no Rust counterpart.
Not reachable from the corpus today (no `suppress_ability_trigger` grant
targets the three scan triggers), documented rather than removed because
`prohibition[]` is a shared table this file does not own.

---

# 8. HARNESS DEFECTS FOUND WHILE MEASURING (not engine bugs, but they hide results)

### H-1 `test_audit_triggers.c` failures are invisible to the documented protocol

The suite prints

```
FAIL [A2.debut_position_left_positive] A2_debut_activation_position: ...
```

**No colon after `FAIL`.** A verifier that greps `^FAIL:` — which is the protocol
AGENTS.md documents — sees this suite as `FAIL_OUT=0 FAIL_ERR=0`. It was in fact
running **4 ok / 3 FAILED** at baseline with 5 real `FAIL` lines on stderr. The
suite's own `return n_failed || n_crashed` is its only signal, and exit code is
explicitly not a signal. Any corpus count that includes this suite understates it
by the number of rows it hides.

**Consequence for this audit's own before/after:** the "978 → 978" figure does
**not** cover `audit_triggers`. Both runs were checked separately with a `^FAIL`
grep, and both sets are identical (5 lines, the same 5 rows).

### H-2 three of those hidden failures assert something Rust does not do

`A2.debut_position_left_positive`, `A3.debut_plain_positive` and both rows of
`A4b.live_success_seen` all fail with the same shape:

```
PAIRED POSITIVE: the same ability MUST fire from the LEFT area (queued=1 use_count=0)
```

`queued` is **correct** (1). `use_count` is **0**, and that is the faithful
value: `trigger_debut_abilities` → `trigger_auto_ability` →
`enqueue_printed_ability` (`abilities.rs:1282-1326`) ends in a bare
`self.ability_queue.enqueue(entry)` and constructs `use_limit_recorded: false`.
The scan records nothing; `record_ability_use` is reached only from eleven
resolution-time call sites (`resolver.rs:1180`/`:1230`/`:1390`, `gates.rs:160`/
`:308`/`:318`/`:331`, `choice.rs:3479`/`:3663`, `actions/mod.rs:1529`).

So these rows' `rb_use_count(...) == 1` assertions are **stale expectations from
an older C shape that recorded the use at enqueue**, and the C they are asserting
about no longer does that. The file's own header instructs "Do not weaken an
assertion; fix the engine or re-classify the row" — this is a re-classification:
the engine is right and the row is wrong. **Reported, not edited** — weakening or
deleting an assertion is not this audit's call.
**BLOCKED-ON: `tests/test_audit_triggers.c` — cluster J's file.**

### H-3 `test_parity_queue_resume.c` aborts mid-file

SIGABRT after `test_optional_look_select_skip_terminates`'s last CHECK
(`:640`), i.e. at `free_optional_look_select(&parent)` (`:641`) or inside
`test_select_routes_card_to_hand` (`:644-663`) before its first CHECK. The last
five tests therefore never run, including the two the file's own header
(`:15-19`) advertises as regression guards
(`test_skip_chain_does_not_grow_the_queue`,
`test_baton_touch_skip_chain_terminates`), and `rb_unload()` is never reached.
It also emits **zero stdout lines**, because block-buffered output is discarded
on abort — so a `^FAIL:`-only verifier sees exactly one failure and no sign that
half the file is dark.

Present in the **baseline**, before any edit here. **BLOCKED-ON:
`src/ability/effects/look.c` for the ownership of `look_action` /
`select_action` across `rb_execute_effect_ex`, plus `tests/**`.**

---

# 6. STILL OPEN — cluster T4 entries for `PARITY_CHECKLIST.md`

| id | file:line | exact change needed |
|---|---|---|
| T4-OPEN-1 | `ability_queue.c:307` | `rb_queue_push_with_trigger` has zero production callers and builds an entry with `player_id` left empty, so any future caller produces an entry the owner-scoped drain (`game_state_abilities.c:2670-2680`) cannot see and silently drops. Either delete it or make it stamp the owner. Cheapest correct action: **delete it** — no caller, and `rb_queue_enqueue` + `rb_queue_set_entry_trigger` already covers every real producer. |
| T4-OPEN-2 | `game_state_abilities.c:~1660-1810` (Rust `abilities.rs:1701-1821`) | Rule 9.5.3 needs all three of `rb_queue_pause_for_auto_ability_choice` (`ability_queue.c:626`), `rb_queue_promote_entry_by_abs` (`:787`) and `rb_queue_set_current_entry` (`:804`), which are defined, tested and unreachable. In `rb_process_player_abilities`, when two available entries share an owner, build `Choice::SelectAutoAbility { player_id, options, description, description_en, description_ja }`, call `rb_queue_pause_for_auto_ability_choice` and break; otherwise `rb_queue_promote_entry_by_abs(idx)`. In the depth-first loopback, `rb_queue_set_current_entry(new_idx)`. **BLOCKED-ON: game_state_abilities.c — not mine.** |
| T4-OPEN-3 | `ability_queue.c:967` + `compound.c` | `rb_queue_save_pending_actions` is the twin of Rust's `queue.rs:539 save_pending_actions`, which has a **production** caller at `ability/compound/conditional_on.rs:43` (`gs.ability_queue.save_pending_actions(vec![finish])`). Find the C of `conditional_on_result` / `conditional_alternative` and APPEND the finished action with `rb_queue_save_pending_actions` there. **BLOCKED-ON: compound.c (Rust twin `ability/compound/conditional_on.rs`) is not mine — needs the owner of compound.c.** The queue half is correct and needs no change. |
| T4-OPEN-4 | `triggers.c:441` + `turn/phase.c:174-175` | Remove the `rb_drain_ability_queue(g)` from inside `rb_trigger_live_start` and drain from the phase instead, matching `phases.rs:245-253` (enqueue both seats, then `process_pending_auto_abilities` per seat). **BLOCKED-ON: turn/phase.c — not mine.** Body of `rb_trigger_live_start` is otherwise byte-faithful to the Rust scan. |
| T4-OPEN-5 | `game_state_abilities.c` / `modifiers.c` | `rb_queue_push_constant_context` (`:717`) and `rb_queue_pop_constant_context` (`:736`) are dead only because E3 (`evaluate_success_zone_constant_abilities`, Rust `modifiers.rs:2330-2345`) has no C twin. They must be called from that new function, in that order, around `rb_recalc_constants`. Same for E4 (`restore_performance_need_heart_modifiers`, Rust `modifiers.rs:2155`). |
| T4-OPEN-6 | `turn/live.c` | D-1: the phase term and the `need_heart` re-check. Only closable once E3/E4 land, or by a suite-side change converting the direct `rb_trigger_live_success` callers to set the phase properly. |

---

# 7. ATTRIBUTION

Both files were quiescent across a 20 s settle before editing
(`ability_queue.c` `288144C2…`, `triggers.c` `514E29B8…`, identical on two
reads), and the snapshot matched the live tree on all 216 `.c`/`.h` files
(`same=216 diff=0 missing=0`) before any edit.

Snapshot: `C:/Users/trios/AppData/Local/Temp/kilo/qt/`. Builds were run there
only, with `make data` frozen as
`CARDS_BIN=src/cards.bin STRINGS_BIN=src/abilities_strings.bin` so the guard
self-verifies against the pinned bytes instead of installing from
`cards/build/` (which is 726649 B and stale). The blob was re-checked before
and after every build: **726831 B, sha256 `89204DF0BBDE08A0…` unchanged.** No
generator ran. No `.o` was written into the shared tree (`-fsyntax-only` only).

| file | baseline (my hunks reverted) | after my edits |
|---|---|---|
| `src/ability/ability_queue.c` | `288144C25301B0A2FF99A3F4CD8E114F9D0904C89B07AC96C3DCD860B04CD5CF` | `934614AB96829AE9…` |
| `src/turn/triggers.c` | `514E29B850974EBB5BF75935ED956BA29D85C3D5C040AB070B1F3A87F20F902A` | `9A376868F135F971…` |
| `tests/test_audit_triggers.c` | `484C46385FDAE695…` (unchanged) | `484C46385FDAE695…` — added rows A8/A9/A10 |
| `tests/test_parity_queue_resume.c` | `7E73AAA4BFD2F2E5…` | `7E73AAA4BFD2F2E5…` — **byte-identical, restored** |
| `include/rabuka.h` | `9497B0EBB200A3F2…` | unchanged (not mine) |
| `src/ability/choice.c` | `BFC99DD749C3F4BA…` | unchanged (cluster B) |
| `src/core/game_state_abilities.c` | `C1A46CEDEF4B8C03…` | unchanged (cluster G+H) |

The data blob was re-checked after every build: **726831 B, sha256
`89204DF0BBDE08A02A7F978A…`, unchanged.**

## Measurement record, including one incident worth writing down

| run | what was linked | `^FAIL:` lines across the 33-suite corpus | audit_triggers non-colon `FAIL` |
|---|---|---|---|
| baseline | pre-edit objects | 978 | 5 |
| attempt 1 | **pre-edit objects (stale)** | 978 — identical, and *meaningless* | 6 |
| attempt 2 | rebuilt `ability_queue.o` + `triggers.o` | **978 — identical** | 5 |
| attempt 3 | `audit_triggers` with A8/A9/A10 added | — | 5 (all pre-existing rows) |

**The attempt-1 run is a trap worth documenting.** The suite runner compiles only
the *test* file and links pre-existing `.o` files. After editing the two `.c`
files it produced a byte-identical result — which looked like a clean
measurement and was in fact a *second baseline*. The giveaway was that the three
new parity rows also failed when run standalone against the same stale objects.
The correct procedure is: after editing a behaviour file, **rebuild its object
before the corpus run**, and treat an unexpectedly perfect zero-delta as evidence
of a stale link rather than of neutrality. Only attempt 2/3 are measurements.

Final verdict: **978 → 978, zero net movement, in either direction**, across all
33 suites, with the corrected objects. That is the expected result and it is
consistent with the census: T4-1 is 0-of-985, T4-3 has no collisions to remove,
and T4-2 changes a field that is written but only read by `those_cards` /
`movement_condition` conditions, of which the 33 suites happen to exercise none.

## Tests added

All in `tests/test_audit_triggers.c`, which is fork-per-test — so a fault in one
row is tallied, not fatal. Every row cites the Rust line it asserts and pairs
each negative with a positive that differs by exactly the factor under test.

| row | asserts | Rust line |
|---|---|---|
| `A8.debut_carries_moved_snapshot` | the 登場 entry carries the enqueue-time `recently_moved_cards` | `trigger_scan.rs:148`, `:156`; `abilities.rs:2377-2384` |
| `A8b.live_start_carries_no_moved_snapshot` | a ライブ開始時 entry carries **none** (the asymmetry must not leak the other way) | `trigger_scan.rs:345-346` |
| `A9.gained_debut_never_fires` | the 登場 scan enqueues **zero** gained ability indices, and still enqueues exactly the one printed one | `trigger_scan.rs:78-139` |
| `A9b.gained_live_success_still_fires` | the ライブ成功時 gained arm **does** fire — the paired positive that stops A9 being read as "gained abilities never fire" | `trigger_scan.rs:462-514` |
| `A10.gained_and_printed_use_keys_are_distinct` | spent uses on `(card, 3)` and `(card, 10003)` are independent, in **both** directions, and still turn-scoped | `abilities.rs:146-151`, `:216-231`; `gates.rs:93-111` |

`A8`, `A9` (both rows) and `A10` pass. The 4 rows still failing in this suite
(`A2`, `A3`, `A4b` ×2, `A7`) are **pre-existing and identical to the baseline**;
see §8.

`tests/test_parity_queue_resume.c` was briefly given four equivalent cases and
then **restored byte-for-byte** (`7E73AAA4BFD2F2E5…`): the suite aborts mid-file
(see §8) and would never have executed them. Unreachable tests are exactly the
false confidence this audit exists to remove.