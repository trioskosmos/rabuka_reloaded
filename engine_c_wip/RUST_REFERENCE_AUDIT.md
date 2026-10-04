# RUST_REFERENCE_AUDIT — findings against the oracle

**HEAD read: `cf315a9a`** (`chore: parity checklist update`). Parent `2130769d`, grandparent `f152a50f`.
The brief's ordering "`f152a50f` → `cf315a9a` → `2130769d`" is inverted: `cf315a9a` is HEAD and `2130769d` is its **parent**.

Every finding below is tagged:
- **[WT]** = read from the working tree
- **[HEAD]** = read from committed HEAD via `git cat-file` / `git show`

No Rust file was edited. No generator was run. Nothing was built (no `.o` / `rb_engine*` created; nothing was
compiled at all — every finding below is established by reading source and by arithmetic over the committed
binaries). No git writes were performed. Scratch file removed; no files added to the tree by this audit.

**Headline: the premise "Rust is the authoritative semantic reference" is false in at least three independent
places, and the strongest single finding is that Q213 — the bug that started this audit — was never actually
tested in Rust. The one Rust test named for it asserts a tautology.**

---

## Rank 1 — Q213 has ZERO effective Rust coverage, and the one test named for it cannot fail

`engine/tests/test_modules/jidou/discard_watch/live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs`
**[WT]** — 63 lines, 1 `#[test]`.

The file name claims *"live_start heart requirement reduction counts live slot cards only"*. The doc comment
(lines 1-3) claims it proves Q213: members set as live are moved to the waitroom before `live_start`, so they do
not reduce the required heart. **It asserts nothing about hearts.**

| line | code | defect |
|---|---|---|
| 59-62 | `assert!(after_waitroom >= before_waitroom, "Non-live cards removed from live slot")` | **Tautology.** `>=` is satisfied by zero cards added. Passes identically whether the member was correctly evicted or never evicted at all. The comment on line 58 ("at least 1 card added") states a condition the assertion does not test. |
| 1-3, whole file | — | No read of `mods.get_need_heart_modifier`, `need_heart`, or any heart requirement. The card's entire stated purpose is unasserted. |
| 46-50 | `// The member card should be in waitroom (or should it?)` / `// Actually set_live_card would fail for member cards because` / `// But set_live_card just puts cards in the zone.` | The author's own recorded uncertainty about whether the premise holds. The test proceeds on an unverified assumption. |
| 6-10 | `for _ in 0..5 { game.pass(); }` | Blind phase counting — the exact anti-pattern `engine/tests/helpers/mod.rs` (`advance_to_phase` docstring) warns against: *"a blind count works only while the phase sequence is frozen: the moment a phase gains or loses a step, the test is standing in a different window than it thinks."* Q213 **is** a phase-ordering defect. |

**The complementary half is also missing.** The other Q213-related file,
`engine/tests/test_modules/effects/cost_mod/required_hearts/hanamusubi_required_hearts_exclude_self_test.rs`
**[WT]**, has 9 tests — all substantive, all asserting real `get_need_heart_modifier` values. But **every single
one uses live cards as the "other 蓮ノ空 card"** (`PL!HS-bp5-017-L`, `PL!HS-bp5-018-L`, more copies of
`PL!HS-bp5-019-L`, `PL!-sd1-019-SD`). **Not one stages a 蓮ノ空 *member* in the live slot** — the only case
Q213 adjudicates.

**Net effect: the two halves of Q213 are each tested by the file that structurally cannot observe it.**
- "member cards don't reduce the heart" → untested (only the waitroom tautology).
- "live cards do reduce the heart" → tested 9×, but only for live cards.

**C inherits the defect.** Not by porting a bug — by there being nothing to port. `engine_c_wip/src/turn/phase.c:1658`
sets `g->live_face_up[performer] = 1` then `rb_check_timing(g)` at 1663, reproducing Rust's ordering inside the
performance phase, which is itself downstream of `trigger_live_start_abilities`. The C is faithful; the oracle
was silent.

---

## Rank 2 — Q213 root cause in Rust source: `face_up` gate defers eviction one phase past `live_start`

**Q213 ruling** [WT], `cards/cards.json` → `PL!HS-bp5-019-L` (ハナムスビ), FAQ Q213 (2026.02.28):

> 「いいえ、{{live_start.png|ライブ開始時}}能力が発動する前にメンバーカードは控え室に移動します。そのため、メンバーカードでは減りません。」

**Card ability** [WT] `cards/cards.json` `PL!HS-bp5-019-L`:

> 「{{live_start.png|ライブ開始時}}自分のライブカード置き場にあるこのカード以外の『蓮ノ空』のカード1枚につき、このカードの必要ハートをheart04×2減らす。」

**The chain, all [HEAD] = [WT] (`engine/src` is clean, see Rank 8):**

1. `engine/src/core/zones.rs:501` — `LiveCardZone.face_up` is **one flag for the whole zone**, not per card.
   `zones.rs:514` — initialised `false`.
2. `engine/src/turn/phases.rs:832` and `:887` — set back to `false` by every `set_live_card` /
   live-card confirmation.
3. `engine/src/turn/phases.rs:343` — set to `true`, **only for the performer, only inside
   `execute_performance_phase`**.
4. `engine/src/turn/actions/mod.rs:1734` — `check_invalid_live_cards` early-returns unless that seat's
   `face_up` is true. Called for **both** seats from `check_timing` at `actions/mod.rs:1653-1654`.
5. `engine/src/ability/condition/card.rs:4419-4421` — the counting arm:
   `Some(Zone::LiveCardZone) => self.count_group_cards_in_cards(&player.live_card_zone.cards, group_filter, ct, exc)`
   — **no `is_live()` filter**. Any card in the live slot counts toward the group total.

**Phase order in `engine/src/turn/phases.rs` (`Phase::LiveCardSetSecondAttacker` arm):**

| line | call | `face_up` | effect |
|---|---|---|---|
| 240 | `Self::check_timing(game_state)` | `false` for both | eviction **skipped** for both seats; members stay in the live slot |
| 242 | `current_phase = FirstAttackerPerformance` | | |
| 245-246 | `trigger_live_start_abilities(first)` / `(second)` | `false` for both | **Q213's window opens here — members are still in `live_card_zone.cards` and still counted** |
| 247-253 | `process_pending_auto_abilities` | | |
| 343 | `performer.live_card_zone.face_up = true` | `true`, performer only | |
| 349 | `Self::check_timing(game_state)` | | eviction runs, **performer only** |

**Two consequences, both violating the ruling:**

- **The first attacker's member cards reduce ハナムスビ's required heart by 4** (Q213 says 0). The C inherits this.
- **The second attacker's member cards survive the entire first attacker's performance**, including both
  `trigger_live_start_abilities` calls, and are only evicted at their own `phases.rs:349`. Q213 says they are
  in the waitroom before *either* live_start window.

**Related, and a documentation defect that will propagate:** `engine_c_wip/src/turn/phase.c:1651-1654` diagnoses
this wrongly:

> *"Writing it after the check_timing below is the bug this comment exists to prevent — it leaves the eviction to
> the NEXT timing pass, which is one phase later and not the same board, so a non-live card forced into the live
> slot survives ライブ開始時 (measured: tests/test_parity_jidou_watch.c Q213 ハナムスビ)."*

In Rust the flag **is** written before `check_timing` (`phases.rs:343` then `:349`), so statement order inside
the performance phase is not the defect. The defect is that `execute_performance_phase` runs *after*
`trigger_live_start_abilities`. An agent reading that comment will "fix" the wrong thing.

---

## Rank 3 — Q68 violated on 3 cards: `live_start` fires for a 「ライブできない」 player

**Ruling** [WT] `cards/cards.json`, FAQ Q68 (2025.02.07), on **`PL!HS-bp2-014-N`, `PL!SP-bp1-001-P`,
`PL!SP-bp1-001-R`** (identical text on all three):

> 「『ライブできない』状態のプレイヤーは、ライブカードセットフェイズでライブカード置き場に手札のカードを裏向きで置くことはできますが、パフォーマンスフェイズで表向きにしたカードの中にライブカードがあったとしても、そのライブカードを含めて控え室に置きます。その結果、ライブカード置き場にライブカードが置かれていないため、ライブは行われません。（{{live_start.png|ライブ開始時}}の能力は使えず、エールも行いません）」

Three requirements: (a) live cards **also** go to the waitroom; (b) **live_start abilities are unusable**;
(c) **no yell**.

**What Rust does** [HEAD] = [WT]:

- `engine/src/turn/trigger_scan.rs:186-212` — `trigger_live_start_abilities` gates on
  `is_trigger_suppressed` (195) and, for the second attacker only, on `player.live_card_zone.cards.is_empty()`
  (207-212). **It never consults `cannot_live_players`.** Confirmed by full-tree grep: `cannot_live_players` has
  17 references; `trigger_scan.rs` has none.
- `engine/src/turn/phases.rs:351` — `cannot_live` is first read **after** `check_timing` (349) and after
  `trigger_live_start_abilities` (245-246).
- `engine/src/turn/phases.rs:380` — `cannot_live` is consumed in the **yell** phase (Phase A).
- `engine/src/turn/phases.rs:411-415` — the live cards are moved to the waitroom **during the yell**, i.e. after
  `live_start` already fired; the comment calls this "the cannot_live path".

**So for a ライブできない player who set cards: Rust fires their `live_start` abilities (zone non-empty at
245-246), then discards the live cards during the yell.** (b) and (c) are violated, and (a) happens one phase
later than the ruling's ordering. The `cards.is_empty()` proxy at trigger_scan.rs:209 is also evaluated before
the member eviction, so it cannot stand in for the ruling's condition either.

**C inherits the defect.** This is Q213-class: `cards.json` disagrees with the Rust implementation on three
cards, and nothing in the Rust suite adjudicates it.

---

## Rank 4 — Every module excluded from the Rust build

`engine/build.rs:90` — the **entire** exclusion list is one entry:

```rust
const EXCLUDED_MODULES: &[&str] = &["pvp_room_test"];
```

(The doc comment at `build.rs:87` calls it `EXCLUDED_FILES`; the const is `EXCLUDED_MODULES`. Naming only.)

### 4a. `pvp_room_test` — 14 tests, 345 lines, never compiled. Sole oracle for `choice_player_id` routing.

`engine/tests/test_modules/integration/pvp_room_test.rs` **[WT]**. 14 `#[test]` fns, 345 lines.
`build.rs:131-133` skips it, so `integration/mod.rs` declares only `full_round`, `multi_card`, `parser_e2e`,
`per_card`.

**Why it is excluded — established, not guessed:** line 6 imports
`rabuka_engine::web_server::pvp_player_can_act`, but `engine/src/lib.rs:149-150` gates the module:

```rust
#[cfg(feature = "server")]
pub use game::web_server;
```

`server` is **not** in `Cargo.toml:62` `default`. The function under test is
`engine/src/game/web_server.rs:1151`. **The file cannot compile with default features** — the exclusion is a
build necessity, not a choice, and nothing records that.

**What is lost — the only coverage of opponent choice routing:**

| lines | test | covers |
|---|---|---|
| 127-147 | `opponent_choice_routed_via_choice_player_id_works_in_pvp` | `choice.choice_player_id = Some("p2")` routes the choice to P2 and blocks P1 |
| 150-173 | `opponent_choice_via_select_auto_ability_works_in_pvp` | `Choice::SelectAutoAbility { player_id: "p2" }` via `pause_for_auto_ability_choice` |
| 176-193 | `normal_choice_stays_with_active_player_in_pvp` | no-routing defaults to the active player |
| 204-345 | `both_players_multiple_live_start_abilities_get_correct_choice_routing` | 3× Shizuku each; asserts `SelectAutoAbility.player_id` `"p1"` then `"p2"`, `options.len() == 3`, **and** that `get_pending_choice_json()["choice_player_id"]` is `"p1"`/`"p2"` — i.e. the JSON wire contract |
| 28-124 | 10 tests | `pvp_player_can_act` across every phase (RPS, ChooseFirstAttacker, both Mulligans, Main, both LiveCardSet) |

**Cost:** `choice_player_id` — the field `pvp_room_test.rs:140` writes directly onto
`gs.ability_queue.current_entry_mut()` — has **zero** compiled coverage. The JSON field name
`"choice_player_id"` that the web client depends on is asserted **nowhere** in the compiled suite.

**Would it pass if compiled?** Undeterminable without building, which was out of scope. Note line 267:
`json.get("choice_player_id")` depends on `get_pending_choice_json` emitting that key — that is precisely the
contract with no other compiled guard.

### 4b. `bytecode_deep_compare_test` — the bytecode↔JSON deep-compare oracle, never compiled

`engine/tests/test_modules/characterization/bytecode_deep_compare_test.rs:1` **[WT]**:

```rust
#[cfg(all(feature = "bytecode_abilities", feature = "json_path_test"))]
```

`json_path_test` is **not** in `Cargo.toml:62` `default`. `build.rs:149-152` adds only
`#[cfg(feature = "bytecode_abilities")]` to the `mod.rs` declaration, so the module is registered and then
compiles to nothing internally. **The only oracle that would catch a bytecode/JSON disagreement never runs in
`cargo test`** — which is directly relevant to Rank 7.

### 4c. `bytecode_validation_test` — gated the same way

`characterization/mod.rs:8-9`. The `mod.rs` gate is satisfiable (`bytecode_abilities` *is* default) but the same
`json_path_test` risk applies; I did not confirm its in-file gate **[WT]**.

### 4d. Modules referenced by a committed `mod.rs` but absent from disk **and from all of git**

`build.rs` regenerates these at build time, so a local build self-heals — but the **committed** files lie, and
anyone auditing coverage from the tree reads three suites that do not exist. Each parent directory contains
**only** `mod.rs` — zero test files:

| committed `mod.rs` | declares | git history |
|---|---|---|
| `engine/tests/test_modules/ability_gates/mod.rs:3` | `pub mod bounded_fan_cost_equality_gate_test;` | **never existed** |
| `engine/tests/test_modules/effects/hearts/mod.rs` | `pub mod set_heart_type_multi_target_test;` | **never existed** |
| `engine/tests/test_modules/expansion/mod.rs` | `pub mod pbl02_card_table_test;` | **never existed** |

All three `mod.rs` are tracked. `git log` returns nothing for all three names. `ability_gates/` is an entire
top-level category with no tests.

### 4e. `unique_abilities_test.rs` is a 0-byte file

`engine/tests/test_modules/characterization/unique_abilities_test.rs` **[WT]** — **0 bytes**, declared at
`characterization/mod.rs:28`. It compiles as an empty module. It tests nothing. Its name implies unique-ability
coverage that does not exist.

---

## Rank 5 — `TEST_INVENTORY.json`: two of the three stated defects do not exist; the counts moved under me

| claim | verdict |
|---|---|
| `n_files 1013` / `n_tests 4175` | **true at HEAD [HEAD]**, **already stale in the working tree [WT]** |
| omits the in-module `#[cfg(test)]` tests | **confirmed** — `n_test_fns_parsed` and `stats` do not cover `engine/src` |
| `trigger_counts` keys are **mojibake** | **FALSE — at HEAD and in the working tree** |

- **[HEAD]** `n_files = 1013`, `n_tests = 4175`.
- **[WT]** `n_files = 1014`, `n_tests = 4179`. A peer regenerated it during this audit (+1 file, +4 tests).
- **[HEAD] and [WT]** `trigger_counts` keys are clean Japanese, 7 of them:
  `'起動'`, `'登場'`, `'ライブ開始時'`, `'(none)'`, `'常時'`, `'ライブ成功時'`, `'自動'`. **Zero mojibake.**
  Anything quoting mojibake is quoting a stale copy outside the repo.
- The omitted in-module count: **`engine/src` carries 0 `#[test]` in 0 files by my scan** — the premise that
  144 in-module tests are being missed is **not confirmed** at this HEAD. See Rank 9 for the discrepancy.

---

## Rank 6 — Vacuous / weaker-than-named Rust tests

Full scan of all 4180 `#[test]` functions in `engine/tests` [WT] for the named classes:

| class | result |
|---|---|
| `assert_eq!(x, x)` / `assert!(x OP x)` — value against itself | **0** |
| assertion between two literal constants | **0** |
| `assert!(a >= b)` where `a` was bound from `b` within 3 lines | **0** |
| two locals bound to the same source, then compared | **0** |
| name says cannot/never, assertion is `>=`/`<=` | **4** (below) |

**The suite is not full of trivial assertions.** The one high-severity case is Rank 1. The four weaker-than-named:

| file:line | test | assertion | why weaker than the name |
|---|---|---|---|
| `test_modules/jidou/discard_watch/live_start_heart_requirement_reduction_counts_live_slot_cards_only_test.rs:59` | `hanamusubi_q213_member_card_moved_before_live_start` | `after_waitroom >= before_waitroom` | **Rank 1 — cannot fail.** |
| `test_modules/effects/compound/cost_and_effect/nahone_live_start_same_name_test.rs` :: `other_same_name_not_buffed` | | `assert!(blade1 >= 1); assert!(blade2 >= 1);` | name says *not buffed*; asserts a **floor** of 1, which a buff only raises. A same-name second buff passes. Needs an upper bound or an equality against the un-buffed baseline. |
| `test_modules/integration/full_round/live_score_tie_live_zone_placement_across_rounds_test.rs` :: `no_live_cards_at_all_no_winner_clean_rollover` | | `assert!(game.state.turn_number >= 1, "turn advanced")` | asserts the turn counter is ≥1. It is initialised to 1, so this holds before any pass. Establishes nothing about rollover. |
| `test_modules/jidou/title_once/dive_ab0_does_not_arm_without_own_main_phase_discard_to_hand_test.rs` :: `both_retrieval_arms_own_dive_not_opponents` | | `assert!(guard <= 20, "runaway prompt loop: …")` | a liveness guard, not a behavioural assertion; fine as a guard, but it is the only assertion standing behind that test name. |

---

## Rank 7 — The 325/805 skew: **a generation skew, and Rust is the correct side.** The brief's premise is inverted.

Measured, not inferred.

**Both engines are internally self-consistent. They are pinned to two different generator runs.**

| | Rust | C |
|---|---|---|
| bytecode blob | `cards/build/abilities.bin` = **98733 B** = 8-byte header `52424b41 01000000` (`RBKA` v1) + **98725 B payload** | `engine_c_wip/src/core/generated/bytecode_blob.c` — **98737 B**, header already stripped (starts `8,8,8,6,…`), `RBKA_BYTECODE_LEN = 98737` |
| slice-size table | `abilities_gen.rs:25` `OFFSET_DELTAS`, 985 entries, **sum = 98725** | `gen_data.c:7` `RBKA_OFFSET_DELTAS[985]`, **sum = 98737** |
| sum vs own payload | **98725 == 98725 — EXACT** | **98737 == 98737 — EXACT** |
| structural validation | **985/985** slices begin with the format tag `0x08` | **985/985** slices begin with `0x08` |
| strings blob | `cards/build/abilities_strings.bin` = 726649 B, `STRINGS_OFFSETS` 6031 entries, last offset **exactly 726649** | `engine_c_wip/src/abilities_strings.bin` = 726831 B, `RBKA_STRINGS_OFFSETS` 6033 entries, last offset **exactly 726831** |
| strings coherence | **0/6030 non-UTF-8** | **0/6032 non-UTF-8** |
| cross-read (wrong pairing) | — | 726831-blob read with 6031-offsets → **1920/6030 non-UTF-8** |

### The "8-byte shortfall" does not exist

`engine/src/ability/vm.rs:174-183`:

```rust
fn strip_bytecode_header(data: Vec<u8>) -> Vec<u8> {
    if data.len() >= 8 && &data[0..4] == BYTECODE_MAGIC {   // b"RBKA"
        … assert!(ver == BYTECODE_VERSION …);
        return data[8..].to_vec();                            // header removed
    }
```

and `vm.rs:165-168` / `:260-267` slice the **header-stripped** payload with `offset_of(idx)` = a bare prefix sum
over `OFFSET_DELTAS` (**no** `+8`). So `OFFSET_DELTAS` must sum to the **payload**, not the file.
98725 == 98725. **Rust is byte-exact. The 8 bytes are the header, by design.**

### Slice 325 and 805 are the two abilities that changed between generations

| slice | Rust | C | Δ | C start offset | drift |
|---|---|---|---|---|---|
| 325 | **149** | **159** | +10 | 32302 (identical) | 0 |
| 805 | **236** | **238** | +2 | +10 already | +12 |

Offsets agree exactly through slice 325, drift +10 for slices 326-804, +12 for 806-984.
**Against the repo's `cards/build/abilities.bin` the C table desyncs at slice 326** (reads `0xa5` where `0x08`
is expected) — but that is the *wrong* comparison, because the C does not read that file. Against its own blob
it is 985/985 clean. **Neither side has a data bug.**

**C's payload differs from Rust's in 63277 of the 98725 shared bytes** (first divergence at byte 163). This is
two generator generations, not corruption.

**Which cards, and does any Rust test depend on them? Yes.**

`CARD_ABILITY_PAIRS` (`abilities_gen.rs:71`) is `(str_idx, ability_idx)`; resolved via `STRINGS_OFFSETS`:

| slice | card | Rust tests naming it |
|---|---|---|
| **325** | **`PL!-pb2-012-P＋`** and `PL!-pb2-012-R` | `pb2_kidou_additional_cost_discard_two_or_wait_two_printemps_pl_pb2_012_p_plus_test.rs`, `pb2_optional_two_distinct_printemps_wait_costs_two_less_pl_pb2_012_p_plus_test.rs` |
| **805** | **`PL!N-bp7-028-L`** | `live_start_discard_shuffle_group_members_heart01_test.rs` (+ `card_type_filter_test.rs`) |

**So three Rust tests pin behaviour derived from the OLD generation's encoding of slices 325 and 805, while the C
executes the NEW generation.** Any C agent told to "match Rust" on these two abilities was matching a stale
artifact, and the instruction was unsatisfiable. `PL!-pb2-012-R` has **no** Rust test at all.

### Why nobody noticed: the Rust staleness guard is disabled

`engine/build.rs:321-325` **[WT]** — the hard error is commented out:

```rust
// if stale { panic!("Stale generated files — regenerate as warned above"); }
let _ = stale;
```

`build.rs:267-320` computes `stale` from four independent checks and then **throws it away**. So Rust will never
warn that its committed dataset is two generations behind the C's. This is the single highest-leverage fix in the
audit: it is the only thing standing between the two trees and a silent, permanent parity skew.

**Not a Rust bug. Not a stale artifact in the corrupting sense.** It is a pinned-generation skew, invisible
because the detector that would report it is disabled. `engine_c_wip/Makefile:122-123` documents the C's
generation explicitly (`985 abilities / 6032 interned strings / 726831 string-table bytes / 98737 bytecode bytes`)
and `Makefile:134-151` guards it — the C side is better protected than the Rust side.

---

## Rank 8 — Uncommitted in `engine/src`: **nothing.** The oracle is not moving under us.

`git status --porcelain -- engine/src` → **empty**, at both the start and the end of this audit.

**Correction to the brief: there are not 6 new untracked Rust test files. There is 1.** Untracked in `engine/`:

| path | size | assessment |
|---|---|---|
| `engine/tests/test_modules/effects/recover/to_hand/self_to_waitroom_recover_member_own_card_is_candidate_test.rs` | 155 lines, **3 tests** | **Genuine, high-quality, and does NOT run.** Untracked ⇒ absent from CI and from every peer's checkout. |

I read it. It is **not** vacuous: it asserts real outcomes (`:79-88` the cost-undoes-itself recovery;
`:108-131` that the sacrificed member is *offered* among two candidates, pinning candidacy rather than
coincidence) and it carries a **negative control** (`:138-155` `a_live_in_the_waitroom_is_not_a_candidate`)
that would fail an implementation recovering any waitroom card. It is also self-aware about the exact hazard
class this audit is hunting — `:21-25` names the "snapshot the cards this cost moved and exclude the mover"
optimisation and warns it would find nothing. This is the standard the rest of the suite should meet. **It is
untracked and therefore contributes zero coverage right now.**

Also untracked (not Rust source, baselines): `engine/tests/DEFECT_NAME_BASELINE.json`,
`engine/tests/NAME_MISMATCH_BASELINE.json`.

Modified in `engine/tests` [WT]: `TEST_COVERAGE.md`, `TEST_INVENTORY.json` (1013→1014 / 4175→4179),
`TEST_INVENTORY.md`, `TEST_QUALITY.md`,
`test_modules/effects/look_select/look_and_filter/look_seven_select_three_heart_member_test.rs`,
`test_modules/effects/recover/to_hand/mod.rs`. **All test-tree. Zero `engine/src`.**

---

## Rank 9 — Rust source vs Rust tests, and a genuine Rust card-loss bug

### 9a. `set_live_card` destroys a card when the live zone is full — and the C deliberately does NOT inherit it

`engine/src/turn/phases.rs:827-847` **[WT]** = **[HEAD]**:

```rust
830:        if !player.hand.cards.is_empty() && idx < player.hand.cards.len() {
831:            let card = player.hand.cards.remove(idx);   // ← removed from hand FIRST
832:            player.live_card_zone.face_up = false;
833:            let live_cards = &mut player.live_card_zone.cards;
834:            if live_cards.len() >= MAX_LIVE_CARDS {
835:                return Err("Live card zone is full".to_string());   // ← card is now nowhere
836:            }
837:            live_cards.push(card);
```

**On a full live zone, Rust removes the card from hand and then returns `Err` without pushing it anywhere. The
card ceases to exist.** No Rust test covers a full-live-zone rejection. The C refuses *before* removing
(`engine_c_wip/src/turn/phase.c:1288`, per its own comment at `:1350-1355`: *"Noted, not fixed"*).

**Here the C is right and Rust is wrong.** A legitimate source-vs-source divergence where the C deliberately
declined the oracle. Any parity harness asserting equality on this path is asserting the bug.

### 9b. `live_face_up` is declared per-card in the C header but is per-zone in Rust

`engine_c_wip/include/rabuka.h:1801` declares `uint8_t live_face_up[RB_MAX_LIVE_CARDS]` with a comment calling it
a per-card parallel array beside `live`. Rust's `LiveCardZone.face_up` (`zones.rs:501`) is **one flag for the
whole zone**, and `zones.rs:1038` `add_card(&mut self, card_id, _face_up: bool)` **ignores** its flag (hence the
underscore) — so no per-card value ever exists. The C writes it seat-indexed at `[0]`/`[1]`
(`phase.c:703`, `:1356`, `:1432`, `:1658`). Storage is oversized; the **comment is wrong** and will mislead the
next agent.

### 9c. `fetch_look_pool` silently ignores `all_any` for deck sources

`engine/src/ability/effects/look.rs:1017-1053` **[WT]** = **[HEAD]**. `take_cards` honours `fetch_all`
(`1023-1029`), but the two deck arms bypass it:

- `1031-1033`: `Zone::Deck | Zone::DeckTop => player.main_deck.draw_multiple(count)` — ignores `fetch_all`
- `1034-1040`: `Zone::DeckBottom => { … drain(start..) }` — ignores `fetch_all`
- `1041-1049` `Stage` and `1051` default: **do** honour it

So `{{任意の枚数}}` / `all_any` returns *all* cards from a stage or hand but only `count` from a deck. Also note
`execute_look_at:1068-1071` treats `DeckBottom` as a deck source for the Q85 under-deck refresh branch, but
`fetch_look_pool`'s deck-bottom arm **drains from the bottom-most end** (`:1035-1039`, "the bottom-most card is
the last index") while recording `carried.looked_at_origin = source` (`:1086`) — a declined optional move would
return them to the deck top, reordering it. Three agents rewrote this arm in C; the asymmetry is a divergence
candidate in either direction.

### 9d. `evaluate_success_zone_constant_modifiers` scans four zones, and `non_stackable` is keyed by text

`engine/src/core/game_state/modifiers.rs:2188-2305` **[WT]** = **[HEAD]**. `2232-2237` scans
`player1.live_card_zone`, `player1.success_live_card_zone`, `player2.live_card_zone`,
`player2.success_live_card_zone`. The justification (`2225-2230`) cites `PL!-pb2-038-L`
「自分のライブカード置き場**か**成功ライブカード置き場にあり」. Note this function runs from
`recalculate_constants` ← `check_timing` ← **`phases.rs:240`, i.e. inside the Q213 window** — so it grants
success-zone blade/heart/score constants from cards in an unrevealed live slot. `2280-2287`: `non_stackable` is
keyed `format!("{}:{}", effect.action, effect.text)` — **not by card id** — so two distinct cards with identical
wording, or two copies of one card now that both zones are scanned, collapse to a single application. The
four-zone widening makes that collision window strictly larger.

### 9e. Regions checked and found consistent

- `turn/actions/mod.rs:1626-1683` `check_timing` — `check_invalid_live_cards` correctly invoked for **both**
  seats (`:1653-1654`); the defect is the `face_up` gate, not the dispatch. Step order
  duplicate-members → refresh → victory → invalid-live → invalid-energy → orphaned-under → `recalculate_constants`
  → invalid-resolution → auto-abilities is internally coherent. `:1672-1675` documents why the omitted second
  victory pass is a no-op — that reasoning holds.
- `ability/effects/move/mod.rs:1855-1869` `maybe_prompt_deck_order` — `is_deck_dest` correctly covers
  `Deck | DeckTop | DeckBottom`, with `DeckBottom` added for 「3枚まで好きな順番でデッキの下に置く」 and the
  regression it fixed recorded inline. Sound.
- `ability/cost/handlers.rs:1683-1700` `pay_cost_all:discard_all` — `hand.cards.drain(..)` →
  `waitroom.cards.extend(..)` ignores `cost.count` and `cost.source_str()`; correct **only** while the caller
  guarantees `source == "hand"`. The prompt arm (`:378-392`) and the execution arm (`:1696`) are bound by the
  **string literal** `"pay_cost_all:discard_all"` — a typo on either side is a silent no-op with no compile
  error. `:1286` `push_revealed_cost_card` is consistent with `:1285`.
- `ability/condition/card.rs:1632-1658` `cause_card_id` arm — guarded by
  `this_turn && self_target && group_names non-empty && card_type == member_card`; iterates
  `turn_area_movements` for `moved_card_id == activating_card_id && effect_only && cause_player_id == player.id`
  and requires `cause_card_id.is_some_and(...)`. Reads correctly against the text. This is the arm three agents
  rewrote in C and it is the one region in this set where I found **no** source-level defect.

---

## Rank 10 — The Q213 list: `cards.json` vs the Rust implementation

Cases where the ruling text and the Rust implementation disagree. Ranked by blast radius.

| # | card(s) | ruling | Rust behaviour | file:line | C inherits? |
|---|---|---|---|---|---|
| **1** | `PL!HS-bp5-019-L` (+ any card counting its live slot) | **Q213**: members move to the waitroom **before** `live_start`, so they do not reduce the required heart | members are still in `live_card_zone` and are counted; second attacker's members survive the whole first performance | `phases.rs:240/245-246/343/349` + `actions/mod.rs:1734` + `condition/card.rs:4419-4421` | **yes** |
| **2** | `PL!HS-bp2-014-N`, `PL!SP-bp1-001-P`, `PL!SP-bp1-001-R` | **Q68**: a ライブできない player's live cards also go to the waitroom; `live_start` unusable; **no yell** | `live_start` fires (no `cannot_live` gate); live cards discarded during the yell | `trigger_scan.rs:186-212` + `phases.rs:351/380/411-415` | **yes** |
| **3** | any card setting a live card with a full live zone | a rejected action must not destroy a card | card removed from hand, then `Err` — **card destroyed** | `phases.rs:831` then `:834-836` | **no — C refuses first** |
| **4** | `PL!N-bp1-027-L` | **Q67**: ハート counts as an arbitrary colour for the live required-heart check, but **not** at `ライブ開始時` | two separate paths required; I confirmed the distinct code paths exist but did **not** establish that they diverge correctly | `condition/state.rs:215-253` region | **unverified** |
| **5** | `PL!S-pb1-008-P＋`, `PL!S-pb1-008-R` | **Q131**: `live_start` effects activate on **your own** live start only | attribution by `player_id` in the scan; consistent with the text | `trigger_scan.rs:186-246` | no divergence found |

Only **7** FAQ answers in all of `cards.json` (2639 cards) mention `ライブ開始時`; 5 distinct rulings, listed
above. #1 and #2 are confirmed Q213-class. #3 is a Rust bug the C escaped. #4 I could not settle and flag as
open. #5 is clean.

---

## Rank 11 — `TEST_INVENTORY.json` count discrepancy (open)

My independent scan of `engine/tests` **[WT]** found **4180** `#[test]` functions (script: split on
`#[test]`/`#[tokio::test]` across every `.rs` under `engine/tests`, excluding `mod.rs`).
`TEST_INVENTORY.json` **[WT]** says `n_tests = 4179`. **`engine/src` contributed 0 in-module `#[test]`s**, so
the "144 omitted in-module tests" premise is **not reproduced at this HEAD** — either those tests were moved into
`engine/tests` by commit `2130769d`, or they were deleted. Resolving this needs a `git log -S` on a known
in-module test name; I did not run it. **Do not quote 1013/4175, 1014/4179, or 4180 as *the* test count until
this is settled** — three numbers are currently in circulation.

---

## Recommended order of repair (for a human to decide)

1. **Fix or retire the Q213 test** (Rank 1). It is actively harmful: it is named for a ruling, asserts nothing
   about that ruling, and cannot fail. Either assert `get_need_heart_modifier` with a 蓮ノ空 **member** in the
   live slot, or delete it.
2. **Re-enable `build.rs`'s staleness panic** (Rank 7). It is four lines and already written, commented out at
   `:323`. It is the only thing that would have surfaced the generation skew.
3. **Un-track the `recover/to_hand` test** (Rank 8) — it is good and currently runs nowhere.
4. **Decide the generation skew** (Rank 7): pin both trees to one generator run, or document that the C is
   ahead. Then re-baseline every parity number — 63277 bytes of bytecode differ.
5. **Fix the C's Q213 comment** (`phase.c:1651-1654`) before someone acts on its wrong diagnosis.
6. Add a `live_start`-suppression test for `cannot_live_players` (Rank 3) and a full-live-zone card-conservation
   test (Rank 9a).
7. Triage `pvp_room_test` (Rank 4a): either gate the suite on `--features server` and run it, or delete it. Its
   14 tests are the only guard on the `choice_player_id` JSON contract.

**Not fixed, by instruction:** every Rust defect above is reported, not edited. Rust's `OFFSET_DELTAS`,
`set_live_card`, `check_invalid_live_cards` gate and `trigger_live_start_abilities` are all byte-for-byte as I
found them.