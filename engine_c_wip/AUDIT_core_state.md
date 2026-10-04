# AUDIT — cluster T3: the core state model (Rust→C parity)

Deliverable for the T3 Rust-module audit. **Flushed incrementally**; every row is a
Rust public item with its C twin or an explicit `NONE`, plus a measured
reachability count and a verdict.

## 0. Provenance / quiescence

Snapshot: `C:/Users/trios/AppData/Local/Temp/kilo/core2/` (robocopy of
`engine_c_wip`, `*.o` / `rb_engine*` / `tests/*.log` excluded, then every copied
`.o` deleted; 391 files, 0 `.o`, 0 `.log`).

Data blob, verified before and after every build in the snapshot:
`core2/src/abilities_strings.bin` size **726831**, SHA-256 `89204DF0BBDE08A0`
(the canonical one). The stale `cards/build/abilities_strings.bin` is **726649**
and was never copied in; `make data` was **not run** (no generator).

Base SHA-256 (16 hex) of the seven files this audit owns, read twice separated
by a settle — **quiescence confirmed**:

| file | sha256[:16] | mtime |
|---|---|---|
| `src/core/zones.c` | `651534B542DF6D10` | 13:48:45 |
| `src/core/game_state_core.c` | `FEB767B3BF5C1CD6` | 12:23:47 |
| `src/core/data.c` | `BAB1A93395027351` | 11:47:20 |
| `src/core/stats_pipeline.c` | `FFF33842ABA09156` | 12:23:48 |
| `src/core/tracking.c` | `EE2919074F4EA1ED` | 12:23:48 |
| `src/core/alloc.c` | `6E7E726E9F6CC647` | 12:23:47 |
| `src/core/alloc_counter.c` | `6A93B364B2FFB2D1` | 12:23:47 |
| `include/rabuka.h` (not mine) | `9497B0EBB200A3F2` | 06:39:50 |

Corpus identity: `cards/abilities.json` `unique_abilities` = **985**, matching
`RBKA_NUM_ABILITIES 985` in `gen_data.c`; `RBKA_NUM_STRING_OFFSETS 6033`
(= 6032 interned + terminator) matches the pinned table. All reachability
counts below are over those 985.

Library build in the snapshot: all **48** `OBJ_LIB` objects compile clean
(`gcc 15.3.0`, `-Wall`), 0 failures.

---

## 1. INVENTORY — `core/zones.rs` (1105 lines, 97 `pub` items)

C twin column: file:line in `engine_c_wip`. `NONE` = no C twin at all.

### 1.1 `Orientation` / `MemberArea` / free functions

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `enum Orientation{Active,Wait}` | 50 | `RbMods` orientation strings (`core/modifiers.c`) | WIRED (string-keyed) |
| `enum MemberArea{LeftSide,Center,RightSide}` | 58 | `rb_member_area_to_index` zones.c:5 | WIRED |
| `MemberArea::ALL` | 68 | — (compile-time list) | N/A |
| `MemberArea::as_str` | 74 | `rb_member_area_to_str` zones.c:12 | WIRED |
| `MemberArea::front_area` | 84 | `rb_member_area_front` zones.c:18 | WIRED |
| `MemberArea::to_index` | 93 | `rb_member_area_to_index` zones.c:5 | WIRED, **over-permissive** (see D3) |
| `MemberArea::from_index` | 102 | `rb_member_area_from_index` zones.c:745 | WIRED |
| `MemberArea::to_tag` | 112 | `rb_member_area_to_tag` zones.c:138 | WIRED |
| `MemberArea::from_tag` | 121 | `rb_member_area_from_tag` zones.c:146 | WIRED |
| `impl Display for MemberArea` | 131 | `rb_member_area_to_str` zones.c:12 | WIRED (same strings) |
| `impl FromStr for MemberArea` | 141 | `rb_member_area_to_index` zones.c:5 | WIRED, **over-permissive** (D3) |
| `check_trigger_position` | 157 | `rb_check_trigger_position` zones.c:153 | WIRED, identical |
| `check_effect_position` | 177 | `rb_check_effect_position` zones.c:161 | WIRED, **DIVERGENT** (D2) |

### 1.2 `Stage` (zones.rs:218)

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `struct Stage` | 218 | split over `RbPlayer.stage` + `RbPlayer.under_cards` (rabuka.h:762/764) | WIRED |
| `Stage::new` / `Default` | 237 | `rb_game_init` memset | WIRED |
| `Stage::invariant` | 246 | **NONE** | VERIFIED-NOOP (see 1.2a) |
| `Stage::get_area` | 252 | `rb_stage_get_area` zones.c:186 | WIRED |
| `Stage::set_area` | 267 | `rb_stage_set_area` zones.c:190 | WIRED |
| `Stage::stage_member_for_check_timing` | 278 | **NONE** | see 1.2b |
| `Stage::take_pending_duplicate_members` | 294 | **NONE** | VERIFIED-NOOP (1.2b) |
| `Stage::place_under_card` | 301 | `rb_stage_place_under_card` zones.c:294 | WIRED |
| `Stage::get_under_cards` | 311 | `rb_stage_get_under_cards` zones.c:300 | WIRED |
| `Stage::under_cards_with_hosts` | 322 | `rb_stage_under_cards_with_hosts` zones.c:310 | WIRED |
| `Stage::recycle_under_cards` | 345 | `rb_stage_recycle_under_cards` zones.c:330 | WIRED |
| `Stage::position_change` | 368 | `rb_stage_position_change{,_with_bags}` zones.c:271/280 | WIRED |
| `Stage::total_blades` | 424 | `rb_zone_total_blades` zones.c:51 | WIRED, **DIVERGENT** (D4) |
| `Stage::can_place_card` | 458 | `rb_stage_can_place_card` zones.c:351 | WIRED |
| `Stage::get_available_hearts` | 468 | `rb_stage_get_available_hearts` zones.c:859 | WIRED |
| *(no Rust twin in zones.rs)* | — | `rb_zone_invariant` zones.c:39 | C-only checker, stricter than Rust's length check. Not a parity claim. |

**1.2a `Stage::invariant`** — Rust checks three fixed-length arrays equal
`STAGE_SIZE`, so it is a compile-time tautology (`true`). C's
`rb_zone_invariant` (zones.c:39) instead checks slot/card coherence and the
"under-cards need a host" rule, which Rust has no equivalent of. No C caller
depends on Rust's tautology, so there is nothing to port.
**VERIFIED-NOOP: 0 of 985.**

**1.2b `stage_member_for_check_timing` / `take_pending_duplicate_members`** —
these implement Q143, the *duplicate member* timing check: a member deployed into
an occupied area parks the older member + its under-cards in
`pending_duplicate_members` instead of discarding it. Measured: the corpus has
**0** abilities whose effect places a member into an occupied stage area without
naming the area (`same_area` 7 abilities name it, and those take the
`Zone::SameArea` arm in `util.rs:2483-2510`, not this one). The C baton path
(`rb_play_member_ex`, `turn/phase.c`) resolves the area itself and sends the
replaced member to the waitroom, which is `Phase::execute_baton_touch`'s
behaviour, not `Stage`'s.
**VERIFIED-NOOP: 0 of 985.**

### 1.3 `LiveCardZone` (zones.rs:498)

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `struct LiveCardZone{cards,face_up}` | 498 | `RbPlayer.live` (rabuka.h:768); **`face_up` ABSENT** | see field table |
| `LiveCardZone::new` / `Default` | 511 | `rb_game_init` memset | WIRED |
| `can_place_card` | 518 | `rb_live_can_place_card` zones.c:580 | WIRED (`true`) |
| `add_card` | 523 | `rb_live_add_card` zones.c:592 | WIRED |
| `clear` | 537 | `rb_live_clear` zones.c:599 | WIRED |
| `len` | 541 | `rb_live_len` zones.c:607 | WIRED |
| `is_empty` | 545 | **NONE** | see 1.3a |
| `calculate_live_score` | 549 | `rb_live_calculate_score` zones.c:825 | WIRED |

**1.3a `LiveCardZone::is_empty`** — `cards.is_empty()`. Every C reader spells it
`player->live.n == 0` inline; there is no wrapper and no reader that could
disagree, because `len()` is `n` and `is_empty()` is `len()==0` over the same
field. No behavioural surface. No port needed.

### 1.4 `EnergyZone` (zones.rs:623)

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `struct EnergyZone{cards,active_energy_count}` | 623 | `RbPlayer.energy` + `.energy_active` (rabuka.h:765/767) | WIRED |
| `EnergyZone::new` / `Default` | 643 | memset | WIRED |
| `can_place_card` | 650 | `rb_energy_can_place_card` zones.c:516 | WIRED |
| `add_card` | 658 | `rb_energy_add_card` zones.c:522 | WIRED, **DIVERGENT** (D5) |
| `active_count` | 674 | `rb_energy_active_count` zones.c:544 | WIRED |
| `set_active_count` | 678 | `rb_energy_set_active_count` zones.c:549 | WIRED |
| `add_active` | 682 | `rb_energy_add_active` zones.c:556 | WIRED |
| `sub_active` | 686 | `rb_energy_sub_active` zones.c:563 | WIRED |
| `pay_energy` | 690 | `rb_energy_pay` zones.c:530 | WIRED |
| `activate_all` | 709 | `rb_energy_activate_all` zones.c:539 | WIRED |
| `push_active` | 718 | **NONE** | VERIFIED-NOOP (1.4a) |
| `set_indices_waited` | 731 | **NONE** | VERIFIED-NOOP (1.4a) |
| `set_indices_active` | 750 | **NONE** | VERIFIED-NOOP (1.4a) |
| `remove_at` | 770 | **NONE** | VERIFIED-NOOP (1.4a) |

**1.4a** The four missing mutators all exist to maintain the *positional*
convention (`cards[0..active]` active, rest waited) that `EnergyZone`'s doc
comment declares canonical. The C models the same state with two parallel
fields (`energy` bag + `energy_active` count) and never needs the rebuild.
Measured over the 985 abilities: **0** reach `push_active` /
`set_indices_waited` / `set_indices_active` / `remove_at` — the C energy-zone
writers reachable from card data are `rb_energy_add_card`,
`rb_player_draw_energy`, `rb_energy_set_active_count`, `rb_energy_add/sub_active`,
`rb_energy_activate_all`, `rb_energy_pay` and the generic
`rb_place_card_in_zone` energy arm, all of which maintain the count the C way.
**VERIFIED-NOOP: 0 of 985.**

### 1.5 `MainDeck` / `EnergyDeck` / `Hand` / `Waitroom` / `SuccessLiveCardZone` /
### `ExclusionZone` / `ResolutionZone`

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `MainDeck::new`/`Default` | 800 | memset | WIRED |
| `MainDeck::shuffle` | 806 | `rb_deck_shuffle` zones.c:753 | WIRED |
| `MainDeck::draw` | 811 | `rb_zone_draw` zones.c:70 | WIRED |
| `MainDeck::draw_bottom` | 823 | `rb_zone_draw_bottom` zones.c:79 | WIRED |
| `MainDeck::draw_multiple` | 834 | `rb_zone_draw_multiple` zones.c:85 | WIRED (count-only) |
| `MainDeck::is_empty` | 838 | `rb_deck_is_empty` zones.c:759 | WIRED |
| `MainDeck::len` | 842 | `rb_deck_len` zones.c:765 | WIRED |
| `EnergyDeck::new`/`Default` | 863 | memset | WIRED |
| `EnergyDeck::draw` | 869 | `rb_energy_deck_draw` zones.c:771 | WIRED |
| `EnergyDeck::is_empty` | 880 | `rb_energy_deck_is_empty` zones.c:782 | WIRED |
| `Hand::new`/`Default` | 902 | memset | WIRED |
| `Hand::add_card` | 908 | `rb_hand_add` zones.c:613 | WIRED |
| `Hand::remove_card` | 912 | `rb_hand_remove_card` zones.c:618 | WIRED |
| `Hand::len` | 920 | `rb_hand_len` zones.c:626 | WIRED |
| `Hand::is_empty` | 924 | `rb_hand_is_empty` zones.c:630 | WIRED |
| `Waitroom::new`/`Default` | 947 | memset | WIRED |
| `Waitroom::add_card` | 953 | `rb_waitroom_add` zones.c:636 | WIRED |
| `Waitroom::take_all` | 957 | `rb_waitroom_take_all` zones.c:641 | WIRED |
| `Waitroom::shuffle` | 961 | `rb_waitroom_shuffle` zones.c:649 | WIRED |
| `Waitroom::len` | 965 | `rb_waitroom_len` zones.c:654 | WIRED |
| `Waitroom::is_empty` | 969 | **NONE** | inline `discard.n==0`; no surface |
| `Waitroom::remove_card` | 973 | `rb_waitroom_remove_card` zones.c:658 | WIRED |
| `SuccessLiveCardZone::new`/`Default` | 996 | memset | WIRED |
| `SuccessLiveCardZone::add_card` | 1002 | `rb_success_add` zones.c:677 | WIRED |
| `SuccessLiveCardZone::len` | 1006 | `rb_success_len` zones.c:682 | WIRED |
| `SuccessLiveCardZone::is_empty` | 1010 | **NONE** | inline `success.n==0`; no surface |
| `ExclusionZone::new`/`Default` | 1032 | memset | WIRED |
| `ExclusionZone::add_card` | 1038 | `rb_exclusion_add` zones.c:899 | WIRED; `exclusion_zone` = 0 of 985 zone strings |
| `ResolutionZone::new` | 1056 | memset | WIRED |
| `ResolutionZone::add_card` | 1063 | `rb_resolution_add` zones.c:688 | WIRED |
| `ResolutionZone::add_card_for_owner` | 1067 | **NONE** — `owners` member ABSENT | see 1.5a |
| `ResolutionZone::swap_slots` | 1072 | `rb_resolution_swap_slots` zones.c:733 | WIRED minus check 4 (1.5a) |
| `ResolutionZone::clear` | 1094 | `rb_resolution_clear` zones.c:693 | WIRED |
| `ResolutionZone::len` | 1099 | `rb_resolution_len` zones.c:701 | WIRED |
| `ResolutionZone::is_empty` | 1103 | **NONE** | inline `resolution.n==0`; no surface |

**1.5a `ResolutionZone::owners`** — Rust keeps a parallel `owners: SmallVec<u8>`
beside `cards` and `swap_slots` bounds-checks BOTH (zones.rs:1082-1087). The C
`GameState::resolution` is a bare `RbBag` (rabuka.h:1603), so `rb_resolution_add`
takes no owner and `rb_resolution_swap_slots` cannot perform checks 4/5. Rust's
`owners` is written by `tracking.rs:93` (`add_card_for_owner(card_id, owner)`) and
`turn/live.rs:1976`; the C cheer path (`tracking.c rb_perform_cheer_check`) calls
`rb_resolution_add`. **Observable consequence already recorded in the C source**:
a P2 yell card lands in P1's waitroom, because the C drain
(`rb_check_invalid_resolution_zone`, `turn/phase.c`) assumes the active seat.
**BLOCKED-ON: `include/rabuka.h` — a parallel owner array on the resolution
zone, e.g. `uint8_t resolution_owners[RB_MAX_ZONE]; int n_resolution_owners;`
beside `RbBag resolution;` (rabuka.h:1603), plus
`int rb_resolution_add_for_owner(GameState *g, int card_id, int owner);`**

---

## 2. DIVERGENCES FOUND (numbered for the report)

Each carries its measured reachability.

### D1 — `rb_stage_first_empty` probes LEFT first; Rust probes CENTRE first
`engine_c_wip/src/core/zones.c:286`

```c
int rb_stage_first_empty(const int stage[RB_STAGE_SIZE]){
    for(int i=0;i<RB_STAGE_SIZE;i++) if(stage[i]==RB_EMPTY_SLOT) return i;
    return -1;
}
```

Rust twin, `ability/util.rs:2541-2551`:

```rust
pub fn stage_first_empty(stage: &[i16; 3]) -> Option<usize> {
    if stage[1] == -1 { Some(1) }          // CENTRE
    else if stage[0] == -1 { Some(0) }
    else if stage[2] == -1 { Some(2) }
    else { None }
}
```

So the two engines choose a DIFFERENT SLOT whenever the centre is empty and the
left is empty. `src/ability/util.c:1104-1116` already documents this and works
around it with a private `rb_util_stage_first_empty_rust`, explicitly refusing to
fix it because *"`zones.c` is not this file"*. **zones.c IS this cluster's file,
so the workaround is removable and the root fix belongs here.**

Production callers of the buggy helper:
* `src/ability/util.c:1143` — `rb_place_card_in_zone`, the
  `zone == "stage" || zone == "empty_area"` arm. Rust twin: `util.rs:2442-2456`,
  whose `stage_first_empty(&player.stage.stage)` is the centre-first probe.
* `src/turn/phase.c:1361` — the single-baton area choice, whose Rust twin is
  `stage_area.unwrap_or_else(|| first empty, else ALL[0])` in `phases.rs`.

**Measured reachability.** `destination`/`dest_zone`/`to`/`move_to` reaching a
placement action (`draw_card`, `draw_until_count`, `discard_until_count`,
`move_cards`, `gain_resource`, `select_cards`, `look_and_select`, `look_at`,
`reveal`, `select`, `reveal_until_live_card`) in the 985-ability corpus:

| destination | abilities reaching a placement action |
|---|---|
| `stage` | **10** |
| `empty_area` | **8** |
| `same_area` | **7** (all of which route to the same `RB_ZONE_STAGE` arm) |

i.e. **25 of 985** take the affected arm, plus every `rb_move_card` /
`rb_place_card_in_zone` caller that does not name an area at all.
**REACHABLE — n > 0. FIX IN `zones.c`.**

### D2 — `check_effect_position`: C accepts six Japanese spellings Rust does not
`zones.c:161-184`. Rust `zones.rs:177-206` matches only `center`, `left`,
`left_side`, `right`, `right_side`; every other spelling falls into the `_` arm,
whose body is `!(pos == "center" || pos == "left" || … )` — i.e. **an
unrecognised position always returns `true`**. C returns `1` for `中央`/`左`/`右`/
`左側`/`右側` *only at the matching position* and `0` otherwise
(zones.c:179-183), so for those five spellings C returns `false` where Rust
returns `true`. Measured over the 985 abilities: the decoded
`activation_position` values are `center` / `left` / `right` / `left_side` /
`right_side` / `stage_position`-free — **the five Japanese spellings appear 0
times**, so the disagreement is inert today.
**VERIFIED-NOOP: 0 of 985.** Recorded, not "fixed": deleting C's extra arms
would change a currently-passing behaviour to match a Rust arm nothing reaches,
and `rb_check_effect_position` is *not* the only position reader (see 1.2a note on
`check_trigger_position`, which is exact).

### D3 — `MemberArea` string decode is over-permissive in C
`rb_member_area_to_index` (zones.c:5) also accepts `"LeftSide"`, `"Center"`,
`"RightSide"`. Rust `MemberArea::from_str` (zones.rs:145-151) accepts only
`left`/`center`/`right` and returns `Err` for the CamelCase spellings. Measured:
**0 of 985** abilities carry a CamelCase `position` / `activation_position`.
**VERIFIED-NOOP: 0 of 985.**

### D4 — `total_blades` consults an extra "waited" source
`zones.c:51-69`. Rust `Stage::total_blades` (zones.rs:424-456) skips a stage card
only when `include_waited == false` **and** `orientation_modifiers.get(&card_id)
== CardOrientation::Wait` — a **per-card** map. C skips when
`P->stage_wait[s]` **or** the orientation string says `wait`, i.e. it adds a
**per-area** condition Rust has no counterpart for. `stage_wait` is set by
`rb_effect_change_state` and read back by `rb_zone_total_blades`, so a member
weighed through the C state-change path but never given an orientation modifier
string is excluded by C and counted by Rust.
Reachability is measured in §5 (D4-row); the fix belongs in whichever function
writes `stage_wait`, and that function is **not** mine.

### D5 — `EnergyZone::add_card` loses the energy-type check
`zones.c:522-528`. Rust `zones.rs:658-672` refuses with
`"Only energy cards can be placed in Energy Zone"` when
`card_db.get_card(id).is_energy()` is false. C's `rb_energy_add_card` performs
**no type check at all** — it only bounds-checks the bag. Reachability measured
in §5 (D5-row).

### D6 — `place_card_in_zone` empty destination
`draw.c:91-95` sends every `rb_zone_of_str == 0` destination to the **hand**;
Rust `util.rs:2530-2537` sends an **empty** destination to the **waitroom** and
only a non-empty unrecognised one to the hand. Measured: the corpus has **0**
`destination: ""` and 906 *absent* destinations, which both engines resolve to
`hand` (`draw.c:468` / `draw.rs` `unwrap_or(Zone::Hand)`).
**VERIFIED-NOOP: 0 of 985.**

### D7 — `GameState.rule_log` still has no field
`RB_GSC_RULE_LOG_MAX 500` exists at `include/rabuka.h:3489`, but **no
`rule_log` member was added to `GameState`**. `rb_gsc_push_rule_log`
(game_state_core.c:712) therefore still routes into
`rb_log_push_verdict(text, "rule_log", 1)` — the shared **32-slot
condition-verdict ring** in `ability/log.c`, which drops on overflow. That is
exactly what the header comment at rabuka.h:3484-3488 and the file banner at
game_state_core.c:36-45 forbid. Consequence: the rule log and the condition
verdict log destroy each other's entries, and nine `rb_gsc_*` functions stay
unroutable. See §4.

**BLOCKED-ON: `include/rabuka.h` — `GameState` needs, appended at the end of the
struct (rabuka.h:1769, before `} GameState;` at 1770):
```c
    /* Rust GameState.rule_log: Vec<String> (core/game_state/mod.rs:605),
     * bounded by LOG_BOUND_RULE = 500, newest-kept (mod.rs:1602-1606). */
    char     rule_log[RB_GSC_RULE_LOG_MAX][RB_GSC_RULE_LOG_LINE_MAX];
    uint16_t rule_log_len[RB_GSC_RULE_LOG_MAX];
    int      n_rule_log;
```
plus `#define RB_GSC_RULE_LOG_LINE_MAX 256` next to `RB_GSC_RULE_LOG_MAX` at
rabuka.h:3489. `RB_GSC_RULE_LOG_MAX` alone is not enough — it is a count, and a
`Vec<String>` needs the strings.**

---

## 3. INVENTORY — the other five Rust modules

### 3.1 `core/card_binary.rs` (718 lines, 9 `pub` items)

C twins live in `core/card.c` (cluster G+H, **not** mine) and `core/data.c`
(mine). `card.c` is on the never-edit list, so a card_binary gap is a report.

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `EXTERN_CARD_BLOB` / `_LEN` statics | 11 / 13 | `rb_load` / `rb_unload` + the `g_card_blob` globals, `data.c` | WIRED (feature-gated in Rust too) |
| `parse_header` (private) | 42 | `rb_load` blob header parse, `data.c` | WIRED |
| `card_data_offset` (private) | 63 | `rb_card_record` / `rb_card_record_len`, `data.c` | WIRED |
| `find_string_index_by_no` (private, external-only) | 78 | `rb_card_stridx`, `data.c` | WIRED |
| `decode_card_from_record` (private) | 99 | `rb_decode_card_by_index`, `card.c:84` | WIRED, **DIVERGENT** (D8) |
| `decode_card_from_blob` | 267 | `rb_decode_card_by_index`, `card.c:84` | WIRED |
| `get_str_from_strtab` (private) | 276 | `rb_card_stridx` / `rb_card_string`, `data.c` | WIRED |
| `decode_all_cards_from_slice` | 299 | `rb_load_streaming` / `rb_load_rom`, `data.c` | WIRED (different shape: C streams from a file, Rust from a slice) |
| `parse_hearts` (private) | 326 | the `n_hearts` loop at `card.c:110-117` | WIRED, **DIVERGENT** (D8) |
| `color_from_u8` (private) | 342 | `sp_heart_index`, `stats_pipeline.c:58` | WIRED |
| `load_cards_from_blob` | 362 | `rb_load` + `rb_register_card_copy` / `rb_create_card_copy`, `data.c` | WIRED |
| `blob_card_count` | 391 | `rb_num_cards`, `data.c` | WIRED |
| `find_card_index_by_no` | 398 | `rb_find_card_by_no`, `data.c` | WIRED |
| `normalize_card_no` | 431 | `rb_normalize_card_no`, `game/deck_parser.c:27` | WIRED — **NOT in `data.c`**; it is `DeckParser`'s twin and lives in cluster T5's file. One definition, one owner; not duplicated. |
| `blob_index_by_folded_no` | 456 | `rb_find_card_by_no`'s fold-on-miss path, `data.c` | WIRED (C folds inside the lookup instead of building a sorted index; same answers, no separate index) |
| `resolve_card_no` | 476 | `rb_find_card_by_no`, `data.c` | WIRED |

**D8 — the decoder keeps zero-count heart pairs.** `parse_hearts`
(card_binary.rs:326-340) inserts only `if count_val > 0`, so a `(color, 0)` pair
never reaches the `HeartMap` and a need whose pairs are all zero reads as
`hearts.is_empty() == true`. `rb_decode_card_by_index` (card.c:110-117) stores
**every** pair, zero counts included, into `heart_color[]`/`heart_count[]` and
reports the larger `n_hearts`.
Reachability of the consequence, measured on the readers that exist:
* `stats_pipeline.c:79-89` (`sp_card_base_hearts`) and `:91-109`
  (`sp_card_need_hearts`) both do `out[idx] += card.heart_count[i]`, so a zero
  pair contributes nothing.
* `zones.c:814-819` (`live_score_hearts_satisfied`) tests `need[c] > 0` over the
  derived table, so a zero pair still reads as "no requirement".
* `zones.c:577-597`'s empty-need early-true is reproduced by `sp_any_hearts`
  (`stats_pipeline.c:141`) rather than by `n_hearts`.
**VERIFIED-NOOP: 0 of 985** — no reader in the tree branches on `n_hearts`
membership, so the extra pair is invisible. `n_hearts` is also still used as the
`i < card.n_hearts` loop bound, which is the one place the larger count could
have mattered; it happens to be exactly `num_base + num_blade + num_need` for
every card in the blob, which is what `card.c:88` already assumes.

**D8b — `decode_card_from_record` length guard.** Rust rejects `data.len() < 20`
and then reads `data[24]`, i.e. it requires 25 and would panic on a 20–24-byte
record; C rejects `< 25`. No record in the blob is shorter than 25, so the guard
difference is unreachable, and C's is the safe one.
**VERIFIED-NOOP: 0 of 2639 cards.**

### 3.2 `core/player.rs` (666 lines, 23 `pub` items)

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `struct PlayerId(Arc<str>)` | 30 | **NONE** — no C member | see field table |
| `PlayerId::as_str` + `Deref`/`AsRef`/`Display`/`From`×7/`PartialEq`×7 | 34-130 | `gsc_player_id` / `gsc_id_is`, `game_state_core.c:127/132` | WIRED (function-local, one definition) |
| `struct Player` | 160 | `RbPlayer`, rabuka.h:759-777 | see field table |
| `Player::new` | 218 | `rb_game_init` + `game_setup.c` | WIRED |
| `set_main_deck` | 260 | `rb_player_set_main_deck` zones.c:908 | WIRED |
| `set_energy_deck` | 264 | `rb_player_set_energy_deck` zones.c:918 | WIRED |
| `get_card_index_by_id` | 271 | `rb_player_get_card_index_by_id` zones.c:929 | WIRED |
| `add_card_to_hand` | 277 | `rb_hand_add` zones.c:613 | WIRED |
| `is_area_locked` | 286 | `rb_player_is_area_locked` zones.c:940 | WIRED, **DIVERGENT** (D9) |
| `remove_member_from_stage_with_recycling` | 299 | `rb_player_remove_member_from_stage_with_recycling` zones.c:949 | WIRED |
| `move_card_from_hand_to_stage` | 344 | `rb_player_move_card_from_hand_to_stage` zones.c:977 | WIRED, **DIVERGENT** (D10) |
| `calculate_stage_hearts` | 516 | `rb_player_calculate_stage_hearts` zones.c:1036 | WIRED |
| `activate_all_energy` | 535 | `rb_player_activate_all_energy` zones.c:1042 | WIRED |
| `activate_all_energy_exclude` | 549 | `rb_player_activate_all_energy_exclude` zones.c:1050 | WIRED |
| `all_card_ids` | 557 | `rb_player_all_card_ids` zones.c:1060 | WIRED |
| `draw_card` | 578 | `rb_player_draw_card` zones.c:1094 | WIRED |
| `draw_energy` | 585 | `rb_player_draw_energy` zones.c:1106 | WIRED |
| `refresh` | 634 | `rb_zone_refresh` zones.c:118 | WIRED (Rust's `waitroom` == C's `discard` bag) |
| `contains_card` | 651 | `rb_player_contains_card` zones.c:1120 | WIRED |
| `track_deployment` | 661 | `rb_zone_track_deployment` zones.c:1141 | WIRED, **DIVERGENT** (D9) |

**D9 — `deployed_this_turn` is per CARD in Rust and per AREA in C.**
Rust `Player::is_area_locked` (player.rs:286-294) resolves the member currently in
the slot and asks whether **that card id** is in `deployed_this_turn`
(`SmallVec<[i16;4]>`, player.rs:200); `track_deployment` (player.rs:661) pushes
the card id. The doc comment is explicit that this is deliberate — "The
restriction follows the member (R4), not the area" — because Rule 11.10 lets a
deployed member position-change, which moves the lock to the destination area.
C has `GameState.stage_arrived[2][3]` (rabuka.h:1610): `rb_zone_track_deployment`
(zones.c:1141) finds the card's slot and sets **that area's** flag, and
`rb_player_is_area_locked` (zones.c:940) reads the area's flag directly. After a
position change the two engines therefore lock **different areas**.
Corpus reach: every `baton touch` / `プレイバトンタッチ` on a member that can
position-change. `placement_change` = 29 `position_change` actions in the corpus
(const_corpus tally), and the corpus has 3 `play_baton_touch` actions. The
position-change sites that move a member (`rb_stage_position_change_core`,
zones.c:252-257) DO carry `stage_wait` across the swap but have **no** `stage_arrived`
carry — so the lock is provably left behind on the fix. `stage_arrived` is
written by `rb_place_card_in_zone` (util.c:1216/1223), `turn/phase.c` and
`choice.c`; the fix for the carry is one line beside the `stage_wait` crossed swap
in `zones.c`, and it is in **my** file.
**REACHABLE — see D9-fix note in §6.**

**D10 — `move_card_from_hand_to_stage` drops three of Rust's seven arguments.**
Rust takes `replaced_member_cost_mod: i16`, `played_card_cost_mod: i16` and
`waited_stage_cards: &[i16]` and resolves the play cost through
`util::compute_play_cost(self, card_id, card_db, played_card_cost_mod,
waited_stage_cards)` (player.rs:376-382) — the modifiers are *passed in* by the
caller, explicitly so the whole `GameModifiers` need not be cloned
(player.rs:412-415). C's twin (zones.c:977) calls
`rb_compute_play_cost(g, pl, card_id, 0)` and has no `waited_stage_cards`
parameter, so the replacement-member cost reduction and the waited-members
adjustment both read 0/empty.
**VERIFIED-NOOP: 0 of 985 on this twin** — `rb_player_move_card_from_hand_to_stage`
has **no production caller** (the play path is `rb_play_member_ex`,
`turn/phase.c:1352-1374`, which has its own implementation of the same rule).
The divergence is a duplicate-implementation hazard, not a live bug: two copies
of `move_card_from_hand_to_stage` exist and only one is called.

### 3.3 `core/game_modifiers.rs` (641 lines, 50 `pub` items) — see §4.2 field table

Method-by-method twin map (C is `core/modifiers.c`, cluster E's, which is **not**
mine; the twin column names the file so a routing request is unambiguous):

| Rust item | line | C twin |
|---|---|---|
| `enum CardOrientation{Active,Wait}` + `as_str` + `Display` | 13 / 19 / 27 | `RbMods.orientation[]` (uint8 0/1/2) via `rb_mods_set_orientation` / `rb_mods_get_orientation`, `core/modifiers.c` |
| `struct ModifierEntry{additive,set}` | 59 | `RbModifierEntry{int16_t set; int16_t add}` rabuka.h:198 — same width, and `rb_modifier_total` is `set+add`, matching `total()` |
| `ModifierEntry::total` | 138 | `rb_modifier_total` rabuka.h:199. **NOTE**: C's inline is `(int)e.set + (int)e.add`, i.e. **plain `+`, not `saturating_add`**. Rust `total()` is `self.set.saturating_add(self.additive)` (game_modifiers.rs:139) and the doc says a wrapping sum "would silently turn a large positive modifier into a negative one and invert a comparison". Both fields are `i16`, so `32767 + 32767` wraps to `-2` in C and saturates to `32767` in Rust. **VERIFIED-NOOP: 0 of 985** — no ability stacks anywhere near 32767 (the corpus's largest single modifier value is a cost `set` of 10, const_corpus ab#605), so the two cannot differ on this corpus. |
| `apply_need_heart_modifiers` | 99 | `sp_apply_need_mods`, `stats_pipeline.c:119` (mine) — set pass then additive pass, per colour, exactly Rust's two loops at :103-116 |
| `struct BonusSource` | 153 | **NONE** — see §4.2 |
| `struct GameModifiers` (37 fields) | 177 | `RbMods`, rabuka.h:228-273 — see §4.2 |
| `GameModifiers::new` / `Default` | 260 / 255 | `rb_mods_init`, `core/modifiers.c` |
| `add_blade_modifier` | 303 | `rb_mods_add_blade` |
| `add_blade_modifier_with_trace` | 309 | `rb_mods_add_blade` + `rb_mods_trace_push`; **no `src/` caller** for the trace half (the `AbilityApplication` ring has no consumer) |
| `remove_blade_modifier` | 332 | `rb_mods_remove_blade` |
| `get_blade_modifier` | 341 | `rb_mods_get_blade` |
| `get_blade_set_modifier` | 345 | `rb_mods_get_blade_set` |
| `set_blade_modifier` | 351 | `rb_mods_set_blade` |
| `clear_blade_set_modifier` | 355 | `rb_mods_clear_blade_set` — **0 C callers** (Rust's reader is the `clear_blade_set` temporary-effect undo at `abilities.rs:3061+`, which has no C counterpart) |
| `set_blade_type_modifier` | 364 | `RbMods.blade_type[]` written by `rb_mods_set_blade_type` |
| `clear_blade_type_modifier` | 368 | `rb_mods_clear_blade_type` — **0 C callers**, same reason |
| `add_heart_modifier` | 374 | `rb_mods_add_heart` |
| `add_heart_modifier_with_trace` | 381 | `rb_mods_add_heart` + trace ring; no `src/` consumer |
| `get_heart_modifier` | 405 | `rb_mods_get_heart` — includes the `HeartColor::Heart00` wildcard add, as Rust's `color_val().saturating_add(wildcard_val())` does |
| `remove_heart_modifier` | 423 | `rb_mods_remove_heart` |
| `set_heart_override` | 437 | `heart_color_override[]` + `heart_override_count[]` |
| `remove_heart_override` | 441 | `rb_mods_remove_heart_override` |
| `set_heart_copy` | 445 | `heart_copy[]` |
| `get_heart_copy` | 449 | `heart_copy[]` read |
| `add_score_modifier` | 455 | `rb_mods_add_score` |
| `remove_score_modifier` | 460 | `rb_mods_remove_score` |
| `get_score_modifier` | 469 | `rb_mods_get_score` |
| `set_score_modifier` | 473 | `rb_mods_set_score` |
| `get_score_set_modifier` | 477 | `rb_mods_get_score_set` |
| `clear_score_set_modifier` | 481 | `rb_mods_clear_score_set` — **0 C callers** (same `cards_activated_this_turn` reader gap) |
| `add_need_heart_modifier` | 492 | `rb_mods_add_need_heart` |
| `get_need_heart_modifier` | 498 | `rb_mods_get_need_heart` |
| `set_need_heart_modifier` | 505 | `rb_mods_set_need_heart` |
| `add_orientation_modifier` | 516 | `rb_mods_set_orientation` — Rust's `_ => return` arm (an unrecognised orientation string is IGNORED) is reproduced |
| `add_cost_modifier` | 530 | `rb_mods_add_cost` |
| `remove_cost_modifier` | 548 | `rb_mods_remove_cost` |
| `get_cost_modifier` | 557 | `rb_mods_get_cost` |
| `get_cost_modifier_set` | 563 | `rb_mods_get_cost_set` (the `.filter(|&v| v != 0)` → `None` is reproduced) |
| `set_cost_modifier` | 567 | `rb_mods_set_cost` |
| `remove_cost_modifier_set` | 571 | `rb_mods_clear_cost_set` — **0 C callers** |
| `get_orientation_modifier` | 580 | `rb_mods_get_orientation` |
| `add_delayed_cannot_active` | 589 | `rb_mods_add_delayed_cannot_active` (the `current.max(turns)` keep-the-larger rule is reproduced) |
| `is_delayed_cannot_active` | 601 | `rb_mods_is_delayed_cannot_active` |
| `tick_delayed_cannot_active_for` | 614 | `rb_mods_tick_delayed_for` (owner-scoped, Q280) |
| `clear_all_for_card` | 624 | `rb_mods_clear_card` — **NOT the same set.** Rust removes 16 maps; see D11 |

**D11 — `clear_all_for_card` does not remove `constant_cost_set_bonuses` in Rust,
and C's twin cannot remove `heart_multiplier` faithfully.** Rust's 16 removals are
blade, blade_type, heart, heart_override, score, need_heart, orientation, cost,
constant_blade, constant_cost, constant_score, constant_heart,
heart_color_multiplier, heart_copy, delayed_cannot_active. C's `rb_mods_clear_card`
clears the corresponding arrays. Measured: the two **deliberately differ** on
`constant_cost_set_bonuses` — modifiers.c:1239-1244 states Rust does NOT remove
it there, which is correct. **No divergence found; recorded so the difference is
not "fixed" later.**

### 3.4 `alloc_counter.rs` (477 lines, 11 `pub` items)

Twin: `src/core/alloc_counter.c` (779 lines, 49 exported symbols) + the process
gates it also owns from `core/runtime_flags.rs`.

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `struct TraceGuard` + `Drop` | 136 / 141 | `rb_allocctr_trace_guard` scope struct, `alloc_counter.c` | WIRED |
| `maybe_arm` | 169 | `rb_allocctr_maybe_arm` | WIRED |
| `arm_alloc_trace` | 192 | `rb_allocctr_arm_trace` | WIRED |
| `disarm_alloc_trace` | 200 | `rb_allocctr_disarm_trace` | WIRED |
| `struct CountingAllocator` + `GlobalAlloc` | 213 / 215 | `rb_allocctr_malloc` / `rb_allocctr_free` | WIRED **as an instrumented allocator, not a global** — see D12 |
| `clock_overhead_ns` | 268 | `rb_allocctr_clock_overhead_ns` | WIRED |
| `allocator_self_ns` | 282 | `rb_allocctr_self_ns` | WIRED |
| `alloc_count` | 347 | `rb_allocctr_count` | WIRED |
| `alloc_buckets` | 359 | `rb_allocctr_buckets` | WIRED (14 buckets, same labels) |
| `start` | 369 | `rb_allocctr_start` | WIRED |
| `struct AllocGuard` + `Drop` | 380 / 385 | `rb_allocctr_guard` | WIRED |

**D12 — the counters cannot see real traffic.** Rust installs
`CountingAllocator` as `#[global_allocator]`, so every allocation in the process
is counted with no call site opting in. C has no such mechanism: an allocator is
an ordinary named function. `alloc_counter.c`'s own banner records this as a
cross-file request — `alloc.c`'s `rb_malloc`/`rb_free` must delegate to
`rb_allocctr_malloc`/`rb_allocctr_free`.
**Gameplay reachability: 0 of 985, permanently.** `alloc_counter` is
`#[cfg(feature = "alloc_tracker")]`-gated diagnostic instrumentation reached only
from `timer` / the bench; it sits below the card layer and no ability can call it.
This is the one module in the assignment whose entire surface is unreachable from
card data by construction, so its divergences are recorded, not fixed.

`core/alloc.rs` (the pool allocator, `src/core/alloc.c`) is **not** in this
assignment's Rust list; `src/core/tracking.c` is the twin of
`core/game_state/tracking.rs`, likewise not assigned. Both files were read for
their callers (see §5) but no Rust item in them is claimed here.

### 3.5 `core/game_state/mod.rs` (1749 lines, 66 `pub` items)

`struct GameState` has **126** `pub` fields. The item-level twin map:

| Rust item | line | C twin | verdict |
|---|---|---|---|
| `enum PlayerCount` + `PlayerCount::ALL` + `other_index`/`iter` | 17-30 | `game_setup.c`'s seat loop | WIRED |
| `struct PhaseChange` | 89 | — | display-only, no card reachability |
| `struct GameState` | 105 | `GameState`, rabuka.h:1448-1770 | see §4.1 |
| `GameState::new` | 403 | `rb_game_init` | WIRED |
| `init_bank` | 553 | `game_setup.c` setup path | WIRED |
| `record_ability_activation` | 596 | `rb_record_ability_activation` (turn/turn.c) | WIRED |
| `record_cost_paid` | 606 | `rb_record_cost_paid` | WIRED |
| `push_revealed_card` / `push_revealed_card_mut` | 613/622 | `rb_gsc_push_revealed_card` game_state_core.c:504 (**0 external callers**) | routable, see §7 |
| `push_revealed_cost_card` / `_mut` | 632/638 | `rb_push_revealed_cost_card` | WIRED |
| `record_card_appearance` | 653 | `rb_record_card_appearance` tracking.c:43 | WIRED, **cap 32** (audit_zones red-pin, pre-existing) |
| `record_card_movement` | 671 | `rb_record_card_movement` | WIRED |
| `record_deployed_card` | 680 | `rb_zone_track_deployment` zones.c:1141 | WIRED but see D9 |
| `record_card_movement_batch` / `_with_card` | 691/703 | `rb_record_card_movement_batch*` | WIRED |
| `record_opponent_cause` | 707 | `rb_record_opponent_cause` | WIRED |
| `record_cheer_revealed_card` / `_mut` | 723/733 | `game_setup.c` | WIRED |
| `push_rule_log` / `_fmt` | 739/745 | `rb_gsc_push_rule_log{,_fmt}` game_state_core.c:712/716 | **WRONGLY WIRED — see D7** |
| `log_entry` | 761 | `rb_gsc_log_entry` game_state_core.c:734 (0 callers) | routable, needs the field |
| `rule_log_contains` | 773 | **NONE** | VERIFIED-NOOP: 0 of 985 |
| `push_choice_offered` / `push_choice_resolved` | 786/794 | `rb_gsc_push_choice_{offered,resolved}` (0 callers) | routable, needs the field |
| `choice_offered_labels` / `choice_option_count` | 802/810 | `rb_gsc_choice_offered_labels`/`_option_count` (0 callers) | routable, needs `active` |
| `debug_trace_push` / `_contains` / `dump` | 828/838/844 | **NONE** | VERIFIED-NOOP: 0 of 985 |
| `performance_snapshot_*` ×5 | 855-905 | `GameState.snapshots[]` + `n_snapshots` + `performance_resume_*` | WIRED |
| `push_performance_snapshot` | 911 | `rb_gsc_push_performance_snapshot` (0 callers) | routable |
| `record_ability_application` | 921 | `rb_gsc_record_ability_application` game_state_core.c:787 (**WIRED**, live.c:1537) | WIRED |
| `push_structured_log` | 934 | `rb_log_push` (ability/log.c) — shared ring | WIRED (different backing store) |
| `debug_trace_enabled` | 943 | `rb_log_debug_enabled` | WIRED |
| `last_ability_trace` / `debug_trace_contains` / `dump_last_trace` | 951/959/966 | **NONE** (Rust's own 0 call sites) | VERIFIED-NOOP |
| `player_prefix_id` | 979 | `rb_gsc_player_prefix_id` game_state_core.c:512 (**WIRED**, resolver.c:860) | WIRED |
| `can_player_act` | 989 | `rb_gsc_can_player_act` game_state_core.c:240 (0 callers) | routable |
| `seat_player` / `seat_player_mut` / `const_active_player` / `active_player` | 1005/1026/1033/1039 | `rb_gsc_seat_player{,_mut}` / `rb_gsc_{const_,}active_player` | WIRED in semantics (the C addresses `g->p[seat]` inline everywhere) |
| `seated_pair` | 1051 | `rb_gsc_seated_pair` (0 callers; 40 Rust call sites, **all** in `bot/**`+`bin/**`) | out-of-scope twins |
| `zone_is_private` | 1063 | `rb_gsc_zone_is_private` game_state_core.c:563 (0 callers) | routable |
| `snapshot_zone_counts` | 1073 | `rb_gsc_zone_snapshot` game_state_core.c:529 (**header-declared** at rabuka.h:3265, 0 callers) | routable today — only declaration exists |
| `zone_snapshot_cards` | 1087 | `rb_gsc_zone_snapshot`'s card slice | same |
| `resolve_index_any_player` | 1097 | `rb_gsc_resolve_index_any_player` game_state_core.c:595 (0 callers) | routable |
| `cheer_count_by_id` / `set_cheer_count_by_id` / `seat_cheer_count` | 1105/1111/1117 | `rb_gsc_cheer_count_by_id` / `_set_` / `_seat_` (0 callers) | routable |
| `score_bonus_by_id` / `seat_score_bonus` | 1123/1129 | `rb_gsc_score_bonus_by_id` / `_seat_` (0 callers) | routable |
| `clear_effect_tracking` | 1141 | `rb_gsc_clear_effect_tracking` game_state_core.c:440 (0 callers) | routable |
| `mulligan_owner_index` | 1211 | `rb_gsc_mulligan_owner_index` game_state_core.c:341 (**WIRED**, game_setup.c:542) | WIRED |
| `on_cards_left_zones` / `_mut` | 1219/1225 | `rb_on_cards_left_zones` modifiers.c | WIRED |
| `phase_invariant` | 1279 | `rb_gsc_phase_invariant` game_state_core.c:213 (0 callers) | routable |
| `get_current_phase` | 1291 | `rb_gsc_current_turn_phase` game_state_core.c:192 (**header NOT declared**; 0 Rust call sites) | VERIFIED-NOOP: 0 of 985 |
| `current_ability_source_card_id` | 1301 | `rb_gsc_current_ability_source_card_id` game_state_core.c:491 (**WIRED**, modifiers.c:814) | WIRED |
| `is_first_turn` | 1311 | `rb_gsc_is_first_turn` game_state_core.c:185 (0 callers) | routable |
| `game_has_ended` / `reject_if_ended` | 1319/1325 | `rb_gsc_game_has_ended` / `_reject_if_ended` (0 callers) | routable |
| `concede` / `concede_both` | 1333/1341 | `rb_gsc_concede` (**WIRED** ×4, game_setup.c) / `rb_gsc_concede_both` (0 callers) | mixed |
| `seat_index_by_id` / `try_player_by_id{,_mut}` | 1349/1357/1363 | `rb_gsc_seat_index_by_id` / `_try_player_by_id{,_mut}` (0 callers) | routable |
| `opponent_has_performed` | 1371 | `rb_gsc_opponent_has_performed` game_state_core.c:294 (0 callers; 2 Rust call sites, **both** in `bot/`) | out-of-scope twins |
| `last_area_move_card_id` / `_by_player` | 1379/1385 | `rb_gsc_last_area_move_card_id` / `_by_player` (0 callers) | routable |
| `last_energy_placed_by_{effect,player}` | 1393/1399 | `rb_gsc_last_energy_placed_by_{effect,player}` (0 callers) | routable |
| `last_cost_wait_member` | 1407 | `rb_gsc_last_cost_wait_member` game_state_core.c:462 (0 callers) | routable |
| `recently_moved_cards` / `set_recently_moved_cards` | 1415/1421 | `rb_gsc_recently_moved_cards` / `_set_` (0 callers; **0 Rust call sites**) | VERIFIED-NOOP |
| `first_attacker` / `second_attacker` | 1429/1435 | `rb_gsc_first_attacker_index` / `_second_` game_state_core.c:144/145 (0 callers) | routable |
| `assert_game_over` | 1461 | **NONE** — a Rust `debug_assert!` | N/A (debug only) |

---

## 4. FIELD-BY-FIELD TABLES

Three counts first, because they are the headline: **`Player` has 19 Rust fields
and 19 C members, but 5 of the 19 have NO member of their own**; `GameState` has
126 Rust fields and **14 have no C member at all**; `GameModifiers` has 36 Rust
fields and **13 have no C member**. Name-matching alone is meaningless here — the
C renames almost everything — so every row below is a semantic mapping read out
of `include/rabuka.h` and cross-checked against the writer/reader in `src/`.

### 4.1 `Player` (19 Rust fields, player.rs:160-213)

| # | Rust field | Rust line | C member | verdict |
|---|---|---|---|---|
| 1 | `id: PlayerId` | 163 | **ABSENT** | Seat-index addressing. `gsc_player_id(g, seat)` (game_state_core.c:127) synthesises the `"P1"`/`"P2"` string and `gsc_id_is` (:132) does the compare, so every `player.id == "p1"` reader in Rust has a C answer. **Flattened, faithful.** |
| 2 | `name: String` | 165 | **ABSENT** | Display/log label only. **0 readers in `src/`. VERIFIED-NOOP.** |
| 3 | `is_first_attacker: bool` | 167 | **ABSENT from `RbPlayer`** | C stores the resolved *seat*: `GameState.first_attacker` / `.second_attacker` (rabuka.h:1590/1591), read through `rb_gsc_first_attacker_index`. Semantically identical. **Flattened, faithful.** |
| 4 | `stage: Stage` | 169 | `RbPlayer.stage[3]`, `.stage_wait[3]`, `.under_cards[3]` (rabuka.h:761-763) | WIRED, **plus a C-only per-area `stage_wait`** Rust has no counterpart for (D4). |
| 5 | `live_card_zone: LiveCardZone` | 171 | `RbPlayer.live` (rabuka.h:767) | WIRED. **`LiveCardZone.face_up` (zones.rs:501) has no C member** — see §4.4. |
| 6 | `energy_zone: EnergyZone` | 173 | `.energy` + `.energy_active` (rabuka.h:764/766) | WIRED, split into bag + count. |
| 7 | `main_deck: MainDeck` | 175 | `.deck` (rabuka.h:760) | WIRED, renamed. |
| 8 | `energy_deck: EnergyDeck` | 177 | `.energy_deck` (rabuka.h:765) | WIRED. |
| 9 | `hand: Hand` | 179 | `.hand` (rabuka.h:759) | WIRED — but capped at `RB_MAX_HAND 40` where Rust's `SmallVec` is unbounded (pre-existing `test_audit_zones` red-pin). |
| 10 | `waitroom: Waitroom` | 181 | `.discard` (rabuka.h:769) | WIRED, renamed (`discard` **is** the waitroom). |
| 11 | `success_live_card_zone` | 183 | `.success` (rabuka.h:768) | WIRED, renamed. |
| 12 | `exclusion_zone: ExclusionZone` | 185 | **ABSENT from `RbPlayer`** | C has ONE global `GameState.exclusion_zone` (rabuka.h:1604) shared by both seats. **D13.** |
| 13 | `yell_from_bottom: bool` | 191 | `.yell_from_bottom` (rabuka.h:775) | WIRED. |
| 14 | `deployed_this_turn: SmallVec<[i16;4]>` | 200 | `GameState.stage_arrived[2][3]` (rabuka.h:1610) | WIRED but **per-AREA where Rust is per-CARD — D9.** |
| 15 | `stage_hearts: Option<BaseHeart>` | 202 | `GameState.stage_hearts[2][8]` (rabuka.h:1650) | WIRED. |
| 16 | `debut_count_this_turn: u8` | 204 | `GameState.debut_count_this_turn[2]` | WIRED. |
| 17 | `deck_refreshed_this_turn: bool` | 206 | `.deck_refreshed_this_turn` (rabuka.h:770) | WIRED. |
| 18 | `live_card_set_limit_reduction: u8` | 208 | `GameState.live_set_limit_reduction[2]` (rabuka.h:1460) | WIRED. |
| 19 | `last_resolution_cards: SmallVec<[i16;4]>` | 210 | `GameState.last_resolution_cards_p1` / `_p2` | WIRED, split per seat. |

**`Player`: 5 of 19 fields have no C member of their own — `id`, `name`,
`is_first_attacker`, `exclusion_zone`, and `deployed_this_turn`'s per-card
identity. Four of the five are flattened faithfully (identity is a seat index);
`exclusion_zone` is not (D13).**

### 4.2 `GameModifiers` (36 Rust fields, game_modifiers.rs:177-251 → `RbMods`, rabuka.h:228-273)

| Rust field | Rust line | C member | verdict |
|---|---|---|---|
| `blade_modifiers` | 178 | `blade[RB_MAX_CARD_IDS]` | WIRED (HashMap → dense array; absent ≡ zero, identical for every reader) |
| `blade_type_modifiers` | 179 | `blade_type[]` | WIRED |
| `heart_modifiers` | 180 | `heart[][8]` | WIRED |
| `heart_override` | 181 | `heart_color_override[]` + `heart_override_count[]` | WIRED |
| `orientation_modifiers` | 182 | `orientation[]` | WIRED |
| `cost_modifiers` | 183 | `cost[]` | WIRED |
| `score_modifiers` | 184 | `score[]` | WIRED |
| `need_heart_modifiers` | 185 | `need_heart[][]` | WIRED |
| `constant_blade_bonuses` | 186 | `constant_blade[]` | WIRED |
| `constant_cost_bonuses` | 187 | `constant_cost[]` | WIRED |
| `constant_cost_set_bonuses` | 189 | **not a member** — a function-local `expected_set[]` in modifiers.c:1249 | **D14** |
| `constant_score_bonuses` | 190 | `constant_score[]` | WIRED |
| `constant_heart_bonuses` | 191 | `constant_heart[][8]` | WIRED |
| `constant_global_need_heart` | 194 | **ABSENT** | `Vec<(i16,String,i16)>` global required-heart grant. **VERIFIED-NOOP: 0 of 985** — the corpus has no `modify_required_hearts_global`, re-measured 2026-10-03 (the generator only emits `modifier_type: "heart"`). |
| `p1_constant_total_score_bonus` | 197 | `p1_constant_total_score_bonus` | WIRED |
| `p2_constant_total_score_bonus` | 198 | `p2_constant_total_score_bonus` | WIRED |
| `constant_score_sources` | 201 | **ABSENT** | `Vec<BonusSource>` display ledger |
| `constant_blade_sources` | 205 | **ABSENT** | display ledger |
| `constant_heart_sources` | 206 | **ABSENT** | display ledger |
| `constant_cost_sources` | 207 | **ABSENT** | display ledger |
| `constant_need_heart_sources` | 208 | **ABSENT** | display ledger |
| `success_zone_blade_sources` | 211 | **ABSENT** | display ledger |
| `success_zone_heart_sources` | 212 | **ABSENT** | display ledger |
| `success_zone_score_sources` | 213 | **ABSENT** | display ledger |
| `heart_color_multiplier` | 214 | `heart_multiplier[]` + `heart_multiplier_amt[]` | WIRED (the Rust `Option<(HeartColor,u8)>` is split into presence+amount, exactly as `last_surplus_loss_count` is not) |
| `heart_copy` | 217 | `heart_copy[]` | WIRED |
| `last_cost_discard_count` | 219 | `last_cost_discard_count` | WIRED |
| `last_cost_moved_card_ids` | 221 | `last_cost_moved_card_ids[8]` + `n_` | WIRED (wider) |
| `opp_cause_fired_keys` | 225 | `opp_cause_fired_keys[16]` + `n_` | WIRED (wider) |
| `last_cost_energy_count` | 227 | **ABSENT** | **VERIFIED-NOOP: 0 of 985** — the intersection of `modify_cost` (4 abilities) and `energy_mode: "discard"` (1 ability) is **0**. Re-measured 2026-10-03: the single `discard`-mode cost ability is a hand cost, and the single `modify_cost` energy ability uses `energy_mode` "active"/"activate". |
| `last_under_move_host_ids` | 233 | `last_under_move_host_ids[4]` + `n_` | WIRED |
| `delayed_cannot_active` | 236 | `delayed_cannot_active[]` | WIRED |
| `last_surplus_loss_count` | 239 | **ABSENT from `RbMods`**; C has `GameState.last_surplus_loss_count[2]` (rabuka.h:1580) | **D15** |
| `success_zone_blade_bonuses` | 242 | **ABSENT** | **D16** |
| `success_zone_heart_bonuses` | 245 | **ABSENT** | **D16** |
| `success_zone_score_bonuses` | 248 | **ABSENT** | **D16** |

**`GameModifiers`: 13 of 36 fields have no C member — 8 are `Vec<BonusSource>`
display ledgers (0 gameplay reachability), 3 are the success-zone bonus ledgers
(D16), and 2 (`constant_cost_set_bonuses`, `last_surplus_loss_count`) are live
but stored somewhere else.**

### 4.3 `GameState` — the 126 fields, grouped

| Group | count | C member(s) | verdict |
|---|---|---|---|
| players | 2 | `RbPlayer p[2]` | WIRED |
| `ability_queue` | 1 | `RbAbilityQueue queue` | WIRED |
| `card_database` | 1 | process-global `g_cards` | WIRED (Arc → global) |
| `mods` | 1 | `RbMods mods` | WIRED |
| `resolution_zone` | 1 | `RbBag resolution` | WIRED, `owners` ABSENT (§1.5a) |
| `game_state_history` | 1 | `n_game_state_history` ring | WIRED |
| **`rule_log`** | 1 | **ABSENT** | **D7 — BLOCKED-ON rabuka.h** |
| **`structured_log`** | 1 | `rb_log_push` shared ring (`ability/log.c`) | WIRED but to a **different store**: the 32-slot verdict ring, not a `Vec<LogEntry>`. Overflow drops entries. |
| **`debug_trace`** | 1 | **ABSENT** | 0 of 985; the Rust doc says "debugging (tests, support dumps)". **VERIFIED-NOOP.** |
| turn-scalar (`turn_number`, `cheer_*`, `baton_touch_*`, `yell_*`, `last_*`) | 30 | `turn`, `cheer_blade_hearts[2]`, `cheer_checks_required`, `cheer_check_base`, `cheer_checks_done`, `yell_count_mod[2]`, `baton_touch_*`, `re_yell_*`, `last_wait_to_active_count`, `last_vacated_area`, `rps.*` | WIRED |
| per-turn card-id sets (`cards_appeared/activated/paid_cost/moved_this_turn`, `areas_placed_this_turn`, `auto_ability_trigger_counts`, `revealed*`, `looked_at_cards`) | 11 | `appeared_this_turn[]`, `cards_activated_this_turn[]`, `cards_paid_cost_this_turn[]`, `moved_this_turn[]`, `areas_placed_this_turn[]`, `auto_ability_trigger_counts[]`, `revealed[]`, `revealed_cost_cards[]`, `player1/2_cheer_revealed_cards[]` | WIRED, **except `looked_at_cards`, which has no C member** |
| `card_appearance_source` | 1 | `card_appearance_source_card[]`/`_zone[]` | WIRED, **capped at 32** (existing `test_audit_zones` red-pin) |
| movement-event logs (`batch_movements`, `turn_area_movements`, `position_change_events`, `recently_state_changed`, `turn_state_changes`) | 5 | `batch_movements[]`, `turn_area_movements[]`, `position_change_events[]`, `recently_state_changed[]`, `turn_state_changes[]` | WIRED |
| **`turn_movements`** | 1 | **ABSENT** | Rust's turn-level *all-zone* movement log (mod.rs:255, "used by movement conditions with source+destination zones", the `idoutsuuji` clause). C keeps only the per-batch `batch_movements`. **See D17.** |
| **`movement_event_counter`** | 1 | **ABSENT** | Rust assigns unique timestamps to `MovementEvent`s (mod.rs:257); C has no timestamp field. Diagnostic only. |
| **`state_snapshot_before_change`** | 1 | **ABSENT** | Rust's pre-`change_state` orientation snapshot (mod.rs:261). C has `state_change_from[]`/`_to[]`/`_triggering[]` — a *post*-hoc record, not a before/after diff. **D17.** |
| **`turn_state_changes`** | 1 | `turn_state_changes[]` + `n_` | WIRED |
| `recently_moved_cards` / `recently_appeared_cards` | 2 | `recently_moved[]` / `recently_appeared[]` | WIRED |
| `recently_moved_from_zone` | 1 | `move_src_zone[16]` | WIRED |
| **`deck_emptied_by_effect`** | 1 | **ABSENT** | **D17.** |
| `gained_abilities` / `constant_gained_abilities` / `gained_card_abilities` / `gained_ability_sources` | 4 | `gained_cards[]` + `n_gained_cards` (card IDs only) | **PARTIAL — D18.** Rust stores the ability *text* and, for `gained_card_abilities`, the full `Ability` structs; C stores IDs. |
| `delayed_gained_effects` | 1 | `n_delayed_gained_effects` + a parallel array | WIRED (flattened) |
| `prohibition_effects` / `delayed_prohibition_effects` / `non_stackable_effects` / `temporary_effects` | 4 | `prohibition[]`/`n_prohibition`, `prohibition_effects[]`/`n_`, `temp_effects[]`/`n_` | WIRED |
| `cannot_activate_members` / `constant_cannot_activate_members` / `cannot_live_players` | 3 | `cannot_active_cards[]`, `player_cannot_activate`, `opponent_cannot_activate` | WIRED |
| **`wait_immune_members`** | 1 | **ABSENT** | `Vec<(member_id, owner_id)>`, cleared at end of live. **D19.** |
| **`turn_limited_abilities_used`** | 1 | **ABSENT** | `HashMap<(i16, usize, u8), u8>` — the turn-limit counter for "N times this turn" abilities. **D20.** |
| `mulligan_selected_indices` / `live_card_selected_indices` | 2 | `mulligan_selecting`/`mulligan_done` are flags; `selected_cards[]`/`n_selected_cards` | WIRED |
| `play_time_cost_play` / `play_time_cost_reduction_accepted` | 2 | the `ptc_*` block (rabuka.h:1611-1619) | WIRED, flattened into 9 scalars |
| `ability_applications` | 1 | `RbMods.trace[]`/`n_trace` (relocated) | WIRED |
| `scratch_exp_*` (4) + `scratch_constant_effect_ids` | 5 | **ABSENT by design** | Rust's transient `mem::take` scratch buffers; C uses file-static scratch. Faithful. |
| `ability_invalidations` / `replacement_effects` / `constant_ability_statuses` | 3 | `ability_invalidations[]`/`n_`, `replacement_effects[]`/`n_`, **ABSENT** for the third | `constant_ability_statuses` **ABSENT — D21.** |
| `revealed_card_meta` / `revealed_cost_card_meta` | 2 | **ABSENT** | `RevealedCardMeta` carries the reveal's provenance (source zone / cost paid). Diagnostic only; 0 of 985 as a match target. |
| `initial_yell_revealed_cards` / `re_yell_revealed_cards` | 2 | `initial_yell_revealed[]`, `re_yell_revealed[]` | WIRED |
| `performance_snapshots` | 1 | `snapshots[]`/`n_snapshots` | WIRED |
| `last_ability_trace` | 1 | **ABSENT** | debug trace; 0 Rust call sites. **VERIFIED-NOOP.** |
| `pending_loop_protocol` / `loop_last_action` / `loop_detected` | 3 | `loop_detected`; the protocol pair **ABSENT** | **D22** |
| `pending_success_replacement_card_id` / `_player_id` | 2 | **ABSENT** | **D23** |
| `pending_rps_player_id` | 1 | `rps.*` (`web_server`-set) | WIRED |
| `activating_ability_index` | 1 | `active_ability_index` (via `rb_gsc_zone_snapshot` path) | WIRED |
| `just_completed_moved` / `this_batch_triggered_ability_ids` / `just_completed_ability_key` / `depth_first_cutoff` | 4 | `batch_triggered_keys[]` + `play_depth` | WIRED, flattened |

**`GameState`: 14 of 126 fields have no C member — `rule_log`,
`structured_log`-as-a-store, `debug_trace`, `looked_at_cards`, `turn_movements`,
`movement_event_counter`, `state_snapshot_before_change`, `deck_emptied_by_effect`,
`gained_abilities`, `constant_gained_abilities`, `gained_card_abilities`,
`gained_ability_sources`, `wait_immune_members`, `turn_limited_abilities_used`,
`constant_ability_statuses`, `last_ability_trace`, `pending_loop_protocol`,
`loop_last_action`, `pending_success_replacement_card_id`,
`pending_success_replacement_player_id`, `revealed_card_meta`,
`revealed_cost_card_meta`, `scratch_*`×5 (by design) — of which the ones with a
reachable consumer are `rule_log` (D7), `turn_movements` + `state_snapshot_before_change`
+ `deck_emptied_by_effect` (D17), `gained_abilities` (D18), `wait_immune_members`
(D19), `turn_limited_abilities_used` (D20), `constant_ability_statuses` (D21),
`loop_last_action` (D22), `pending_success_replacement_*` (D23).**

### 4.4 `LiveCardZone.face_up` — ABSENT
`zones.rs:501`, `pub face_up: bool`. Rust pushes `(card, face_up)` on every
`add_card` (zones.rs:523-535) and `calculate_live_score` **ignores** it
(zones.rs:549-593 has no `face_up` term). The only Rust readers are in
`core/game_state/modifiers.rs`'s success-zone and live-zone scans, which read
`live_card_zone.cards` directly, not `face_up`. **VERIFIED-NOOP: 0 of 985** — the
field is written and never read on this corpus.
**BLOCKED-ON (only if the display layer is ever ported): `include/rabuka.h` —
`uint8_t face_up;` inside `RbBag`-style `RbPlayer.live`… i.e. a parallel
`uint8_t live_face_up[RB_MAX_LIVE_CARDS];` beside `RbBag live;` (rabuka.h:767).**

---

## 5. REACHABILITY HISTOGRAM

Corpus: 985 abilities / 6032 interned strings (`cards/abilities.json`
`unique_abilities` == `RBKA_NUM_ABILITIES`; verified equal, not assumed).

| bucket | count | what it means |
|---|---|---|
| `n = 0` | **15** of the 29 gaps in §2 | `VERIFIED-NOOP`, recorded not ported |
| `n = 1..9` | 9 | small but real cards; fix |
| `n = 10..99` | 3 | ordinary card families; fix |
| `n >= 100` | 2 | pervasive; fix |

Gap-by-gap:

| gap | n (of 985) | verdict |
|---|---|---|
| D1 `rb_stage_first_empty` probe order | **25** reach a stage-placement arm (`stage` 10, `empty_area` 8, `same_area` 7) + every `turn/phase.c` baton default | **FIXED in `zones.c`** |
| D2 `check_effect_position` Japanese spellings | 0 | VERIFIED-NOOP |
| D3 `MemberArea` CamelCase spellings | 0 | VERIFIED-NOOP |
| D4 `total_blades`'s extra `stage_wait` source | 0 — and `rb_zone_total_blades` has **0 C callers at all** | VERIFIED-NOOP |
| D5 `EnergyZone::add_card` type check | 0 — `rb_energy_add_card`'s only production callers are `rb_setup_initial_energy` (phase.c:1386/1390), which draws from the energy deck | VERIFIED-NOOP |
| D6 `place_card_in_zone` empty destination | 0 (`destination: ""` never occurs; 906 absent destinations resolve to `hand` on both sides) | VERIFIED-NOOP |
| D7 `rule_log` absent | n/a — the field itself | **BLOCKED-ON rabuka.h** |
| D8 zero-count heart pairs | 0 of 2639 cards has a reader that can tell | VERIFIED-NOOP |
| D9 `deployed_this_turn` per-card vs per-area | **3 `play_baton_touch` × 29 `placement_change`** | see §6b — open, needs a reachability decision |
| D10 `move_card_from_hand_to_stage` argument loss | 0 — the twin has no production caller | VERIFIED-NOOP |
| D11 `clear_all_for_card` field set | n/a — verified **equal** | no divergence |
| D12 alloc counters see no traffic | 0 by construction (`alloc_tracker` feature) | VERIFIED-NOOP, permanent |
| D13 per-player `exclusion_zone` | 0 — `exclusion_zone` appears in **0** of the 2674 zone-valued corpus occurrences | VERIFIED-NOOP |
| D14 `constant_cost_set_bonuses` not a field | 1 (`modify_cost op=set`, ab#605) | cluster E's file; see §7 |
| D15 `last_surplus_loss_count` shape | see §7 | routable |
| D16 success-zone bonus ledgers | see §7 | routable |
| D17 `turn_movements` / `state_snapshot_before_change` / `deck_emptied_by_effect` | see §7 | routable |
| D18 `gained_abilities` text vs ids | `gain_ability` actions: **11** in the corpus | see §7 |
| D19 `wait_immune_members` | `cannot_wait_by_effect`: **0** in the corpus | VERIFIED-NOOP |
| D20 `turn_limited_abilities_used` | see §7 | routable |
| D21 `constant_ability_statuses` | see §7 | routable |
| D22 `loop_last_action` | see §7 | routable |
| D23 `pending_success_replacement_*` | see §7 | routable |

---

## 6. GAPS CLOSED

### 6a. D1 — `rb_stage_first_empty` probe order — **FIXED**

`engine_c_wip/src/core/zones.c:304-315` now reads

```c
int rb_stage_first_empty(const int stage[RB_STAGE_SIZE]){
    if(!stage) return -1;
    if(stage[1]==RB_EMPTY_SLOT) return 1;   /* Centre first (util.rs:2542) */
    if(stage[0]==RB_EMPTY_SLOT) return 0;
    if(stage[2]==RB_EMPTY_SLOT) return 2;
    return -1;                              /* None (util.rs:2549) */
}
```

replacing the ascending `for(i=0..2)` scan. The NULL guard is new: the old body
dereferenced the pointer unconditionally, and the A/B baseline below shows it
**segfaults** on a NULL stage array, which the new test exercises.

**Who else must stop disagreeing.** `src/ability/util.c:1104-1116` already had
the right answer in a private `rb_util_stage_first_empty_rust`, written with the
comment *"Divergence reported, not silently reconciled: **zones.c is not this
file**"*. That is now false, so the root is fixed and util.c's private copy is
redundant. It is **identical** to the new body, so nothing in util.c has to change
for correctness; util.c's owner can collapse it. Not edited — not my file.

**Two ascending probes remain, both in files I do not own, and both are the ones
the 25 corpus abilities actually reach:**

| site | what it does | owner |
|---|---|---|
| `src/ability/effects/draw.c:131` | `for(s=0;s<3 && !placed;s++) if(stage[s]==EMPTY)` — the `destination:"stage"` / `"empty_area"` placement | cluster F |
| `src/ability/util.c:1219` `rb_util_stage_first_empty_rust` | private duplicate, already correct | cluster F |

`rb_place_card_in_zone`'s own `stage`/`empty_area` arm (util.c:1143) and
`turn/phase.c:1361`'s baton default both call the fixed helper.

### 6b. File-isolated A/B

Baseline = the same snapshot with **only** the `rb_stage_first_empty` body
reverted, not git HEAD. Proved by hash sweep and by an explicit rebuild:

| stage | `src/core/zones.c` sha256[:16] | `zones.o` mtime | `zones.o` sha256[:16] |
|---|---|---|---|
| pre-fix | `A7EBC37A43C1CE0C` | 16:45:07 → **17:04:29** | `C1716121F6B0D5F1` |
| post-fix | `956CCD50138D5013` | → **17:05:21** | `D56BEA93F2D2006C` |

The object mtime moved on both rebuilds, so neither measurement is a stale link.

`tests/test_audit_zones.c` (mine, claimed):

| run | FAIL_ANY | new-case assertions passing |
|---|---|---|
| pre-fix (`A7EBC37A…`) | **9** | 2 of 9 |
| post-fix (`956CCD50…`) | **3** (the 3 pre-existing red-pins) | **9 of 9** |

The 6 pre-fix failures are, verbatim from `out/audit_zones.err`:

```
FAIL: an empty stage answers CENTRE (util.rs:2542 `stage[1] == -1 -> Some(1)`), not LEFT: the C scanned 0,1,2
FAIL: stage_first_empty_is_centre_first: fatal SIGSEGV (11) in child after 0 assertion(s)
FAIL: stage with no vacated area lands in the CENTRE (util.rs:2447 + :2542); left is still empty (left=130)
FAIL: stage: no other stage slot is touched
FAIL: empty_area with no vacated area lands in the CENTRE (util.rs:2447 + :2542); left is still empty (left=130)
FAIL: empty_area: no other stage slot is touched
```

`left=130` is the old engine putting the card in the **left** slot when Rust puts
it in the **centre**. The SIGSEGV is the old body's missing NULL guard.

### 6c. Regression sweep — one assertion moved, and it is a TEST bug

| suite | FAIL_ANY before | FAIL_ANY after |
|---|---|---|
| `audit_zones` | 3 | **3** (+17 new passing assertions, 9/9) |
| `zone_filter_parity` | 0 | 0 |
| `mechanics` | 0 | 0 |
| `parity_zones_position` | 38 | 38 |
| `parity_mill_deck` | 82 | 82 |
| `parity_draw_flat` | 74 | 74 |
| `parity_energy_remainder` | 7 | 7 |
| `parity_energy_position_delta` | 10 | 10 |
| `parity_integration` | 31 | 31 |
| `parity_sequential` | 142 | 142 |
| `parity_under_member` | 91 | 91 |
| `position_and_source` | 0 | 0 |
| `parity_wave1a` / `1b` / `1c` | 38 / 53 / 44 | 38 / 53 / 44 |
| `parity_small_clusters` | 111 | 111 |
| `parity_cce_gaps` | 92 | 92 |
| `parity_full_round` / `full_round3` | 45 / 18 | 45 / 18 |
| **`parity_full_round2`** | **68** | **69** |
| `parity_live_total` / `live_total2` | 53 / 10 | 53 / 10 |
| `parity_recover_live` | 57 | 57 |
| `parity_jidou_watch` / `_clusters` / `_move_yell` / `_life` | 79 / 62 / 88 / 31 | not re-run; unaffected by any `zones.c` path |
| `parity_hearts_live_start` / `required_hearts` / `parity_restriction` / `restriction_mechanics` | 64 / 28 / 62 / 0 | not re-run; unaffected |

Exactly **one** line moved, isolated by a sorted `FAIL`-set diff of both runs:

```
only in AFTER: FAIL [N3_sync_opponent_gets_choice]: N3: the card P2 selected is at P2 left (got -1 expected 2642)
only in BEFORE: (none)
```

`parity_full_round2::N3_sync_opponent_gets_choice` was **already red with 6 failures**
before this change (the opponent routing and the missing `SelectPosition` prompt);
this change adds the 7th. **I did not edit that file** — it is not mine, and the fix
belongs to its owner.

**Traced to the exact call, in the snapshot only** (a temporary `fprintf` in
`util.c`'s stage arm, reverted; `util.c` sha256[:16] back to `89730B86DBDB73EC`,
matching the repo, and `util.o` rebuilt from scratch after the revert because the
first rebuild left the mtime unmoved):

```
TRACE rb_place_card_in_zone pl=1 card=2642 zone=empty_area vacated=-1 -> area=1 stage=[-1 -1 -1]
[DISCARD_MOVE] selected=2642 of 1 available=2 moved=1 destination=empty_area state_change=- seat=1
```

So the call is `place_card_in_zone(card, Zone::EmptyArea, vacated_stage_area=None)`
against an **entirely empty** stage. Rust's arm for that exact input,
`ability/util.rs:2442-2456`:

```rust
Some(Zone::Stage) | Some(Zone::EmptyArea) => {
    ...
    let pos = if is_max && empty_slots.len() < count { None }
              else { stage_first_empty(&player.stage.stage) };   // util.rs:2447
```

and `stage_first_empty([-1,-1,-1])` is `Some(1)` — the **CENTRE**
(`util.rs:2542`, `if stage[1] == -1 { Some(1) }`). There is no reading of Rust that
puts this card on the left.

**Conclusion: the assertion `"N3: the card P2 selected is at P2 left"`
(`tests/test_parity_full_round2.c:780`) encodes the C's ascending scan, not Rust's
centre-first probe. It should read `centre`. Recommended to that file's owner with
the Rust line cited; deliberately not applied here.**

### 6d. What this fix does NOT reach

Eleven suites are byte-identical because the paths the 25 corpus abilities take are
not `rb_stage_first_empty`:

| site | what it does | owner |
|---|---|---|
| `src/ability/effects/draw.c:131` | `for(s=0;s<3 && !placed;s++) if(stage[s]==EMPTY)` — `destination:"stage"` / `"empty_area"` via `draw_place_in_zone_g` | cluster F |
| `src/ability/util.c:1111` `rb_util_stage_first_empty_rust` | private duplicate, already correct | cluster F |
| `src/ability/effects/move.c:1846-1857` | `empty_slots[]`/`avail[]` built ascending, then a `SelectPosition` prompt — the path N3 takes | cluster F |

`rb_place_card_in_zone`'s own `stage`/`empty_area` arm (util.c:1143) and
`turn/phase.c:1361`'s baton default both call the fixed helper, and the N3 trace
above shows the `empty_area` route reaching it.

---

## 7. THE 47 `rb_gsc_*` ROUTABILITY VERDICT

**Headline: `rule_log` still has no field, so NOTHING became newly routable from
that change, and the answer is 0 of 47.** `RB_GSC_RULE_LOG_MAX 500` exists as a
`#define` at `include/rabuka.h:3489`, but `GameState` (rabuka.h:1448-1770) has no
`rule_log` member, so `rb_gsc_push_rule_log` still routes into
`rb_log_push_verdict(text, "rule_log", 1)` — the shared **32-slot**
condition-verdict ring, which drops on overflow. The header's own comment at
rabuka.h:3484-3488 and the file banner at game_state_core.c:36-45 both forbid
exactly this. The nine logging functions therefore remain unroutable.

**The real blocker is broader than `rule_log`, and it is not the fields — it is
the declarations.** `include/rabuka.h` declares exactly **ONE** of the 53
`rb_gsc_*` symbols: `rb_gsc_zone_snapshot` at rabuka.h:3265 (`rb_gsc_current_turn_phase`
is only *mentioned* in a comment at rabuka.h:1321). A function with no prototype
cannot be called from another translation unit, which is why 45 of the 47 are
unroutable **even where the data they need already exists.**

Of the 6 that do have external callers, every one got there by hand-writing a
local `extern` — the single documented example is `game/game_setup.c:208`:
`extern int rb_gsc_concede(GameState *g, int player_id);`.

Split of the 47, by what it would take to route each:

| bucket | n | what is missing | example |
|---|---|---|---|
| **DEAD IN BOTH** — 0 in-scope Rust call sites | **9** | nothing; no wiring can change behaviour | `rb_gsc_zone_snapshot`, `rb_gsc_print_trace_node`, `rb_gsc_recently_moved_cards`, `rb_gsc_set_recently_moved_cards`, `rb_gsc_current_turn_phase`, `rb_gsc_game_has_ended`, `rb_gsc_concede_both`, `rb_gsc_seated_pair` (40 Rust call sites, **all** in `bot/**`+`bin/**`), `rb_gsc_opponent_has_performed` (2, both `bot/**`) |
| **HEADER-BLOCKED ONLY** — live Rust twin, data exists, needs a prototype | **34** | one line in `include/rabuka.h` | `rb_gsc_push_rule_log_fmt` (61 Rust call sites), `rb_gsc_active_player` (35), `rb_gsc_record_ability_application` (18, already wired via live.c:1537), `rb_gsc_can_player_act` (10), `rb_gsc_mulligan_owner_index` (9, already wired), `rb_gsc_current_ability_source_card_id` (10, already wired), … |
| **FIELD + HEADER blocked** — the logging group | **4** | the `rule_log` member *and* a prototype | `rb_gsc_push_rule_log_fmt` (61), `rb_gsc_log_entry` (3), `rb_gsc_push_choice_offered` (1), `rb_gsc_push_choice_resolved` (1) |

Reachability histogram over the 53 twins, counting only **in-scope** Rust call
sites (`engine/src/bot/**` and `engine/src/bin/**` excluded — PROGRESS.md puts
them explicitly out of scope):

| in-scope Rust call sites | twins |
|---|---|
| 0 | 9 |
| 1–4 | 28 |
| 5–19 | 9 |
| 20+ | 2 |

**Concrete request, one block in `rabuka.h`, 45 declarations.** Every prototype
below is already implemented and compiling in `game_state_core.c`; the missing
piece is only a declaration. Names and signatures are verbatim from that file.

<details><summary>the 45 missing prototypes (click to expand)</summary>

```c
/* ---- seat identity (game_state_core.c:144-180) ---- */
int         rb_gsc_first_attacker_index(const GameState *g);
int         rb_gsc_second_attacker_index(const GameState *g);
int         rb_gsc_active_player_index(const GameState *g);
RbPlayer   *rb_gsc_active_player(GameState *g);
const RbPlayer *rb_gsc_const_active_player(const GameState *g);
const RbPlayer *rb_gsc_seat_player(const GameState *g, int seat);
RbPlayer   *rb_gsc_seat_player_mut(GameState *g, int seat);
const RbPlayer *rb_gsc_seated_pair(const GameState *g, int me, const RbPlayer **out_opp);
const RbPlayer *rb_gsc_try_player_by_id(const GameState *g, const char *id);
RbPlayer   *rb_gsc_try_player_by_id_mut(GameState *g, const char *id);
int         rb_gsc_seat_index_by_id(const GameState *g, const char *id);

/* ---- turn / phase (game_state_core.c:185-238) ---- */
int         rb_gsc_is_first_turn(const GameState *g);
RbGscTurnPhase rb_gsc_current_turn_phase(const GameState *g);
int         rb_gsc_phase_invariant(const GameState *g);
int         rb_gsc_can_player_act(GameState *g, int seat);
int         rb_gsc_opponent_has_performed(const GameState *g, int seat);
int         rb_gsc_game_has_ended(const GameState *g);
int         rb_gsc_reject_if_ended(GameState *g);

/* ---- concede ---- */
int  rb_gsc_concede(GameState *g, int player_id);
int  rb_gsc_concede_both(GameState *g);

/* ---- scoring / cheer by id (game_state_core.c:413-438) ---- */
int  rb_gsc_cheer_count_by_id(GameState *g, const char *id, int v, int add);
int  rb_gsc_set_cheer_count_by_id(GameState *g, const char *id, int v);
int  rb_gsc_seat_cheer_count(const GameState *g, int seat);
int  rb_gsc_score_bonus_by_id(GameState *g, const char *id, int v);
int  rb_gsc_seat_score_bonus(const GameState *g, int seat);

/* ---- per-turn effect tracking (game_state_core.c:440-503) ---- */
void rb_gsc_clear_effect_tracking(GameState *g);
int  rb_gsc_last_area_move_card_id(const GameState *g, int seat);
int  rb_gsc_last_area_move_by_player(const GameState *g, int seat);
int  rb_gsc_last_cost_wait_member(const GameState *g);
int  rb_gsc_last_energy_placed_by_effect(const GameState *g, int seat);
int  rb_gsc_last_energy_placed_by_player(const GameState *g, int seat);
int  rb_gsc_current_ability_source_card_id(const GameState *g);
int  rb_gsc_push_revealed_card(GameState *g, const char *zone, int card_id);
int  rb_gsc_player_prefix_id(const GameState *g);

/* ---- snapshots / zones / choices (game_state_core.c:529-711) ---- */
int  rb_gsc_zone_is_private(const char *zone);
int  rb_gsc_resolve_index_any_player(GameState *g, const char *zone, int idx, int *out);
int  rb_gsc_choice_offered_labels(const GameState *g, const RbChoice *ch,
                                  char *out, int out_sz);
int  rb_gsc_choice_option_count(const RbChoice *ch);

/* ---- logging: these four ALSO need the rule_log member (see below) ---- */
void rb_gsc_push_rule_log_fmt(GameState *g, const char *fmt, ...);
void rb_gsc_log_entry(GameState *g, const char *text, const char *player_label,
                      int source_card_id, const char *effect_type);
void rb_gsc_push_choice_offered(GameState *g, const RbChoice *ch);
void rb_gsc_push_choice_resolved(const GameState *g, const RbChoice *ch,
                                 int chosen_index, int cost_paid);

/* ---- snapshots / traces (game_state_core.c:778-848) ---- */
void rb_gsc_push_performance_snapshot(GameState *g, int player_index);
void rb_gsc_record_ability_application(GameState *g, int source_card_id,
                                        const char *ability_text,
                                        const char *effect_type,
                                        int target_card_id, int amount);
void rb_gsc_print_trace_node(const RbTraceNode *node, int indent);
```
</details>

And the field the logging group needs, appended at rabuka.h:1769:

```c
    /* Rust GameState.rule_log: Vec<String> (core/game_state/mod.rs:118),
     * bounded by LOG_BOUND_RULE = 500, newest-kept (mod.rs:1602-1606). */
    char     rule_log[RB_GSC_RULE_LOG_MAX][RB_GSC_RULE_LOG_LINE_MAX];
    uint16_t rule_log_len[RB_GSC_RULE_LOG_MAX];
    int      n_rule_log;
```
plus `#define RB_GSC_RULE_LOG_LINE_MAX 256` beside `RB_GSC_RULE_LOG_MAX` at
rabuka.h:3489.

---

## 8. WHAT I CLAIMED AND WHAT I DID NOT TOUCH

**Test files claimed by this audit:** `tests/test_audit_zones.c` (two new cases:
`stage_first_empty_is_centre_first`, `place_stage_lands_in_the_centre` — 9 new
assertions). Nothing else in `tests/`.

**C files edited:** `src/core/zones.c` — one function body
(`rb_stage_first_empty`), lines 304-315. `sha256[:16]`
`651534B542DF6D10` → `956CCD50138D5013`.

**C files read but not edited:** `game_state_core.c`, `data.c`,
`stats_pipeline.c`, `tracking.c`, `alloc.c`, `alloc_counter.c` — no gap found in
any of them that is both reachable and mine to close.

**Damage I caused and repaired, recorded because it is a trap:** a
`Get-Content -Raw | Set-Content -Encoding utf8` round-trip through Windows
PowerShell 5.1 on `tests/test_audit_zones.c` double-encoded one pre-existing
UTF-8 Japanese literal in a **comment** (line 1285) and prepended a UTF-8 BOM.
Both were detected by a byte scan (24 non-ASCII bytes + `EF BB BF` at offset 0),
the literal was replaced with `\xNN` escapes per this repo's ASCII-only rule,
and the BOM was stripped. The file is now **74 499 bytes, 0 non-ASCII**, first
three bytes `2F 2A 20`. The load-bearing `"\xE3\x82\xB9..."` escape on the next
line was never touched. **Never rewrite a source file through PowerShell
`Get-Content`/`Set-Content` on this repo.**

---




