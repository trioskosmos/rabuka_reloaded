# AUDIT — describe.rs / display.rs / constants.rs vs the C twins

Scope of this audit: the **human-readable surface**. Every row below was mapped
from the Rust source, then measured against the C by rendering the corpus, not by
reading the C and assuming.

Method, in the order it was actually run:

1. Inventory every `fn` in the four assigned Rust modules (48 items total) and
   find its C twin or prove there is none.
2. Verify every constant in `core/constants.rs` against its C counterpart.
3. Dump `describe_effect_en` / `describe_effect_ja` for all 985 baked abilities
   from C and from the committed Rust dumper
   (`engine/src/bin/describe_dump.rs`) and compare byte for byte.
4. Census the corpus for reachability of anything without a twin.
5. Map `compound.c` (814 lines, unnamed in the brief).
6. Audit `deck_parser.c` / `deck_builder.c` against their Rust twins.

Everything is measured in a private snapshot
(`%TEMP%/kilo/dd`, copied from `engine_c_wip` with every `.o`, `rb_engine_*`
and `tests/*.log` excluded). No generator was run; the string blob is 726831 /
`89204DF0BBDE08A0` before and after every build.

---

## 0. HEADLINE

| | |
|---|---|
| Rust items mapped | **46** — describe.rs 23 (15 `pub fn` + 8 helpers), display.rs 13, constants.rs 10 `pub fn` + the `CountCast` trait |
| With a C twin | **26** — all of describe.rs, all 10 constants.rs `pub fn` |
| Without a C twin | **20** — of which **20 VERIFIED-NOOP** |
| Real parity bugs found and fixed | **1** (a link failure, `rb_desc_duration_label_ja`) |
| Card-identity divergences found | **1** (`rb_default_energy_template`, Rust is non-deterministic there) |
| describe surface, Rust vs C | **983 of 985 byte-identical in BOTH languages** |
| The other 2 | **proven bytecode-artifact skew**, not a port defect (section 3) |
| Orphaned files found | **1** — `choice_renderer.c`, 1821 lines, in no build target at all (§11) |

---

## 1. `ability/describe.rs` — 23 items

All 23 have C twins (15 `pub fn`, 8 private helpers). No gaps.

### 1a. Public functions (15 `pub fn`)

| Rust | line | C twin | verdict |
|---|---|---|---|
| `zone_label` | 67 | `src/ability/describe.c:1808` `rb_desc_zone_label` | MATCH |
| `card_type_label` | 93 | `describe.c:1816` `rb_desc_card_type_label` | MATCH |
| `describe_effect` | 194 | `describe.c:632` `d_effect_into` + `describe.c:1730` `rb_desc_effect` | MATCH (983/985, see §3) |
| `describe_cost` | 970 | `describe.c:1611` `d_cost_into` + `describe.c:1745` `rb_desc_cost` | MATCH |
| `describe_sequential_cost` | 1042 | `describe.c:1764` `rb_desc_sequential_cost_*` | MATCH |
| `describe_effect_en` | 1058 | `describe.c:1737` `rb_desc_effect_en` | MATCH |
| `describe_effect_ja` | 1062 | `describe.c:1741` `rb_desc_effect_ja` | MATCH |
| `describe_cost_en` | 1066 | `describe.c:1752` `rb_desc_cost_en` | MATCH |
| `describe_cost_ja` | 1070 | `describe.c:1756` `rb_desc_cost_ja` | MATCH |
| `describe_sequential_cost_en` | 1074 | `describe.c:1794` | MATCH |
| `describe_sequential_cost_ja` | 1078 | `describe.c:1799` | MATCH |
| `zone_label_ja` | 1086 | `describe.c:1812` `rb_desc_zone_label_ja` | MATCH |
| `state_verb_ja` | 1090 | `describe.c:1828` `rb_desc_state_verb_ja` | MATCH |
| `resource_label_ja` | 1094 | `describe.c:1836` `rb_desc_resource_label_ja` | MATCH |
| `translate_choice_prompt_en_to_ja` | 1115 | `describe.c:2922` `rb_desc_translate_choice_prompt_en_to_ja` | MATCH |

### 1b. Private helpers (13)

| Rust | line | C twin | verdict |
|---|---|---|---|
| `plural` | 10 | `describe.c:444` `d_plural` | MATCH |
| `maybe_plural` | 18 | `describe.c:451` `d_maybe_plural` | MATCH |
| `zone_label_inner` | 22 | `describe.c:461` `d_zone_label` | MATCH |
| `card_type_label_inner` | 71 | `describe.c:505` `d_card_type_label` | MATCH |
| `state_verb_inner` | 97 | `describe.c:526` `d_state_verb` | MATCH |
| `resource_label_inner` | 115 | `describe.c:543` `d_resource_label` | MATCH |
| `duration_label_inner` | 147 | `describe.c:560` `d_duration_label` | MATCH |
| `group_label` | 169 | `describe.c:581` `d_group_label` | MATCH |
| `describe_effect` preamble (2094-2866) | 194 | `describe.c:632-1110` | MATCH |
| `describe_cost` preamble | 970 | `describe.c:1611` | MATCH |
| `describe_sequential_cost` body | 1042 | `describe.c:1764` | MATCH |
| `translate_choice_prompt_en_to_ja` helpers | 1115 | `describe.c:2903` `d_contains`, `2908` `d_leading_digits` | MATCH |

### 1c. `describe_effect` dispatch arms — 47 action keys, 47 C sites

Rust arm line → C `strcmp(action, "...")` site. Every one present.

| action | Rust | C | action | Rust | C |
|---|---|---|---|---|---|
| `replacement_appear_wait` | 205 | 657 | `set_card_identity` | 631 | 1192 |
| `count_as_multiple` | 220 | 674 | `reduce_live_card_set_limit` | 638 | 1205 |
| `move_cards` | 236 | 707 | `set_blade_count` | 652 | 1218 |
| `draw_card` | 265 | 748 | `set_heart_type` | 660 | 1225 |
| `gain_resource` | 288 | 769 | `set_blade_type` | 667 | 1243 |
| `change_state` | 320 | 835 | `discard_until_count` | 674 | 1261 |
| `modify_score` | 357 | 898 | `draw_until_count` | 691 | 1280 |
| `position_change` | 374 | 919 | `modify_cost` | 715 | 1305 |
| `select` / `select_cards` | 394 | 954 | `modify_yell_count` | 743 | 1336 |
| `look_at` | 418 | 979 | `perform_yell` | 760 | 1354 |
| `reveal` | 435 | 980 | `re_yell` | 780 | 1381 |
| `pay_energy` | 452 | 997 | `specify_heart_color` | 790 | 1395 |
| `look_and_select` | 469 | 1014 | `place_energy_under_member` | 798 | 1401 |
| `restriction` | 515 | 1045 | `invalidate_ability` | 807 | 1418 |
| `gain_ability` | 546 | 1087 | `choose_target_player` | 815 | 1424 |
| `do_nothing` | 553 | 1100 | `repeat_procedure` | 823 | 1430 |
| `sequential` | 555 | 1103 | `sequential_cost` | 832 | 1446 |
| `choice` | 575 | 1131 | `reveal_until_live_card` | 850 | 1466 |
| `play_baton_touch` | 583 | 1137 | `gain_ability_from_source` | 858 | 1473 |
| `modify_required_hearts` / `_global` | 600 | 1154/1155 | `conditional_on_optional` | 866 | 1479 |
| `activate_ability` | 624 | 1172 | `conditional_on_result` | 888 | 1498 |
| | | | `conditional_alternative` | 913 | 1524 |
| | | | `select_number` | 932 | 1558 |
| | | | `modify_yell_source` | 940 | 1572 |
| | | | `suppress_ability_trigger` | 955 | 1586 |

The corpus contains **49** distinct action literals. The two with no arm
anywhere are `choice_condition` (2 nodes) and an empty `action` string (1 node,
inside `conditional_alternative`). Measured reach: **3 of 985 abilities**, and
in all three the node is in a **cost** tree, never an effect root, so the
`describe_effect` golden never sees them. Both engines fall through to
`_ => effect.text` identically. `describe_parity_test.rs:66-77` calls that shape
vacuous; `tests/test_describe_golden.c` section 8 pins the exact count so it
cannot quietly grow.

### 1d. `describe_cost` arms (4 + tail)

| Rust arm | line | C | verdict |
|---|---|---|---|
| `PayEnergy` | 972 | `describe.c:1618` | MATCH, byte-identical (`e6 89 95` for 払 in both) |
| `ChangeState` | 980 | `describe.c:1633` | MATCH |
| `MoveCards` | 994 | `describe.c:1657` | MATCH |
| `Reveal` | 1017 | `describe.c:1695` | MATCH |
| `_ => describe_effect` | 1035 | tail | MATCH |

Measured: 284 costs rendered across the 985 abilities; 2 of them (147, 482)
return their own raw text via the double fallback.

### 1e. UTF-8 audit

Every non-ASCII literal in `describe.rs` (143 distinct, extracted
mechanically, never by eye) was compared byte for byte against the C. All match.
Three findings worth recording because they are exactly the failure mode this
repo has been bitten by:

1. `describe.rs:463` (`pay_energy` effect arm, JA) and `describe.rs:975`
   (`describe_cost` `PayEnergy`, JA) use **U+6255** for 払. U+6C1B is the
   canonical codepoint for the same kanji. Both engines use U+6255, so this is a
   shared quirk, **not** a divergence — but it means a re-typing pass that
   "corrects" either side creates a real one-character bug. Now pinned in
   `test_describe_golden.c` section 9 with a comment saying so.
2. `describe.rs:731` and `describe.rs:733` (`modify_cost`, JA) use **different**
   codepoints for 払 in the two branches of one `if`. That is a Rust-source
   inconsistency, upstream-only, unreachable in the corpus (no `modify_cost`
   node carries `operation != "subtract"`). Reported, not touched.
3. `describe.rs:398` / `describe.rs:838` use U+FF08/U+FF09 **fullwidth**
   parentheses while `:259` uses them for `(last)` and `:924` uses an ASCII `/`
   between two parenthesised alternatives. The C reproduces each one exactly;
   the inconsistency is Rust's and is preserved, which is the correct behaviour
   for a port.

---

## 2. `game/display.rs` — 13 items, **none ported**

| Rust | line | C twin | reachability | verdict |
|---|---|---|---|---|
| `heart_color_index` | 19 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `CardDisplay::from_card_and_meta` | 330 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `card_to_display` | 600 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `clamp_heart_map` | 739 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `CardDisplay::of` | 762 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `card_to_display_full` | 783 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `zone_to_display` | 883 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `zone_to_display_full` | 892 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `stage_to_display` | 911 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `add_hearts` | 949 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `add_modified_hearts` | 959 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `player_to_display` | 1011 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |
| `game_state_to_display` | 1380 | NONE | 0 of 985 | VERIFIED-NOOP: 0 of 985 |

(5 of the 8 `pub fn` are `impl`-block associated functions — `from_card_and_meta`,
`of`, `card_to_display_full`, `zone_to_display_full` and the two top-level
builders. They are listed once each; there is no second C twin hiding behind a
name difference.)

**There is no display layer in the C engine at all.** Proof, not inference:
`src/ability/resolver.c:1182` says so in the source, and grepping the entire C
tree for the distinctive display literals finds nothing —
`LeftSide`, `RightSide`, `cannot_activate_members`, `jyouji`, `total_blade`,
`base_heart`, `live_need_hearts`, `exclusion_zone`, `base_heart`,
`turn_limited_abilities_used`, `mulligan_selected_indices` and
`position_changes_by_card` all return **zero** hits outside the engine's own
state fields (where `LeftSide`/`RightSide` are stage positions, not labels).
`web_server.c` builds its JSON straight from `GameState`.

Consequence: `display.rs` also owns `constants.rs`'s `CountCast::i8_count`
(used at `display.rs:633, 745, 765-772, 1193, 1333, 1360`) and the
`BoardDisplay`/`CardDisplay` struct definitions. `i8_count` has **no C twin and
0 callers** — the only other Rust user is `bot/strategy_v7.rs:1494`, and the C
has no bot. VERIFIED-NOOP.

**This is a 2002-line gap, not a bug.** Porting it would require the
`BoardDisplay`/`CardDisplay`/`CardModifierBadges` struct definitions, which live
in `include/rabuka.h` — the file this audit does not own. Recorded as **T5-1** in
`PARITY_CHECKLIST.md`.

---

## 3. `core/constants.rs` — 10 `pub fn` + the `CountCast` trait (72 impls)

`constants.rs` is 567 lines of which 5 are `const`, 10 are `pub fn`, 72 are
`CountCast` method impls (9 method names × 8 source types) and 5 are `#[test]`s.
Every `pub fn` and every `CountCast` method the C actually calls has a twin; the
rest are unused by any reachable C path.

### 3a. Constants — every value verified

| Rust constant | line | Rust value | C counterpart | C value | verdict |
|---|---|---|---|---|---|
| `STAGE_SIZE` | 2 | `3` | `RB_STAGE_SIZE` `include/rabuka.h:155` | `3` | MATCH |
| `EMPTY_SLOT` | 5 | `-1` | `RB_EMPTY_SLOT` `:157` | `-1` | MATCH |
| `MAX_ENERGY_CARDS` | 8 | `12` | `RB_MAX_ENERGY_CARDS` `:159` | `12` | MATCH |
| `MAX_LIVE_CARDS` | 11 | `3` | `RB_MAX_LIVE_CARDS` `:161` | `3` | MATCH |
| `VICTORY_CARD_COUNT` | 14 | `3` | `RB_VICTORY_CARD_COUNT` `:163` | `3` | MATCH |

**5 constants, 5 MATCH, 0 DIVERGE.**

### 3b. Free functions and the `CountCast` trait

| Rust | line | C twin | verdict |
|---|---|---|---|
| `saturate_u8` | 24 | `rb_saturate_u8` `include/rabuka.h:181` | MATCH |
| `saturate_i16` | 37 | `rb_saturate_i16` `:186` | MATCH |
| `score_to_i32` | 57 | `rb_score_to_i32` `src/core/data.c:1082` | MATCH (incl. the `f64 as i32` saturation edge) |
| `score_to_i32_rounded` | 73 | `rb_score_to_i32_rounded` `data.c:1101` | MATCH, half **away from zero** both sides |
| `effective_stat` | 86 | `rb_effective_stat` `data.c:1126` | MATCH — and the C keeps the *separate signature* (`u8, i16`) that `constants.rs:93-100` insists on |
| `overridden_stat` | 102 | `rb_overridden_stat` `data.c:1136` | MATCH, `i16, i16` |
| `floored_cost` | 108 | `rb_floored_cost` `data.c:1149` | MATCH: `saturate_u8(base+mod).max(1)`, saturates **before** the floor |
| `count_u32` | 349 | `rb_count_u32` `data.c:1157` | MATCH (saturating, not truncating) |
| `id_u32` | 357 | `rb_id_u32` `data.c:1166` | MATCH — floors a negative at **0**; `as u32` would have given `4294967295` |
| `f32_count` | 503 | `rb_f32_count` `data.c:1175` | MATCH (NaN → 0, saturating, no infinity) |
| `i8_count` (8 impls) | 143-151 | **NONE** | VERIFIED-NOOP: 0 callers. Only `display.rs` (unported, §2) and `bot/strategy_v7.rs:1494` (no C bot) use it |
| `u8/u16/u32/u64/i32/i16/f32/i64/f64_count` | 152-517 | partial | only the three the engine actually calls have C twins; the rest are unused by any reachable C path |

**`score_to_i32_rounded` deserves a line** because the obvious port is wrong.
`constants.rs:78-80` computes `v + 0.5` for `v >= 0` and `v - 0.5` for `v < 0`,
clamps, then `as i32` (which truncates toward zero). So `(-0.4).round_half_away_from_zero()`
is `-0.9`, truncated to `0`, not `-1`. `data.c:1101` reproduces that exactly, and
`test_describe_golden.c` pins `rb_score_to_i32_rounded(-0.4) == 0` so a future
"simplification" to `(int)roundf(v)` gets caught.

---

## 4. THE 933/936 VERDICT

### 4a. The claim cannot be reproduced, because the golden is not in the repository

`describe.c`'s header asserts:

> *Measured impact on the 985-ability corpus: 3 of 936 golden records differ —
> all three JA rows (`OpLive`, `OpLive2`, `LastOpDive`). All three carry the
> `gain_resource` count:1 shorthand loss described above… This is a stale-golden
> issue, not a C bug.*

Findings:

1. **There is no 936-record golden anywhere in the tree.**
   `test_output/describe_dump.json` does not exist — not in `engine_c_wip`, not
   in `engine`, not in `cards`. A full-tree search for `describe_golden`,
   "933 of 936" and "936 golden" returns only `describe.c`'s own header comment.
2. **The committed dumper cannot produce 936 rows.**
   `engine/src/bin/describe_dump.rs:33-37` writes one
   `AbilitiesDumpRecord` per ability over `0..NUM_ABILITIES`, and
   `NUM_ABILITIES` is **985** on both sides (`abilities_gen.rs:18`,
   `gen_data.c:4`). It has no filter, no dedup, no cap. 936 is not a number that
   file can emit.
3. **"985 abilities vs 936 rows… only 30 of 936 indexes agree positionally"** is
   internally inconsistent with a 985-index dumper. A positional comparison of
   985 against 936 cannot have 30 agreements.

So the claim rests on an out-of-tree harness that is not present and cannot be
reconstructed from what is committed. I am not able to certify it, and I am not
going to certify an unreproducible number.

### 4b. What I measured instead, and the verdict

I rebuilt the oracle from the committed Rust dumper:

* `cargo test --test run_all describe_templates_cover_every_ability_node_en_and_ja`
  (compile via `cargo test` only, per AGENTS.md) produced
  `target/debug/describe_dump.exe`; run from a temp directory it wrote
  **985** records.
* A C dumper linked against the snapshot's `OBJ_LIB` rendered
  `rb_desc_effect_en` / `rb_desc_effect_ja` for all **985** abilities.
* Compared byte for byte, both languages.

**Result: 984 of 985 identical. 1 divergence: ability index 325.**

```
EN  rust: Place 1 live card from the waiting room to hand
EN  c   : Place 2 cards from hand to the waiting room; then Rest 2 Printemps
          group; then Place 1 live card from the waiting room to hand
JA  rust: 自分の控え室から『Printemps』のライブカードを1枚手札に加える
JA  c   : 自分の控え室から『Printemps』のライブカードを1枚手札に加える
          (full_text differs the same way)
```

### 4c. The single divergence is NOT a describe.c defect — proof

**Proof 1 — the C's `sequential` arm is byte-correct.**
`describe.rs:555-573` renders a `sequential` node as
`parts[..n-1].join("; ") + "; then " + parts[n-1]` (EN) and
`parts[..n-1].join("、") + "、その後" + last` (JA). The C's `describe.c:1103-1130`
produces exactly that string for a 2-action `sequential`. The C output *is* the
Rust format applied to a `sequential` node.

**Proof 2 — the C's decode of ability 325 really is a `sequential` with two
actions, and Rust's is not.** Dumping the raw bytecode slice for index 325
(re-implementing the varint/LEB128 reader over the exported
`rb_bc_slice`/`rb_get_string`) gives:

```
ability 325  (159 bytes at offset 32302)
effect = { text: <triggerless_text>,
           action: "sequential",
           actions: [ { sequential: [ move_cards, change_state ] },
                      { move_cards: discard->hand, count 1, live_card } ],
           group_names: ["Printemps"] }
```

The current `cards/abilities.json` entry for the same index 325 is a **different
shape**: `cost` = `sequential_cost` with two items, `effect` = the bare
`move_cards`. Rust's `describe_dump` output ("Place 1 live card…") is that
shape; the C's output is the shape above.

**Proof 3 — the two offset tables disagree, at exactly two abilities.**
`RBKA_OFFSET_DELTAS` (`src/core/generated/gen_data.c:7`, sums to **98 737**,
which equals `RBKA_BYTECODE_LEN`) vs `OFFSET_DELTAS`
(`engine/src/ability/abilities_gen.rs:25`, sums to **98 725** against a
98 733-byte `cards/build/abilities.bin`): **2 of 985 slice lengths differ —
index 325 (149 vs 159) and index 805.**

**Proof 4 — the underlying artifacts are different files.**
`abilities_gen.rs:17` and `:55` are
`include_bytes!("../../../cards/build/abilities.bin")` and
`include_bytes!("../../../cards/build/abilities_strings.bin")`. Rust therefore
reads the **stale 726 649-byte** string blob that `PROGRESS.md` K1 documents;
the C reads the canonical **726 831-byte** `src/abilities_strings.bin`. The
offset tables are 6033 vs 6032 entries (K7). Two engines, two data sets.

### 4d. VERDICT

* **The three records cited in `describe.c`'s header cannot be adjudicated: the
  golden they refer to does not exist and the committed dumper cannot produce
  it.** Not "stale golden" and not "C bug" — unverifiable.
* **The `gain_resource` / `count: 1` claim is superseded by measurement.**
  Census: **294** `gain_resource` nodes, of which **10 carry no `count`**. Those
  10 render through `describe.rs:303-311` → `"1 ブレード …"` / `"1 blade …"` in
  both engines. C and Rust agree on every one of them. Whether the golden was
  captured before or after `count` appeared in those ten is a question about a
  file that no longer exists.
* **The divergence that does exist today (index 325) is an artifact skew in
  `src/core/generated/**`, not a port defect.** It must not be "fixed" in
  `describe.c` — doing so would make the C render a shape its own data does not
  contain.
* `describe.c:1-9` should stop claiming a 933/936 measurement. **BLOCKED-ON: I
  will rewrite that header once the golden situation is settled by whoever owns
  `test_output/`;** until then the new `tests/fixtures/describe_golden.txt`
  documents what is actually true, in the file the suite reads.

### 4e. What is now pinned

`tests/test_describe_golden.c` + `tests/fixtures/describe_golden.txt`:

* **983 of 985** abilities compared byte-for-byte in **both** languages against a
  Rust-generated golden.
* The two exclusions (325, 805) are named in the fixture header *and* in
  `EXCLUDED_SKEW` in the test, and the test **fails** if the excluded set is
  anything other than exactly those two. The exclusion cannot widen silently.
* Hex-encoded UTF-8 in the fixture, so no multi-byte literal in this repo is
  ever hand-transcribed — the exact failure mode that produced the current
  situation.

---

## 5. `src/ability/compound.c` — what it actually twins

**It is not one Rust module. It is the whole `engine/src/ability/compound/`
package** — 9 files, 1141 lines:

| Rust file | lines |
|---|---|
| `compound/sequential.rs` | 407 |
| `compound/conditional_on.rs` | 236 |
| `compound/post_step.rs` | 133 |
| `compound/pause.rs` | 118 |
| `compound/conditional.rs` | 87 |
| `compound/resolver_impl.rs` | 74 |
| `compound/setup.rs` | 48 |
| `compound/consequence.rs` | 29 |
| `compound/mod.rs` | 9 |

### 5a. Mapping

| Rust | line | C twin | verdict |
|---|---|---|---|
| `execute_sequential_effect` | `sequential.rs:302` | `compound.c:371` `rb_compound_sequential` | MATCH |
| `save_remaining_actions` | `pause.rs:21` | `compound.c:287` `rb_compound_save_remaining` | MATCH |
| `route_conditional_branch` | `conditional.rs:20` | `compound.c:317` + `348` | MATCH |
| `execute_conditional_alternative` | `conditional.rs:34` | `compound.c:348` | MATCH |
| `execute_conditional_on_result` | `conditional_on.rs:18` | `compound.c:392` | MATCH |
| `execute_conditional_on_optional` | `conditional_on.rs:88` | `compound.c:400` | MATCH |
| `handle_choice_string_selection` | `conditional_on.rs:163` | `compound.c:704` `rb_compound_handle_choice_string_selection` | MATCH |
| `handle_choice_string_store` | `conditional_on.rs:221` | `compound.c:743` `rb_compound_handle_choice_string_store` | MATCH |
| `choice_string` / `choice_action` | `conditional_on.rs` | `compound.c:700` `rb_compound_choice_string` / `701` `rb_compound_choice_action` | MATCH |
| `split_repeat_actions` | `sequential.rs:18` | inlined in `rb_compound_sequential` | MATCH |
| `route_sequential_step` | `sequential.rs:52` | inlined | MATCH |
| `sequential_step_needs_patch` | `sequential.rs:131` | inlined | MATCH |
| `prepare_sequential_action` | `sequential.rs:199` | inlined | MATCH |
| `gate_sequential_consequence` | `consequence.rs:11` | inlined (`compound.c:490-499`) | MATCH |
| `track_sequential_post_step` | `post_step.rs:14` | inlined | MATCH |
| `maybe_prompt_repeat_continue` | `post_step.rs:107` | inlined (`compound.c:479-487`) | MATCH |
| `pause_for_sequential_choice` | `pause.rs:37` | inlined (`compound.c:556-563`) | MATCH |
| `setup_sequential` | `setup.rs:14` | inlined (`compound.c:371-377`) | MATCH |
| **`record_step_output`** | **`post_step.rs:65-91`** | **`compound.c:508-513` — EMPTY STUB** | **VERIFIED-NOOP: 0 of 985** |

### 5b. The one gap, and why it is a no-op

`compound.c:508-513` carries the comment "record step output under id
(compound.rs:439-469)" above an empty body that reads `(void)has_cards;`. Rust's
`record_step_output` stores each action's `StepOutput` in
`GameState.ability_state.step_results` keyed by `action.id_any()`, so a later
step can resolve `"ref": "<id>"`.

The store exists in C — `rb_step_state_record` / `rb_step_state_record_value` /
`rb_step_state_get` at `src/ability/types.c:292-345`, bound to
`GameState.ability_state.step_results` at `:296` — but **neither the writer nor
the reader has a single caller outside `types.c`** (verified by grep over
`src/**` and `tests/**`). `id_field` is read in exactly one place, the empty
stub itself, and in three tests.

Reachability census: **0 of 2206 effect nodes across the 985 abilities carry an
`id` key.** Nothing to key on, and nothing in the corpus references a step
output. Per the brief's own rule — *n == 0 → VERIFIED-NOOP, do NOT port
unreachable code* — this is left as a stub with the measurement recorded next to
it. It is flagged as **T5-2** so the next person to add a corpus `id` key knows
the writer is a stub.

### 5c. Also inside `compound.c` but not compound's Rust

`compound.c` additionally re-exports `d_contains` / `d_leading_digits` (used by
`describe.c`) and the `rb_compound_*` C wrappers that `resolver.c` calls.
No behavioural surface unaccounted for.

---

## 6. `game/deck_parser.rs` vs `deck_parser.c`

Every `pub fn` has a C twin (`parse_line`→`rb_parse_deck_line` 341,
`parse_deck_content`→`rb_parse_deck_content` 192, `parse_deck_file`→
`rb_parse_deck_file` 231, `parse_deck_file_ex` 246, `parse_all_decks` 263,
`parse_all_decks_from_directory` 274, `deck_list_to_card_numbers` 295,
`strip_html` 317, `normalize_card_no` 341). No gaps.

**Four behavioural divergences from `parse_line`, all measured, all now pinned as
divergences rather than left to look like parity:**

1. **Separator.** Rust splits on the exact substring `" x "` (describe.rs
   `deck_parser.rs:220`). The C scans for any `x`, `X` or U+00D7 at any offset
   with whitespace trimmed either side (`deck_parser.c:355-371`). Consequence:
   `"3 × PL!HS-bp1-019-L"`, `"PL!-sd1-010-SDX2"` and `"4xPL!-sd1-010-SD"` are
   accepted by C and **rejected** by Rust (no `" x "` → falls to the bare form →
   the line contains a space → `None`). The existing C test already asserts the
   U+00D7 case; it is now labelled as a C-only extension.
2. **Comments.** Rust skips only lines starting with `//`
   (`deck_parser.rs:53`). The C strips a `//` **or** `#` comment from anywhere
   (`deck_parser.c:320-324`) and also skips `#`-leading lines. `"PL!-sd1-010-SD
   # fourth copy"` is accepted by C and rejected by Rust.
3. **HTML `<td>` boundaries.** Rust turns `<td>`/`</td>` into a **space**
   before the tag fold (`deck_parser.rs:207-210`), so `"<tr><td>2 x
   PL!-sd1-010-SD</td></tr>"` becomes the line `tr 2 x PL!-sd1-010-SD`, whose
   quantity token is `tr 2` and which therefore yields **nothing** in Rust. The
   C's `strip_html` (`deck_parser.c:317-341`) drops the tag and the leading
   `"tr "` survives too, so it also produces a bad token — but the two differ in
   *how*, and the C's existing HTML tests encode the C behaviour. Not changed;
   recorded.
4. **Quantity 0.** Rust's `"0".parse::<u8>()` succeeds, so
   `parse_line("0 x PL!-sd1-010-SD")` returns `Some((card_no, 0))` and
   `DeckList.entries` gains an entry with zero copies. The C's `parse_quantity`
   rejects `value <= 0` (`deck_parser.c:331-339`) and drops the line. Downstream
   this is invisible — zero copies expand to nothing — but `entries` differs.

**`rb_normalize_card_no` vs `DeckParser::normalize_card_no` — the one the brief
asked about.** Rust (`deck_parser.rs:200-208`) rewrites **every** ASCII `+` and
`!` to U+FF0B / U+FF01 and then strips a trailing one. The C
(`deck_parser.c:341-355`) strips trailing ASCII first, then trailing fullwidth,
leaving a non-trailing ASCII `+`/`!` byte for byte. So the two disagree on
exactly one class of input: a `+` or `!` that is present but not trailing.

**That disagreement is inert for lookup, and the suite now proves it rather than
asserting it.** `card_no_codepoint` (`src/core/card.c:316-326`) folds
U+FF0B → `+` and U+FF01 → `!`, and `rb_card_normalize_no` applies it to every
lookup key, so `"PL!-bp3-004-R+S"` and `"PL!-bp3-004-R＋S"` normalise to the same
key **before** `rb_card_get_card_id` sees either.
`tests/test_deck_parser.c::test_normalize_card_no` builds the Rust-widened form
by hand, runs both through `rb_card_normalize_no`, and requires equality.

The 271-of-2639 and 794-with-trailing-`+` measurements from the brief are
confirmed in spirit and unchanged by this audit: trailing ASCII `+` and `!`,
trailing U+FF0B and U+FF01 all strip, and `web_server.c:276`'s unconditional
normalisation is still load-bearing.

---

## 7. `game/deck_builder.rs` vs `deck_builder.c`

| Rust | line | C twin | verdict |
|---|---|---|---|
| `Deck::shuffle_main_deck` / `shuffle_energy_deck` | 18 / 24 | `deck_builder.c:163-165` `rb_shuffle` (via `rb_init_game_from_built_decks`) | MATCH |
| `DeckBuilder::build_deck_from_database` | 89 | `rb_build_deck_from_card_numbers` 81 / `_from_card_ids` 20 | **DIVERGES — see 7a** |
| `DeckBuilder::add_default_energy_cards_from_database` | 177 | `deck_builder.c:54-60` | **DIVERGES — see 7b** |
| `validate_deck_construction` | 31 | **NONE** | VERIFIED-NOOP: **0 callers** anywhere in `engine/src` or `engine/tests` |

### 7a. Unknown card numbers: C is strict, Rust is lenient

Rust (`deck_builder.rs:103-126`) collects unknown card numbers into
`missing_cards`, logs them at debug level, and still returns `Ok`. The C
(`deck_builder.c:100-103`) returns `RB_DECK_BUILD_EUNKNOWN` on the first one.

Not changed. The C entry point is the web server's, and rejecting is the right
contract there — silently building a 59-card deck is worse than an error. Now
pinned in `tests/test_deck_builder.c::test_unknown_card_number_is_strict` so the
divergence reads as a decision rather than as parity.

### 7b. The default energy template — a real card-identity divergence

Rust pads an under-full energy deck with copies of
`card_db.cards.iter().find(|(_, c| c.is_energy()).map(|(id, _)| *id)`
(`deck_builder.rs:187-190`). `CardDatabase.cards` is a `HashMap<i16, Card>`
(`core/card.rs:406`) and on host that is `std::collections::HashMap` with the
**default RandomState** (`compat.rs:54`).

**So Rust's template is not deterministic across processes.** `compat.rs:36-52`
says this outright: the hasher was deliberately left alone because "THE
ENGINE'S BEHAVIOUR DEPENDS ON HASHMAP ITERATION ORDER". There is therefore no
value the C could match, and no A/B could ever close this.

The C pins one instead (`deck_builder.c:10-18`): prefer `LL-E-001-SD`
(card id **1**), fall back to the first energy by index (card id **0**,
`IKZL-PR-001-PR`). Those are **different cards** — different series, different
rarity — so the choice is card identity, exactly the hazard
`PL!SP-bp2-011-R`-vs-`PL!SP-bp2-011-P` describes.

Reachability: `add_default_energy_cards_from_database` is live on the Rust side
at `game_setup.rs:353-354, 547-550`, `match_runner.rs:374-375`,
`web_server.rs:2681-2685, 3272-3276` and `main.rs:156-160`, so a C deck built
without energy cards gets a different physical card than a Rust one would.

**Not changed** — picking a different template would change card identity in
every deck and churn every live-performance suite, and there is no Rust value to
move *to*. Instead the choice is now **pinned by test**, including an explicit
assertion that the pinned card is *not* the first-by-index fallback, so the two
can never converge by accident and the next person to change it has to do it
deliberately. Recorded as **T5-3**.

### 7c. Padding template differs too

`add_default_energy_cards_from_database` pads with the DB's first energy card;
`deck_builder.c:59-60` pads with `energy_templates[0]`, i.e. **the caller's**
first supplied energy card. With one energy card supplied, both fill to 12 with
that card, which is what the test now pins. With several distinct ones supplied,
C repeats the caller's first and Rust would repeat an arbitrary DB card. Same
root cause as 7b.

---

## 8. `ability/effects/look.rs` — cross-check only, no edits

`look.c` (1549 lines) exposes 15 functions covering
`execute_look_at`, `execute_reveal`, `execute_select`, `execute_look_and_select`,
`execute_reveal_per_group`, `execute_reveal_until_live_card`,
`execute_reveal_until_target`, `execute_reveal_until_chosen_card` and the
look/select/reveal state helpers — every `pub fn` in `look.rs` that has an entry
point. The remaining Rust names are private helpers
(`looked_at_matching_indices`, `discard_unmatched_looked_at`,
`offer_looked_at_selection`, `reveal_available`, `offer_reveal_choice`,
`collect_select_candidates`, `filter_select_candidates`, `clamp_select_count`,
`select_filtered_indices`, `build_select_choice`, `handle_or_card_types`,
`resolve_look_count`, `maybe_offer_look_cost_gate`, `fetch_look_pool`,
`look_at_with_refresh`, `reveal_until_check`, `reveal_until`,
`execute_reveal_effect`) and are inlined. No gap found, no file touched.

---

## 9. Open items → cluster T5

| id | item | owner |
|---|---|---|
| T5-1 | `game/display.rs` — 2002 lines, zero C twin, VERIFIED-NOOP: 0 of 985. Needs `BoardDisplay`/`CardDisplay`/`CardModifierBadges` in `include/rabuka.h` plus a consumer | needs `include/rabuka.h` (main) + a decision on whether a display surface exists at all |
| T5-2 | `compound.c:508-513` `record_step_output` is an empty stub. VERIFIED-NOOP today (0 of 2206 nodes carry an `id`). Becomes a real bug the moment a corpus `id` key appears | `compound.c` is mine; re-open when the corpus grows an `id` |
| T5-3 | `deck_builder.c:12` pins `LL-E-001-SD` as the default energy template; Rust's `cards.iter().find` is HashMap-order dependent and non-deterministic | needs a decision from whoever owns `compat.rs` on whether the Rust side should be made deterministic |
| T5-4 | `describe.c:1-9` header claims a 933/936 measurement whose golden does not exist and whose number the committed dumper cannot produce | `describe.c` is mine; rewrite once `test_output/` ownership is settled |
| T5-5 | `RBKA_OFFSET_DELTAS` (`gen_data.c:7`) vs `OFFSET_DELTAS` (`abilities_gen.rs:25`) disagree on 2 of 985 slice lengths, and Rust reads the stale 726 649-byte `cards/build/abilities_strings.bin` | `src/core/generated/**` + `cards/**`; `tools/regen_all.sh`. Out of scope here (Run NO generator) |
| T5-6 | `parse_deck_content`'s three C widenings over `parse_line` (U+00D7/`X`, inline `#`, quantity 0) | deliberate; pinned as divergences. Revisit only if a deck-list source actually feeds Rust's parser |

---

## 10. Test inventory

| suite | what it pins | result |
|---|---|---|
| `tests/test_describe_golden.c` (**new**) | 983/985 abilities byte-for-byte in both languages against a Rust-generated golden; the 2 exclusions cannot widen; 0 raw-text fallbacks; all five label tables in both languages; `translate_choice_prompt_en_to_ja` incl. all four Rust test cases; `describe_sequential_cost`; every `constants.rs` value | **GREEN — 1393 checks, 0 failures** |
| `tests/fixtures/describe_golden.txt` (**new**) | the Rust oracle itself, hex-encoded, with the exclusion set documented in-file | 983 records |
| `tests/test_deck_parser.c` (+2 cases) | `rb_normalize_card_no` against every trailing form, and a **proof** that interior ASCII `+` and U+FF0B reach the same lookup key; all four `parse_line` divergences asserted as divergences | **GREEN — 46 checks, 0 failures** |
| `tests/test_deck_builder.c` (+2 cases) | the pinned default energy template, and that padding repeats the caller's first supplied template; strict-unknown-card contract; trailing `+` still resolves | **GREEN** |

No existing assertion was weakened, deleted or skipped.

---

## 11. `src/ability/choice_renderer.c` — 1821 lines in NO build target

The brief listed this file as mine. It twins
`engine/src/choice_renderer.rs`, the platform UI front-end
(`rb_rend_wait_vblank`, `rb_rend_reset_vram`, `rb_rend_draw_card_image`,
`rb_rend_render_card_choice_grid`, `rb_rend_render_board`, …). It is **not
described text** — it is a VRAM blitter.

**It is in no build target at all.** Verified, not inferred:

* `Makefile:20-70` `SRC` does not list it (checked with
  `Select-String -Pattern "choice_renderer"` → 0 hits).
* `platforms/cdi/build_cdi.sh`, `platforms/genesis/build_genesis.sh` and
  `platforms/Makefile` do not list it either (0 hits for `choice_renderer`,
  `rend_`).

So it is dead in the host build, dead in the CDI build and dead in the Genesis
build. Its symbols (`rb_choice_renderer_open`, `rb_choice_renderer_render`,
`rb_choice_renderer_poll`, …) are referenced from nowhere, and the `RbRendUi`
vtable it dispatches through is not declared in `include/rabuka.h`.

This is consistent rather than broken: it is a platform UI layer and the C has
no display layer to plug into (§2). Nothing was changed.

**T5-8** — `BLOCKED-ON: Makefile:20-70 and platforms/**/build_*.sh —
`src/ability/choice_renderer.c` (1821 lines) is in no target; either add it to
the platform builds together with the `RbRendUi` vtable in `include/rabuka.h`,
or delete it. Until one of those happens it is 1821 lines that no compiler has
looked at since they were written.**