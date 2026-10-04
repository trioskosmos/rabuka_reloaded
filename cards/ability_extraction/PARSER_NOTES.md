# Parser Notes — canonical debt, history, and status

**File**: `cards/ability_extraction/parser.py` (~14.0K lines)
This is the single maintained parser maintenance document. It consolidates
the former `PARSER_REFACTOR.md`, `PARSER_DEEP_REFACTOR.md`,
`PARSER_DEBT.md`, and `PARSER_UNTANGLE_PLAN.md` (all completed or rescoped;
history remains in Git).

## Session: untangling pass (2026-09-28)

Every change below was proved output-neutral by the usual method: regenerate,
then `git diff --numstat cards/abilities.json` must show only the
`generated_at` and `engine_commit` lines. Parser suites
(`test_registry_coverage` 4, `test_parse_action` 46, `test_parser_coverage` 44)
and the engine suite (3729) all pass.

### Removed

- `action_047_gain_resource` — unreachable. `action_039`'s condition is a strict
  superset and sits earlier in the first-match-wins list; 047 also had no
  `setter`, so it would have emitted a worse shape than 039 had it ever fired.
  It survived because the dead-rule test only covers the effect and condition
  registries, not `_ACTION_RULES`.
- `ActionRule.priority` / `.order` — decoration. Dispatch (`parse_action`) walks
  `_ACTION_RULES` positionally and breaks on first match; the only read of
  `priority` was the `--list-rules` dump. Removing them from all 83 rows is the
  point: a hand-written `priority=39` implies a ranking the table does not have.
  **Rule order in the list IS the priority.** The `--list-rules` header now says so.
- `build_lib.BinaryEncoder` (~78 lines), `generate_rust_include_bytes`,
  `load_json`, `save_json` — zero references. `BinaryEncoder` was a full second
  copy of the `BC` encoder inlined in `compile_abilities.py`.
- The `git_hash` block in `compile_abilities.py.main` — computed, then never
  passed anywhere; `write_generation_manifest` resolves `engine_commit` itself.
- The headerless `abilities.bin` write in `compile_abilities.py.main` — a 93KB
  write immediately overwritten by the headered one 6 lines later.
- `test_parsing()` and its unconditional call in `main()` — debug scaffolding
  printing a `=== Test Parsing ===` block on every production run.
- `DECOMPRESSED_LEN` from the generated `abilities_gen.rs` — emitted, read by
  nothing in `engine/src` (only the separate `engine_c_wip` C tree reads its own
  copy), and wrong by 8: it said 93023 while the real decompressed size is 93031,
  because the compressed blob carries the 8-byte `RBKA` header.

### Fixed

- `extract_card_abilities.py` deleted `build/abilities.bin` immediately after
  regenerating it, and `abilities_gen.rs` `include_bytes!`s that path under the
  `gba` feature — so the documented entry point broke the GBA build. The
  compiler's exit code is also no longer swallowed, and decoder failure is now
  fatal rather than a printed warning.

### Deduplicated

- Cost-change verbs `(減る|減らす|増える|増やす)` were spelled in three places with
  a **real** divergence: the `action_014` dispatch guard accepts `コスト[はが]`,
  `_COST_DELTA_RE` accepts `コスト[はがを]`, and `COST_MODIFICATION_PATTERNS`
  only `コストは`. Factored to `COST_CHANGE_VERBS` with the divergence documented
  as deliberate — the guard is narrower on purpose so it cannot claim a form the
  extractor would then mis-type. Do not "fix" one to match another.
- The trigger-gate separator list was duplicated verbatim in `card_overrides.py`.
  Now `parser.TRIGGER_GATE_SEPARATORS`, imported by both.
- `SPLIT_LIMIT = 1` was defined in both `parser.py` and `cost_parser.py`. Now in
  `parser_utils`, the module that already owns shared parser vocabulary.

### NOT dead — do not "clean these up"

An earlier review flagged `cards/build/abilities_gen.rs`,
`cards/build/bytecode_data.c` and `cards/build/cards_gen.rs` as unreferenced
duplicates of the engine copies. That is wrong:

- all three are **gitignored** (`.gitignore:112,113,126`), so they cannot drift
  in the repo in the first place;
- `cards/build/abilities_gen.rs` is consumed by the C port
  (`engine_c_wip/Makefile:49` → `tools/gen_from_rs.py`);
- `cards/build/bytecode_data.c` is consumed by
  `engine_c_wip/tools/gen_bytecode.py:11`.

`platforms/snes/bytecode_data.c` is the third, separately-generated copy that
`platforms/snes/build.rs:26` compiles.

### Dispatch order made honest (2026-09-28)

The four tables still exist, but the **order is now readable off one list per
registry** instead of being encoded in magic numbers nothing explained.

Effect priorities used to be `-10`, `0..4`, `100..147` and `10000..10005`, with
the `-10` and `10000+` rules registered *after* the tables were built — so
reading `_EFFECT_RULES` did not tell you the real order. They are now
`_EFFECT_DISPATCH_ORDER`, one list in the order they fire, registered
sequentially: registration index IS the priority. Same order, same output.

Conditions were `tier * 100 + index`, which silently breaks the tiering the
moment the table passes 100 rows. They are now `_CONDITION_DISPATCH_ORDER`,
extended by the one out-of-band rule and registered in a single pass.
`placed_discard_live_or_member` used to be registered at priority `1` *after*
the table was built, shadowing the whole tier-1 block it was written next to; it
is now visibly first in the list.

Rule names were the other half. Effects registered under
`getattr(rule, "__name__", f"effect_rule_{i}")`, but a rule is an
`EffectPattern` *wrapper* — the function that recognises the phrase is its
`handler`, and asking the wrapper left **30 of 61 rules named `effect_rule_N`**,
so the dead-rule test could not say which phrase had gone stale. `_rule_name`
now falls through handler → setter → action, and no registered rule is
identified only by its position.

`python parser.py --list-rules` prints the **effective** order for all layers.
Trust it over the order the tables are written in.

### Merging the four tables — NOT the right fix, don't attempt it

An earlier draft of this doc listed "collapse the four dispatch tables into one
registry" as the remaining parser untangle. On inspection that goal is
**ill-posed**, and attempting it would make things worse:

| Table | Produces | Entry point |
|---|---|---|
| `_ACTION_RULES` | the `action` dict (what the card DOES) | `parse_action` |
| `CONDITION_PATTERNS` | a `condition` dict (a sub-clause) | `parse_condition` |
| `_EFFECT_RULES` + `_STRUCTURAL_EFFECT_RULES` | the `effect` dict (effect text) | `parse_effect` |
| `_COST_HANDLERS` | the `cost` dict (payment) | `parse_cost` |

These are four different OUTPUT SHAPES parsed from four different parts of a
clause. A single dispatch loop would have to be told which shape it is
producing on every call, and the first-match-wins guarantee would have to hold
across tables whose outputs are not comparable — a condition rule that
"matched" would shadow an effect rule for reasons the reader cannot follow. The
tables stay separate.

What actually caused the pain — not the count, but the invisible *order* — has
been fixed. See "Dispatch order made honest" above: no magic priorities, no
out-of-band registrations, every rule named, and `--list-rules` printing the
effective order. The residual cost is one decision per new clause ("is this an
action, a condition, an effect, or a cost?"), which is inherent to clause
grammar and not something a merged table would remove.

### There is NO dead action-rule backlog — do not go looking for one (2026-09-28)

This section previously claimed 36 action rules were "shadowed" and needed
per-rule review. **That claim was wrong and the work it motivated was reverted.**
Corrected record:

A "does this rule ever WIN dispatch" check cannot be made sound for
`_ACTION_RULES`. The table is first-match-wins and is dispatched against many
derived clause forms — sequential steps, nested sub-actions, per-effect
re-parsing — so whichever text you drive it with, live rules go unreached.
Measured false-positive counts on the same 82 rules:

| driving input | "dead" rules reported |
|---|---|
| `parse_action` per corpus `triggerless_text` | 37 |
| full `parse_ability` per corpus text | 29 |
| raw ability strings from `cards.json` | 30 |

Three inputs, three different wrong answers. Acting on the first number meant
deleting 37 rules, which **removed `modify_limit`, `repeat_procedure` and
`draw_until_count` from the emitted corpus** (4 decoder warnings, 2 missing
mechanics, validation 0 → 2 issues, bytecode 93023 → 92509). The regenerate +
`git diff --numstat cards/abilities.json` gate caught it — no test did. The
rules were restored verbatim and the suite is green.

There is no dead-rule backlog here. The single provably dead action rule was
`action_047`, removed by static proof: its condition is a strict subset of
`action_039`'s and 039 sits earlier in the list. **Retire an action rule by
proving the shadowing statically, or not at all.** An "apparently dead" action
rule is almost always live on a clause form the probe did not produce.

`test_registry_coverage.py` carries this as a comment so the test is not
re-added.

### A condition with NO operator and NO bound is a tautology, not a gap (2026-10-04)

`compare_counts` reads a **missing** operator as `>=` — deliberately, so a
parser gap does not silently disable an ability. The cost is that the same
shape silently does the opposite when there is also nothing to compare
against: `count >= 0`, true on every board, forever. Three cards have carried
it, in three different disguises, and **in every case the positive test passed
identically before and after the fix.** A positive cannot see a tautology; only
a negative on the board that should fail can.

| Card | Parsed as | Disguise | Now |
|---|---|---|---|
| `PL!-pb2-021-N` 南ことり | `comparison_condition`, no operator | an EVENT ("she paid a cost") fell into the numeric path | `paid_cost_this_turn` |
| `PL!N-bp5-013-N` 上原歩夢 | `comparison_condition`, no operator | plus `resource_type: energy`, which short-circuits to the energy-zone length **without reading `location`** | `card_count_condition` + `member_has_resource_under` |
| `PL!SP-bp4-016-N` 葉月恋 | `comparison_condition`, no operator | an EVENT ("each time energy is placed") read as a zone STATE — `「〜が置かれるたび」` is habitual-present, and the rule accepted only `「置かれた」` | `movement_condition` + `trigger_event.energy_placed` |
| `PL!N-bp5-010-*` 三船栞子 | `comparison_condition`, no operator, `negation: true` | **negating a tautology**, and the engine does not apply `negation` on this path — so the branch is decided by a bound it does not have | **OPEN, and deliberately so — see below** |
| `PL!-pb2-037-L` 春咲Lewitt | `all_members` dropped at decode | not a missing operator — a marker lost to the wrong variant (see below) | `all_members` on `ConditionCommon` |

### 三船栞子 is OPEN, and the parser half alone makes it worse

This one is recorded as a fixed row only to be immediately withdrawn. 「余剰ハートを
持たない場合、＋１」 parses to `comparison_condition { resource_type: "surplus_heart",
negation: true }` with no `count` and no `operator`, so `compare_counts(None, surplus, 0)`
reads `surplus >= 0` — true everywhere. The tempting fix is to emit `count: 1` and
`operator: ">="` so the clause is discriminating in the letter. **That was tried and
reverted**, for a reason worth keeping:

> `negation` is **not applied** on the surplus comparison path. So the branch is decided by
> the bound alone, and adding a bound of 1 does not produce "has none" — it produces
> "surplus >= 1". At surplus 0 the old shape was *correct* (and is pinned by
> `surplus_conditioned_score_live_total_plus_one_no_floor_needed`), and the new shape broke
> it. The error moved from surplus 0–∞ to surplus 1–∞: still over-firing, just somewhere
> else, and a green test became red.

**The ordering is the lesson.** Honouring `negation` makes the current shape false
*everywhere*, and only then does `count: 1` turn it into 「has none」. Engine first, parser
second — never parser alone. A half-fix that changes a pinned behaviour without fixing the
mechanism is worse than the gap, because it looks like progress and costs a test.

The genuinely unpinned case is surplus **exactly 1**: neither branch should apply there,
and nothing in the suite says so. That is the test to write first, and it will fail until
the engine side lands.

Three lessons worth keeping, all of which cost a card:

- **A flag on the wrong struct is a silent discard.** `all_members` lived on
  the `Group` variant, so a clause the parser typed as `location_condition`
  dropped it at decode and degraded to "at least one". Nothing warned: the
  condition still decoded, still evaluated, still had a plausible shape.
  Anything a clause can carry belongs on `ConditionCommon`, which is why the
  fix is a field move rather than a handler.
- **`resource_type` is not a location, and a count dispatcher that
  short-circuits on it will ignore `location` entirely.** 歩夢's clause reads
  「自分のステージにエネルギーカードが**下にあるメンバーがいる**」 — the counted set
  is MEMBERS on the stage, and the qualifying fact is about a card
  *beneath* each one. No `Location` expresses that (`under_member` means the
  ACTIVATING card's under-cards — 中須かすみ `PL!N-pb1-002-P＋`, a different
  clause), so it is a `ConditionCommon` flag that filters the stage set, and
  the clause reuses the ordinary operator/threshold comparison.
- **A condition can be vacuous while the engine still works, because a gate
  downstream is doing the real check.** 葉月恋 fired correctly for a long time
  on `core/game_state/abilities.rs`'s `each_time` movement-event guard — and that
  guard's clause is keyed on the condition being `Condition::Comparison`
  *specifically*. Fixing her parse therefore switched the gate **off** and made
  her under-fire until `energy_placed: true` was added. Three of the five
  failures in that run were the honest parse being under-served by the engine,
  not the parse being wrong. **When a condition is found vacuous, expect the
  real check to be somewhere else and find it before changing anything** —
  otherwise you replace a working over-firer with a broken under-firer and read
  the regression as evidence that the fix was wrong.
- **A regex keyed on a word the card happens not to use will silently drop a
  flag.** `エネルギーが` does not match `エネルギーカードが`. The marker is
  read by `condition.get_energy_placed().or_else(trigger_event)` — a fallback
  that looks redundant and is the reason a `trigger_event`-only marker works at
  all — so the parse decoded cleanly and evaluated to "no energy check", which
  the engine then read as "an area-move watcher".

**How to find these:** a shape scan over `abilities.json` for
`comparison_condition` / `card_count_condition` nodes carrying no operator and no bound
returns 10 hits on 7 cards, of which 3 are fixed and 1 is open by choice. Most of the rest
are legitimate — 「此外」/「=」 forms and headcounts the extractor reads correctly, and the
`source`-keyed `これにより…` result conditions have a discriminator — so the scan is a
*triage* list, not a defect count. Add the negative first and let it fail; do not reason
about the shape.

### Two of these were invisible for a reason worth naming

栞子's `+1` and the tests around it are the clearest case in the corpus of a green test
proving nothing:

- the clause was `negation` over a tautology, so the branch **under**-fired — and the
  file's two tests asserted a **net of 0**, which is what both a correct cancellation and
  an ability that never ran produce;
- the tests also stopped at `FirstAttackerPerformance` while the trigger is ライブ成功時
  (`LiveVictoryDetermination`), and read the result through `mods.get_score_modifier` when
  a `live_total` change is only visible on `performance_snapshots`. They were reaching
  nothing, for three independent reasons, and asserting the value that reaching nothing
  produces.

A **net** assertion is the weakest kind: it is satisfied by "both halves ran" and by
"neither ran". Where a card has a `+1` and a `−1` on the same target, assert each half
against a board built for it, and read the target where it actually lands. And when the
fixture turns out to be hard — 栞子 needs 余剰ハート = 0 while her own hearts guarantee
excess against every live card in the pool — **pin the parse and say the behavioural half
is open**. Do not ship a test that passes because the harness does nothing.

### A two-leg sentence loses a leg, and the leftover gate cannot decide anything (2026-10-04)

星空 凛 `PL!-pb2-014-P＋` (登場):

> 手札の『lily white』のライブカードを1枚公開してもよい：**(1) 自分の成功ライブカード置き場にあるカードを1枚手札に加える。(2)
> そうしたとき、これにより公開したカードを自分の成功ライブカード置き場に置く。**

The parse kept (2) and **dropped (1)** entirely, so the card emptied the success live zone and
put nothing into hand. What made it invisible is the second half: with the first leg gone,
「そうしたとき」 had nothing to gate, and the generic condition path expressed it as
`{ locations: [success_live_card_zone, hand], count: 1, operator: "=" }` — a card-count
comparison with `=` and no value, which decides nothing and therefore blocks nothing. A
half-built gate is worse than a wrong one: it looks like a condition.

Two things this is *not*:

- **Not the 「そうしたとき」 family.** Every existing rule for that marker splits
  「<optional>そうしたとき、<consequence>」 into a `conditional_on_optional`. Here the
  「してもよい」 belongs to the **cost** (the optional reveal), so the effect text is a
  plain two-step sentence and both legs are mandatory; the marker only means "if leg (1)
  happened". Reaching for that family would have built a conditional on an optional that
  does not exist in this clause.
- **Not new vocabulary.** `source: "preceding_moved"` is the engine's existing spelling
  for "the cards the preceding step moved" (`ability::condition::is_moved_source`), so the
  gate needed no new field and no new evaluator.

`_try_two_step_zone_move` owns it: both legs must parse as `move_cards`, the marker must be
the sentence-initial 「。そうしたとき、」, and a 「N人に以上」 headcount or 「以下」 bound is
refused so it cannot claim the `previous_moved_cards` batch forms the generic
`preceding_moved` handling already covers. Blast radius: one card.

**The lesson generalises to a shape, not a card.** When a clause is dropped, check what the
surviving node's condition is doing — an unevaluable one (`=` with no value, an operator
outside the engine's vocabulary) is the fingerprint of a dropped sibling clause, and it is
invisible to every coverage count because the node still decodes and still looks plausible.

### An ACTION-shaped optional offer is not a cost-shaped one (2026-10-04)

矢澤にこ `PL!-pb2-018-P＋`/`-R` (登場):

> 相手のステージにいるウェイト状態のメンバーを3人までアクティブにしてもよい。
> そうしたとき、これによりアクティブにしたメンバー1人につき、カードを1枚引く。

This reads like the ordinary 「〜してもよい。そうしたとき、〜」 family — ミア,
日野下花帆, 澁谷かのん, 津島善子 all parse into `conditional_on_optional` with an
`optional_action` and a `conditional_action`. **Filing it there would be a real bug.**
`execute_conditional_on_optional` treats `optional_action` as a COST: it presents a
Skip/Pay choice and runs the consequence on Pay. Here the offer is an EFFECT — waking
members on the opponent's stage — so a player could "pay" by waking nobody and still draw.
Action-shaped optionals go through `sequential` + `conditional: true`, the route DIVE!
already takes (see the NOTE above `_try_conditional`'s caller).

What the parse did before: the whole sentence collapsed to a bare `draw_card` behind
`condition { state: wait, count: 1, operator: ">=" }`. Three separate losses in one node —
the **activation**, the **「3人まで」 cap**, and the **「てもよい」 optionality** — and the
consequence no longer depended on the offer at all. She drew 1 for merely *having* a waited
member on the opponent's stage.

`_try_optional_state_change_consequence` owns it and hands the offer to `parse_action` with
the optionality stripped, so the existing `change_state` rules keep the
「アクティブにする」/「ウェイトにする」/「 energetics vocabulary and this rule owns only the
optionality and the 「そうしたとき」 wiring. It refuses any offer that does not parse to
`change_state`, precisely so it cannot re-file the cost-shaped members of the family onto
the wrong container.

**Why it is worth writing down as a class:** the same sentence shape fails two different
ways depending on what the offer does. A `move_cards` offer is correctly a cost
(`make_conditional_on_optional`); a `change_state` offer must not be. A future 「〜を配置
してもよい。そうしたとき、〜」 is the third case and will need a third answer, so the rule
refuses rather than guesses.

### The gap-pin file was the thing that made it findable

`engine/tests/test_modules/effects/choice/selection_filter/niko_debut_draw_per_activated_parser_gap_test.rs`
asserted the *absence* of the activation and said, in its own header, to invert the
assertions when the activation half parsed. That is the convention working: the gap had a
failing-on-purpose test, so closing it turned the test red instead of leaving it silently
stale. Inverting it was mechanical.

One trap in the inversion, worth recording: the old finder located the ability by its ROOT
action (`draw_card`). Once the parse is correct the root is a `sequential`, so the finder
stops matching and the inverted test fails with a `None` that looks like a missing field
rather than a stale lookup. Find the ability by the SHAPE that must exist — two legs,
second is `draw_card` — never by the shape the gap left behind.

### Still open (deliberately not done)

- **The 9 count entry points in `ability/condition/card.rs` still overlap
  structurally.** The duplicated *mechanics* were extracted (`is_moved_source`,
  `util::zone_card_ids_occupied`), but `get_count_for_condition` /
  `get_count_for_target` / `get_group_card_count` remain three parallel
  dispatchers over shared player resolution, multi-zone fan-out and zone
  dispatch. Merging them changes which dispatcher evaluates a given condition,
  so it needs characterization pins per condition shape.
- **`ConditionPattern`'s declarative fields were unused — DONE 2026-10-04.** This entry
  said "all 36 rows pass only `handler=`"; measured with `ast` over every call site it is
  **44 registrations, and `match` / `match_any` / `match_all` / `exclude` / `exclude_any`
  are passed ZERO times.** They were also inert, not merely unused: `text_matches` with all
  five empty is `(not "" or …) and …`, vacuously `True`, so the
  `if not self.matches_text(text): return None` guard in `__call__` could not reject
  anything. Five fields on every instance read as though they were live.
  `ConditionPattern` no longer declares them and no longer calls `matches_text`.

  The same `ast` pass over the other two classes is what stopped this being a
  wrong "delete the lot" edit:

  | class | sites | `match` | `match_any` | `match_all` | `exclude` | `exclude_any` |
  |---|---|---|---|---|---|---|
  | `ActionRule` | 82 | 23 | 11 | 9 | 4 | 1 |
  | `EffectPattern` | 81 | 5 | 0 | 2 | 0 | 0 |
  | `ConditionPattern` | 44 | 0 | 0 | 0 | 0 | 0 |

  So `EffectPattern` keeps `match` and `match_all` and loses the other three, and
  `ActionRule` is untouched. **Re-run the measurement before extending this**: it is three
  numbers, and it is the only thing that distinguishes a dead field from a live one.

  The five now live as class-level defaults on `_TextRule` rather than being redeclared
  per subclass. That is what lets a subclass declare a subset without `matches_text`
  needing a `getattr` to paper over the gap — and because they are plain class attributes
  rather than dataclass fields, a misspelled field name on a subclass still raises
  `AttributeError` instead of silently reading a default. (It has to be this way: making
  them dataclass fields on the base would sort them ahead of `ConditionPattern`'s
  positional `name, tier, handler`, which is its call API at 37 sites.)

  Output-neutral, proven the usual way: regenerate, then diff per ability. 985 → 985,
  0 added, 0 removed, and the only changed entries are the four cards fixed earlier the
  same day (星空凛, 矢澤にこ, 上原歩夢, 葉月恋).
- **Engine:** `owning_effect` / `answering_effect` and the
  `pending_choice_owner` snapshot (2026-09-28) remove the guess at the three
  answer sites. Still open: the sites that read `current_effect` outright with
  no fallback, and `move_cards.rs:2900`'s hand-rolled `select_action` chain,
  which is a third resolution shape not yet folded in.

## Remaining debt (live)

### Engine-side key audit (2026-08-24)
Cross-referenced every JSON key emitted into `abilities.json` against
`card.rs` structs + engine handlers. Findings:

| Key (count) | Disposition |
|---|---|
| `check_self` (6 conditions) | **IMPLEMENTED in engine** — new `ConditionCommon.check_self`, decoded + evaluated in `evaluate_check_self_condition`; presence of the ACTIVATING card in the location instead of counting matching cards. Pinned by `check_self_condition_test.rs` (2 tests) |
| `zone`/`energy`/`costs`/`max_repeats` | handled by vm.rs aliases / decoder (audit false positives) |
| `source_location` (2 gain_ability_from_source) | engine already hardcodes under-member sourcing; documentary |
| `action_reference` (1) | decodes as AlwaysTrue alias; acceptable degradation |
| `action_reference` (1) | decodes as AlwaysTrue alias; acceptable degradation |
| `choice_modifier` (2 choices) | **removed from emission** — structured choice_condition/alternative_condition/alternative_count_type were always present and are what the engine's tiered-choice evaluation reads (compound.rs) |
| `target_event` (1) | **removed from emission** — engine keys replacement effects off destination=success_live_zone (pinned by replacement_destination validation rule) |
| `per_character` (1, LL-bp7-001 play cost) | known limitation: engine selects count=len(characters) restricted to those names but cannot enforce exactly-one-per-name; single card, low impact |
| `baton_touch`(on appearance), `energy_state`, `comparison_source`, `area_direction`, `positions_characters`, `turn_number`, `cost_reference_*` | decoded or harmless documentary |

### Phase 8 — Standardize `_ACTION_RULES` format — DONE (stale entry)
All registrations already go through the ActionRule-normalizing
`_register_action` (`__post_init__` arity normalization, E2a session); the
dispatch loop has no TypeError workaround. Nothing to do.

### `_process_pre_fix` FIX-block triage (2026-08-24 session)
Empirically verified by removal + regen byte-diff, one block at a time:

| Block | Verdict | Evidence |
|---|---|---|
| FIX 6 (opponent_action flatten) | **DEAD — removed** | 0 output change; no producer emits the wrapper |
| FIX 2 (each_time → conditional_on_optional) | **DISSOLVED into producer** | `_try_each_time`/`_finish_each_time` reshapes at parse time; byte-identical |
| FIX 3 (conditional_on_optional cleanup) | **DISSOLVED into parse_effect** | `_strip_coo_child_optional` after `_propagate_optional`; 1-ability delta (stale nested optional on shared children now stripped uniformly — the intended semantics). Dead positive/negative renames dropped |
| FIX 7/7b (ability_filter backfill) | **DISSOLVED into producers** | `_apply_no_ability_filter` in `_handle_cost_modification` + `parse_action` select path; byte-identical |
| FIX 9 (result_condition card_property) | **DISSOLVED into producer** | enrichment moved into `_try_kore_niyori_result`; byte-identical |
| FIX 9b (followup self_target/self_cost) | **DISSOLVED into producer** | moved into `_try_kore_niyori_result` followup construction; byte-identical |
| FIX 10–15, N | characterized 2026-08 by per-block removal-diff | FIX 11 & FIX N: **DEAD — deleted** (0 abilities each). FIX 15: **DISSOLVED into parse_action** (Rule 11.10.1 exclude_self now uniform; 4 nested nodes gained it, semantically correct). FIX 10/12/13a/13b: live pipeline steps (1/1/1/2 abilities), kept with characterization |

Conclusion: every verified-load-bearing compensation block has been
dissolved into its producer or removed as dead. Remaining FIX blocks are
genuine pipeline steps, each with a known ability-count blast radius.

## Fundamental structural issues (from the deep-refactor review)
> **Re-verified 2026-09-28 — this list is partly STALE.** Items 1 and 5 were
> already done before this session and should not be re-attempted. Read the
> verdicts, not the original claims.

1. **`_fill_defaults` re-extracts fields `parse_action` already set** —
   ~~open~~ **ALREADY FIXED (pre-2026-09-28).** `_fill_defaults` states the
   contract in its own docstring: `parse_action` owns ALL extraction, and
   `_fill_defaults` "ONLY fills defaults for fields left unset". Every
   re-extract there is guarded by `not in action` / `is none`; `source` and
   `destination` are additionally threaded in as `_cached_source` /
   `_cached_dest` so they are not recomputed. Pinned by
   `test_parse_action_scans_source_and_destination_once`. The docstring also
   forbids adding extraction here, so the confusion it caused is designed out.
2. **`_fill_defaults_move_cards` is source inference** — **still open.** ~309
   lines (the "~131" figure is stale). Takes `_cached_source`/`_cached_dest`,
   so it no longer recomputes; what remains is the inference itself.
3. **`_walk` + `_propagate_context` = two full tree walks** — **still open.**
   `_walk` (11 sub-walkers) during normalization; `_propagate_context`
   (~240 lines) after `_process_pre_fix`. Merging needs the timing dependency
   understood first — propagate consumes pre_fix output.
4. **`_process_pre_fix` is ~340 lines of compensating patches** — mostly
   **dissolved**; see the triage table above. What remains are genuine
   pipeline steps with a known ability-count blast radius.
5. **Double/triple extraction of the same fields** — ~~open~~ **ALREADY FIXED,
   same refactor as item 1.** `extract_source`/`extract_destination` are
   computed once per `parse_action` call and passed down.

## Completed work (history)

### Session: extract script single-owner parsing (2026-08-24)
- extract_card_abilities.py now calls parser's real `parse_ability()`;
  deleted the divergent weaker inline copy
- Added `normalize_multiline()` (old normalize collapsed `\n` choice bullets)
- Hardened condition back-fill: effect text only, only when no condition
  exists, leading-gate only (was re-scanning cost text and double-gating)
- Generic 「プレイに際し…コストはNになる」 handler (`_try_play_time_cost_set`)
  replaced the LL-bp7-001-specific override
- Deleted dead code: FieldExtractor, _DEBUG_LOG plumbing, segment_clauses
  Stage-A IR (+tests), compile_abilities vocab/encode block, typo'd patterns

### Session: dedup + helper extraction (PARSER_DEBT)
- Extracted shared helpers: `detect_exclude_self()`, `extract_heart_colors_from_text()`,
  `detect_duration_code()`, `extract_cost_operator()`; replaced all inline copies
- Unified dual-path condition enrichment → `_enrich_condition_common()`
- Broke up god-functions into named helpers:
  - `_fill_defaults` 498 → ~260 lines (`_fill_defaults_move_cards`, `_fill_defaults_count_and_refine`)
  - `_extract_generic_fields` 366 → ~250 (`_extract_comparison_fields`, `_extract_resource_fields`)
  - `_process_pre_fix` 476 → ~340 (`_fix_sequential_chain`, `_fix_condition_enrichment`)
  - `_process_post_fixes` ~338 → ~175 (`_fix_conditional_on_result`)
- Removed dead code (duplicate re_yell registration, dead `dur_effect` check)

### Session: overlap cleanup (PARSER_REFACTOR)
- Removed `parser_utils`/`parser.py` overlaps (dead cost pattern, local POSITION_KEYWORDS)
- Deduplicated `_walk` sub-walkers (removed superseded propagation paths)

### Session: validation merge + deep refactor (PARSER_DEEP_REFACTOR)
- Deleted `_validate_output()` from extract_card_abilities.py (~240 lines);
  merged into single `_validate_semantic()` with recursive tree walking
- `_try_per_unit` 350 → 25 lines; `parse_action` reduced by 130 lines

## Untangle plan status

The former `PARSER_UNTANGLE_PLAN.md` is consolidated here. All planned phases
were executed or explicitly rescoped with byte-diff and test evidence. The
remaining live debt is the structural work listed above; the old phase-by-phase
plan is retained in Git history rather than maintained as a second status file.
