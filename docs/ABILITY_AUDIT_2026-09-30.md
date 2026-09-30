# Ability audit — printed text vs. parse vs. engine vs. tests

Audit date: 2026-09-30. Scope: **the abilities themselves**, checked in that order —
what the card prints (`cards/cards.json`) → what the parser produced
(`cards/abilities.json`) → what the engine can execute (`engine/src`) → what the test
claims. A test is only evidence if the parse can express the clause and the engine can
execute it; a green test on a silently-dropped clause is worse than no test.

Baseline established: **3983 passed, 5 failed** (the 5 pre-date this pass; 3 are in a
file that did not compile, see §5).

Method: two corpus scans rather than file-by-file reading.

1. Every numeric **comparison clause** in every printed ability (`N枚以上`, `N以下`, `Nより`)
   was matched against every numeric scalar anywhere in that ability's parse. A number in
   the text that appears nowhere in the parse is a **lost clause**.
2. Every 「`〜`をNつ以上持つ」 heart-count threshold across all 2639 cards, to size the blast
   radius of finding B1.
3. For each hit, the engine path was traced to establish whether the *engine* could execute
   the clause at all — which separates "parser gap" from "engine gap".

Findings are graded:
**A** = ability does not work as printed · **B** = test passes but cannot detect the defect
it names · **C** = missing edge case on an otherwise-correct ability · **D** = latent.

## Outcome

| # | Finding | Status |
|---|---|---|
| A1 | 「heart04を**2つ以上**持つ」 count dropped — `_build_reveal_add_discard` never called `_add_heart_color_threshold` | **FIXED** (parser), 1 node, proven both directions |
| A2 | 「**スコア6以上**のライブカード」 filter dropped entirely | **FIXED** (parser), 1 node, proven both directions, 4 new tests |
| B1 | A1's only test passes for a wrong reason | **FIXED** — the discriminating test goes red without the fix |
| C1 | `PL!N-bp4-028-L` never pinned 5 distinct names | **FIXED** — 5 → +1, engine already correct |
| C2 | `PL!-pb2-038-L` non-stackable tested in one zone only | **FIXED** — one per zone still +1, engine already correct |
| D1 | `parenthetical` decoded then never read | open, engine |
| D2 | `exclude_self: true` inert on the formation change; parser wrong | open, parser |

Verification: **engine 4004 passed / 0 failed**; **parser 124 passed / 0 failed**
(`47 + 24 + 4 + 44 + 5` across the five suites in `cards/ability_extraction/tests`);
`cards/test_inventory.py` regenerated.

`test_inventory.py --check` reports **13 soft-guard ratchet failures**, none in a file
touched by this audit (they are concurrent bot/C-port work).

---

## A1 — 「ハートにheartNNをNつ以上持つ」 is dropped by the parser (13 card prints)

**The defect.** `_add_heart_color_threshold` (`cards/ability_extraction/parser.py:8403`):

```python
m = re.search(rf"{HEART_ICON_ID}を(\d+)(?:個)?以上", text)
```

The counter word is optional but only accepts **個**. The corpus also prints **つ** for the
same clause. Every 「Nつ以上」 heart threshold therefore loses its count.

**This is a parser gap, not an engine gap** — the engine already implements the filter
correctly. `CardFilter::heart_color_count` is decoded (`effect_decoder_gen.rs:181`,
`core/card.rs:1088`) and enforced at `ability/util.rs:1388-1402`, and it is live on the
`look_and_select` select path (`effects/look.rs:45` → `CardFilter::from_effect`).

**Proof that only the counter word differs** — two cards, one clause:

| card | printed | parse |
|---|---|---|
| `PL!S-pb1-013-N` 黒澤ダイヤ | 「ハートにheart04を**2個以上**持つメンバーカード」 | `heart_color_count: 2` ✅ |
| `PL!S-bp5-007-AR` 国木田花丸 | 「ハートにheart04を**2つ以上**持つメンバーカード」 | *(field absent)* ❌ |

**Blast radius** — 13 card prints print 「Nつ以上」 after a heart icon:

```
PL!-bp4-023-L / -SRL      PL!N-bp3-027-L        PL!N-bp5-028-L
PL!S-bp5-007-AR / -P / -R PL!S-pb1-013-N        PL!S-pb1-014-N
PL!S-pb1-015-N            PL!SP-bp4-024-L / -SECL / -SRL
```

**The concrete ability that does not work as written.** `PL!S-bp5-007-AR/-P/-R` 国木田花丸,
cost 13, prints:

> ライブ成功時 自分のデッキの上からカードを4枚見る。その中から**ハートにheart04を2つ以上持つ**
> メンバーカードを1枚公開して手札に加えてもよい。残りを控え室に置く。

Its parse:

```json
"select_action": {
  "heart_colors": ["heart04"],
  "optional": true,
  "cost_limit_operator": ">=",     ← orphan: no cost_limit anywhere
  ...
}
```

Two things wrong at once. `heart_color_count` is missing, so the filter degrades to
*presence* of heart04 — `check_heart_colors` (`util.rs:956-976`) is a `contains_key` test and
never a count. **A member printing exactly one heart04 is offered.** And the stray
`cost_limit_operator` is inert: `card_matches_cost_limit_op` (`util.rs:929-953`) returns
`true` unconditionally on `cost_limit: None` and never reads the operator.

### A1 — the actual bug site, and why my first fix did nothing

My first attempt changed the regex in `_add_heart_color_threshold` to accept `つ`. Regenerating
produced a **zero-line** diff. That was the useful signal: the counter word was never the
problem on this card.

The clause is built by `_build_reveal_add_discard` (`parser.py:8345`), the 「1枚公開して手札に
加えてもよい」 builder. It sets `heart_colors` at `:8359-8363` and **never calls
`_add_heart_color_threshold` at all** — `_enrich_from_text` has always called it, which is
exactly why `PL!S-pb1-013-N` (「2個以上」, same builder family, different route) kept its
`heart_color_count` while `PL!S-bp5-007-AR` (「2つ以上」) did not. Two independent omissions:
the route and the counter word.

Both are fixed. The regex accepts `個|つ|枚` (defence in depth for the other route), and
`_build_reveal_add_discard` now calls the helper.

**Blast radius, measured not assumed** — the entire `cards/abilities.json` diff is
`+ "heart_color_count": 2` on the 国木田花丸 `select_action`, plus the two stamps.

---

## A2 — 「スコアN以上のライブカード」 is dropped entirely (2 card prints), no engine support

**The defect.** `PL!-PR-018-PR` 東條 希, cost 15, prints:

> 登場 自分の控え室から、**スコア6以上の**ライブカードを1枚手札に加える。

Its parse is a bare fetch:

```json
{"source":"discard","destination":"hand","count":1,
 "card_type":"live_card","target":"self","action":"move_cards"}
```

No score filter of any kind. Same ability node as `PL!HS-PR-032-PR` セラス 柳田 リリエンフェルト.

**The engine offers any live card in the owner's waitroom.** `CardFilter`
(`ability/util.rs:1209-1270`) has no score dimension — I read the full struct, the full
`from_effect`, and the full `matches`; there is no score term in any of them. `has_filter()`
likewise. So this is not a parser-only gap: **nothing in the engine can filter a selectable
card by printed score today.**

The nearest existing mechanism is `card_matches_cost_limit_op` (`util.rs:968-993`), which
resolves the value as `c.cost.or(c.score)` — for a live card that *is* the printed score. So
`cost_limit: 6, cost_limit_operator: ">="` would express this clause correctly and no new
engine field is needed.

**No test anywhere names `PL!-PR-018-PR`.** The ability has never been driven.

### A2 — the fix, and why it needed no engine change

`_add_score_threshold` maps 「スコアN以上/以下の<live card>」 onto `cost_limit` +
`cost_limit_operator`. For a live card `cost_limit` *is* the score, because
`card_matches_cost_limit_op` (`util.rs:968-996`) compares `c.cost.or(c.score)` and a live card
carries no cost. Traced end-to-end through
`execute_move_cards → resolve_cards_from_source → resolve_from_standard_zone →
take_cards_from_standard_zone → matching_indices → matches → card_matches_cost_limit_op`,
with the filter re-applied at answer time (`choice.rs:1034-1036`) and again in the executor
(`move/mod.rs:1697-1711`). No bypass on the non-interactive path.

Hooked in **twice, and the first hook alone did nothing** — the same trap as A1:

* `_finalize_effect` (the registry-handler path) — **zero-line diff**;
* the `parse_action` fallback tail of `parse_effect` — **this is the one that bites**.

This clause reaches the engine through `parse_action`, not the handler registry. A single
central post-pass in each of the two exits is more robust than teaching each builder.

Guards, so the rule cannot misfire:

* negative lookahead `(?!の合計)` keeps 「スコアの**合計**が6以上」 (an aggregate over a zone)
  out — different quantity, handled by `comparison_condition`;
* only fires when `card_type` is a live card, because a member has a cost and no score;
* never overwrites an existing `cost_limit`.

Blast radius: `+ "cost_limit": 6, + "cost_limit_operator": ">="` on that one node.

The sibling `PL!S-bp3-008-P/-R` 小原鞠莉 prints 「それがスコア6以上の『Aqours』のライブカードの
場合」 — the score there gates the *effect* on the already-recovered card, not the selection.
Different shape, still unhandled, **not fixed here**; recorded so it is not mistaken for
covered.

---

## B1 — the only test for A1's card passes for a wrong reason

`engine/tests/test_modules/effects/look_select/search_deck/look_four_take_member_with_double_heart04_test.rs`

Fixtures, read from `cards.json`:

| const | card | base_heart |
|---|---|---|
| `DIA_H04X2` | `PL!S-PR-016-PR` 黒澤ダイヤ | `heart02:1, heart04:2` |
| `CLEAN_KOTORI` | `PL!-pb1-021-PR` 南 ことり | `heart03:2, heart06:1` — **zero heart04** |

The negative test `..._no_double_heart04_member_sends_all_four_to_waitroom` builds a deck of
Kotori and `PL!-sd1-010-SD`, asserts nothing reaches hand, and its message reads
「no double-heart04 member among the four」.

**Kotori has no heart04 at all**, so she is excluded by the *colour-presence* filter alone.
The 「2つ以上」 clause — the entire point of the test name — is never evaluated, and the
assertion would still pass with the count parser bug present (it is passing right now). A
card printing exactly **one** heart04 would be offered and no test in the tree would notice.

**Proven, not argued.** With the parser fix disabled and the bytecode recompiled, the two
existing tests still report `ok`. That is the blindness. The new test, same build:

```
..._the_two_heart_threshold_excludes_a_single_heart04_member ... FAILED
  left: Some(2)   right: Some(1)
```

— the engine offered both the 2-heart04 and the 1-heart04 member. With the fix restored, all
three pass. `PL!HS-bp5-009-N` 日野下花帆 (`base_heart: {heart04: 1, heart06: 1}`, no abilities)
is the fixture; 281 members in the corpus print exactly one heart04, so the boundary is
reachable. The test pins both fixtures' printed counts as preconditions, because a substituted
printing could put a card on the qualifying side.

There is a second, quieter reason the file was blind: it never reads `filtered_indices`. It
drains with `select_indices(&[0])`, which indexes the *filtered* list, so "how many candidates
were offered" is never measured. `pending_choice_option_count()` exists for exactly this and
was unused.

---

## C1 — `PL!N-bp4-028-L` never pins the boundary between +1 and +2

`stars we chase`, score 2, prints:

> 自分の控え室にカード名の異なる『虹ヶ咲』のライブカードが**4枚以上**ある場合、このカードの
> スコアを＋１する。**6枚以上**ある場合、代わりにスコアを＋２する。

Parse is correct (`conditional_alternative`, conditions at 4 and 6, both carrying
`distinct: card_name`, `group_names: ["虹ヶ咲"]`, `card_type: live_card`).

`distinct_waitroom_lives_pl_n_bp4_028_l_test.rs` covers 3 → 0, 4 → +1, 6 → +2. **5 is never
staged.** Two implementations pass that suite identically:

* `if n >= 6 {+2} else if n >= 4 {+1}` — correct;
* `if n >= 5 {+2} else if n >= 4 {+1}` — off by one, gives +1 at 5, and is **untested**.

That is the same 6-instead-of-5 defect class the suite already catches for other cards; here
it has no guard.

---

## C2 — `PL!-pb2-038-L` non-stackable is tested in one zone only

`嵐のなかの恋だから`, prints 「この効果は重複しない」, parsed as `non_stackable: true` on a
`live_total` +1.

`pb2_two_mus_members_live_or_success_zone_score_pl_pb2_038_l_test.rs::two_copies_do_not_stack`
stages both copies in the **same** zone (`success_live_card_zone`). But the card's own zone
clause is a **disjunction** — 「自分のライブカード置き場**か**成功ライブカード置き場にあり」,
parsed `any_location: true, locations: [live_card_zone, success_live_card_zone]`. The file
also proves both zone branches work *individually*.

So the untested combination is the one the card text uniquely permits: **one copy in each
zone simultaneously**, which is a real board state (this live performing, a prior live
already succeeded). If `non_stackable` de-dupes per-zone rather than per-ability, that
configuration scores **+2** against a printed +1, and nothing in the tree would catch it.

---

## D1 — `parenthetical` is decoded and then never read

Every hit in `engine/src`: declared `core/card.rs:1148`, JSON reader `core/card.rs:1752`,
bytecode decode `effect_decoder_gen.rs:232`, mirror struct `:359`, clone into `EffectFilter`
`:593`. **No accessor, no reader, no executor.** The field is a pass-through from
`abilities.json` to dead.

The one parenthetical the engine *does* honour — 「(対戦相手のカードの効果でも発動する。)」 —
is handled by a dedicated `fires_on_opponent_effects` bool precisely so the JP text need not
be re-matched. Restrictions that land in `parenthetical` get no such treatment.

`PL!SP-sd2-001-SD2` 澁谷かのん is the live case:

> (メンバーをそれぞれ好きなエリアに移動させる。**この効果で1つのエリアに2人以上のメンバーを
> 移動させることはできない。**)

The capacity clause is parked verbatim in `parenthetical`. Nothing enforces it. The
restriction currently *holds* — but incidentally, from two structural facts, not from the
rule: the stage is a fixed `[i16; 3]`, and `planned_zones` removes an already-claimed zone
before each subsequent member's prompt (`effects/position.rs:689`). Any refactor that relaxes
`planned_zones` or widens the stage silently permits 2-in-an-area, with no test to catch it.

---

## D2 — `exclude_self: true` on that same formation change is inert, and the parser is wrong

The parse carries `exclude_self: true`, but `execute_position_change`'s `multiple_targets`
collection loop (`effects/position.rs:328-332`) pushes **every** occupied slot
unconditionally:

```rust
for i in 0..3 { if player.stage.stage[i] != -1 { card_ids.push(player.stage.stage[i]); } }
```

Nothing consults `exclude_self`. Here the engine and the card text agree by accident, because
the printed clause is 「自分のステージにいるメンバー」 — with no 「このメンバー以外」 — so
self *should* move. **The parser is the wrong one**: it added an exclusion the text does not
state, and the engine's failure to honour it is the only reason the card behaves correctly.

Recorded because the pair (wrong parse + engine ignoring the field) means this
`exclude_self` is dead weight; if the engine ever starts honouring it on this path, 澁谷かのん
will silently stop moving herself and no test covers that card.

---

## 5. Pre-existing build breakage (fixed, not mine)

The suite **did not compile** at audit start. Two errors in test files, both from a
concurrent session:

1. `pb4_gated_activation_score_total_kidou_pl_pb4_002_sec_test.rs:59` —
   `.and_then(|c| c.score)` on an `Option<Card>` yields `Option<Option<u8>>`; the outer
   `unwrap_or_else` left an `Option<u8>` where `i32::from` was called. Fixed to `.map(...)`
   plus a second unwrap.
2. `pb2_blade_per_member_activated_from_wait_this_turn_pl_pb2_010_p_plus_test.rs:98` —
   `record_activation(&mut game, game.id(c))` is a double borrow. Fixed by hoisting the id.

A third surfaced immediately after: `Action` has no `card_id` field; it lives at
`Action.parameters.card_id` (`game/game_setup.rs:282`). Fixed, with the gate-honouring
`disabled` flag also checked so a greyed-out action does not count as offered.

**Not investigated, not mine:** the 5 remaining failures —
`pb2_baton_by_cost_15_mus_…009_pp::{a_cheap_mus_baton_touch_does_nothing,
an_expensive_non_mus_baton_touch_does_nothing}` and all 3 in
`pb4_gated_activation_score_total_kidou_pl_pb4_002_sec_test`.

---

## 7. My own new test had the disease it was written to detect

`a_live_of_score_five_is_below_the_bound_and_is_not_taken` asserted that hand was unchanged
and the waitroom still held both non-qualifying lives. Run against the **unfixed** parse it
**passed**.

Why: with the score clause dropped, both score-5 lives become candidates, `count: 1` with two
candidates raises a `SelectCard`, and `drain_auto_ability_choices` breaks on anything that is
not a `SelectAutoAbility` — so the prompt is never answered. **Nothing moved not because the
filter refused, but because the selection is still pending.** Both assertions are satisfied by
a pending prompt.

Three of the four new tests went red on the unfixed parse; this one did not, and it was the
one that looked most obviously correct. Fixed by asserting the thing that actually
discriminates:

```rust
assert!(!game.has_pending_choice(),
    "zero candidates qualify, so the recovery must resolve SILENTLY. A pending choice here \
     means the score-5 lives were offered as candidates.");
```

Zero qualifying cards *is* a silent no-op (`SelectionOutcome::Skip`), so no prompt may
survive. With that added, all four go red without the fix and green with it.

The general lesson is the one this audit keeps hitting: **an assertion on an ABSENCE is
satisfied by a stalled prompt as readily as by a correct refusal.** Pair every absence with
either a positive twin or a statement of the prompt state.

Two smaller self-corrections, both caught by preconditions rather than by reading:

* my first 028-L premise check allocated fresh ids *inside* its own comparison loop
  (`TestGame::id` hands out a new instance per call) and failed on a correct card;
* `PL!S-pb2-019-L` is in `cards.json` but **not in the engine's baked database**, so
  `game.id` panics on it. Swapped for `PL!N-bp5-029-L`, which existing tests prove present.
  Worth knowing generally: `cards.json` ⊃ the baked DB, so a card number is not proof the
  engine can resolve it.

---

## 8. Fix order

| # | Item | Kind | State |
|---|---|---|---|
| 1 | `parser.py` `_build_reveal_add_discard` → call `_add_heart_color_threshold`; regex accepts `個\|つ\|枚` | A1 parser | **done**, 1 node |
| 2 | `PL!-PR-018-PR` スコア6以上 → `cost_limit`/`cost_limit_operator` | A2 parser | **done**, 1 node, 4 tests |
| 3 | heart04×1 discriminating test for `PL!S-bp5-007-R` | B1 | **done** |
| 4 | 5-distinct boundary case for `PL!N-bp4-028-L` | C1 | **done** |
| 5 | one-copy-per-zone non-stackable case for `PL!-pb2-038-L` | C2 | **done** |
| 6 | `PL!S-bp3-008-P/-R` 「それがスコア6以上の場合」 — effect gate on the recovered card | new | open |
| 7 | `parenthetical` consumption / area-capacity enforcement | D1 | open, engine |
| 8 | drop the bogus `exclude_self` from `PL!SP-sd2-001-SD2` | D2 | open, parser |
