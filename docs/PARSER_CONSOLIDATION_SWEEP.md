# Parser consolidation sweep — fragmentation audit

Date: 2026-09-30. No code changed. This is a work list.

**The rule being applied**, as specified:

* **N inputs → 1 output = convergence. GOOD.** Many phrasings funnelling into one shape is
  redundancy, and redundancy is desirable: it is what makes a new wording work without
  touching the engine. Leave these alone.
* **1 input → M outputs = fragmentation. BAD.** One printed concept producing several
  different shapes is the dangerous direction, because the shapes can disagree — and they
  do. Merge these.

Every finding below is ranked by how badly the fragmentation has already misbehaved. Three of
the five are not hypothetical: they are the direct cause of defects I fixed earlier today
(A1, A2, D2), documented in `docs/ABILITY_AUDIT_2026-09-30.md`.

Method was manual — read the dispatch tables and the builders, then probe individual
extractor functions on individual strings to check whether code reading and actual behaviour
agreed. Where a claim is inference rather than measurement it says so.

---

## F1 — `exclude_self`: one concept, nine extraction sites, 5 of 7 samples disagree

**The single worst instance of the bad direction, and it is the concept that just produced
defect D2.**

`detect_exclude_self` (`parser_fields.py:108-109`) is the canonical form and it is narrow:

```python
def detect_exclude_self(text: str) -> bool:
    return "このメンバー以外" in text or re.search(r"ほかの.*?(?:メンバー|カード)", text) is not None
```

But nine call sites each apply their own widening on top of it, and **no two widen the same
way**:

| site | guard used |
|---|---|
| `parser.py:2852` | `detect_exclude_self` **and** action not in `(gain_resource, select, heart_selection)` |
| `parser.py:3811` | bare `detect_exclude_self` |
| `parser.py:3340`, `:4330`, `:4801`, `:5765` | `detect_exclude_self` **or** `"このカード以外" in text` |
| `parser.py:7230` | the above **or** `自分以外` **or** `他の.*?(?:メンバー\|カード)` **or** `これを除く` |
| `parser.py:8079` | `"以外" in text or "ほかの" in text or "他の" in text` — bare 以外 |
| `parser.py:11390` | context propagation with an `is_self_buff` carve-out |
| `parser.py:2949`, `:4576` | the `position_change` default (source of D2) |

**Measured.** The same seven printed clauses against four of the guards:

| text | `detect` | site 3340 | site 7230 | site 8079 | |
|---|---|---|---|---|---|
| このメンバー以外のメンバーを1人選ぶ | True | True | True | True | agree |
| このカード以外のカードを1枚捨てる | False | True | True | True | **disagree** |
| 自分のステージにいるほかのメンバーを1人ウェイトにする | True | True | True | True | agree |
| 『μ's』以外のメンバーを1人選ぶ | False | False | False | **True** | **disagree** |
| 自分のステージにいる他のメンバーを1人選ぶ | False | False | True | True | **disagree** |
| 自分以外のメンバーを1人選ぶ | False | False | True | True | **disagree** |
| これを除くメンバーを1人選ぶ | False | False | True | **False** | **disagree** |

Two failure directions at once:

* **False positive.** 「『μ's』以外のメンバーを1人選ぶ」 — excluding a *group* is not
  excluding *yourself*. Site 8079's bare `"以外" in text` says yes. Worse, this text is
  already handled correctly by `apply_group_exclusions` (`parser_fields.py:120-130`), which
  sets `exclude_group_names`; so 8079 can put a bogus `exclude_self` next to a correct
  group exclusion.
* **False negative.** 「これを除くメンバーを1人選ぶ」 — 7230 catches it, the broad 8079
  misses it. The broad site is not a superset of the narrow one.

**Why it bit us.** D2 was `position_change` gaining `exclude_self: true` on five
whole-stage formation changes. That was a *tenth* rule (`:2949`/`:4576`) contradicting the
other nine, and it survived because the multi-target collection loop ignores the field while
the destination helper honours it — the disagreement only showed up in one code path.

**To do**

1. One function, one answer. `resolve_exclude_self(text, action_kind, *, context) -> bool`,
   holding the **union** of every form currently recognised, plus an explicit list of forms
   that must *not* trigger it (notably `『X』以外`, which is a group exclusion).
2. Delete the per-site widening. The nine sites become one call.
3. Decide and write down the `is_self_buff` / `gain_resource` carve-out once, instead of
   three different carve-outs.
4. Add a table-driven test over the seven clauses above asserting the same result regardless
   of which builder processes the text. Right now no such test can exist, because the answer
   legitimately depends on the path.
5. Decide whether `position_change`'s "must move to a different area" default belongs in
   `resolve_exclude_self` at all, or should be a separate field. It is a different rule
   (no-op prevention) wearing the same flag.

---

## F2 — `sequential`: one concept, 37 producers

* **12** declared `EffectPattern(action="sequential")` rules (lines 8331, 9132, 9231, 9346,
  9371, 9802, 10360, 10395, 10567, 10614, 10799, 10983)
* **25** sites that build the shape by hand with `result["actions"] =` (lines 2128, 6706,
  8275, 8325, 9103, 9205, 9321, 9366, 9792, 9901, 9981, 10355, 10390, 10450, 10553, 10607,
  10782, 11855, 12409, 13459, 13500, 13507, 13699, 13767, 13808)

Each is a different connector with its own splitting logic: 、, 。, さらに, し, を得て, その後,
そうした場合, 相手は, これにより, もう一度エール, なお, … The *output* concept is identical
in all 37 cases, which is convergence at the output and fine. The problem is the divergence
in what a `sequential` node is allowed to carry:

* `_set_both_discard_until` (`:10540-10553`) sets `target: "both"` and `multiple_targets: True`
  **on the parent**, alongside `actions`.
* `_set_re_yell` (`:10607-10609`) hoists a promoted `condition` onto the parent.
* `_set_opponent_action` (`:8277`) does `result.clear()` in its no-rest branch, discarding the
  pattern's own declared action and adopting the child action's — so the same rule emits two
  different shapes depending on the text.
* `_set_furthermore` (`:9321`) leaves a hoisted `condition` behind *and* the child's
  condition is `pop`ped — parent and child disagree about who owns it.

So `sequential` is a bag, not a shape, and the engine must special-case each producer.

**Which splitter wins is decided purely by registration index.** The module says so itself at
`parser.py:11098-11099`: *"a NARROWER phrase must come before a broader one that would
otherwise swallow it."* That is a load-bearing invariant maintained by hand across 37 rules.
Insert one narrow rule and some unrelated card changes shape.

**To do**

1. One `make_sequential(text, legs, *, parent_condition=None, parent_extras=None)` used by
   all 37 sites, so the shape and the field policy are decided once.
2. Write down the parent-field policy explicitly: which extras may live on a `sequential`
   (target/multiple_targets today) and which must live on the legs. Today that is implicit
   and inconsistent.
3. Decide the `condition` ownership rule once — parent-hoisted vs per-leg — and apply it
   everywhere. This is the one most likely to change golden output, so it needs its own diff
   run.
4. Add a registry assertion: two rules whose matchers both accept the same text should fail
   the test suite, not silently resolve by index. That converts the hand-maintained ordering
   invariant into a checked one.

---

## F3 — the reveal-and-take mechanic has four different output shapes

One printed mechanic — *reveal cards from the deck, put one in hand, put the rest in the
discard* — is emitted as four different action shapes, all four of which the engine supports
(`look_and_select` 9 refs, `select_cards` 38, `reveal_until_live_card` 4, `discard_remaining` 14):

| builder | output |
|---|---|
| `_build_reveal_add_discard` (`:8385`) | **one** `select_cards` node, with `reveal: True` + `discard_remaining: True` |
| `_set_reveal_until_chosen_card` (`:9092`) | **four** nodes: `select` + `reveal` + `move_cards{looked_at→hand}` + `move_cards{looked_at_remaining→discard}` |
| `_set_reveal_until_live` (`:9204`) | **three** nodes: `reveal_until_live_card` + `move_cards` + `move_cards` |
| `_try_look_and_select` (`:9066`) | one `look_and_select` node |

"Put the rest in the discard" is a **boolean flag** in the first and an **explicit third/fourth
node** in the other three. Same printed meaning, two encodings.

**This is exactly why A1 was hard.** Defect A1 was a heart-count threshold dropped because
`_build_reveal_add_discard` set `heart_colors` without ever calling
`_add_heart_color_threshold`. Under one shape that is a one-line fix. Under four shapes it is
a one-line fix **per shape**, and nobody can tell by reading one card whether the other three
are also missing it. I only found the other three by grepping for builders, not from the card
text.

**To do**

1. One builder for the mechanic. `reveal_then_take(source, count, take_filter, rest_to)`;
   "until a live card" and "until the chosen card" become parameters (`until=`), not separate
   rules.
2. Pick ONE encoding for "rest to discard" — a node, not a flag — and migrate the other three
   to it. Keep the flag reading in the engine for as long as both exist, so the migration is
   not a big-bang change.
3. Make every filter clause (`heart_colors`, `heart_color_count`, `cost_limit`, `score`,
   `or_card_types`, `require_all_heart_colors`) get extracted **once**, centrally, for the
   whole mechanic rather than per builder. See F4.

---

## F4 — filter enrichment is opt-in per builder, not automatic — the structural cause of A1 and A2

This is the root cause under F3. Clause extraction is a set of helpers that each builder
**chooses to call**:

| helper | call sites |
|---|---|
| `_enrich_from_text` | 6 — lines 1618, 8405, 8502, 8781, 8803, 8815 |
| `_apply_card_property_filter` | 4 — 8368, 8417, 8782, 8823 |
| `_add_heart_color_threshold` | 4 — 8407, 8444, 8515, 8779 |
| `_add_or_card_types_if_needed` | 4 — 8431, 8435, 8780, 8804 |
| `_apply_ability_filter` | 2 — 8472, 8539 |
| `_add_score_threshold` | 4 — 1393, 1504, 8538, 8552 (the two I added this session) |

Every one of those is inside **8368–8552** — a single builder family. Meanwhile the module
registers **~48 structural effect rules** and calls `_fill_defaults` from 20 sites, and there
are ~19 `extract_cost_limit` call sites scattered to line 8527. Handlers that construct their
own `select` / `move_cards` node get **no** enrichment at all.

Concrete proof, from the code read: `_set_reveal_until_chosen_card:9093` rolls its own cost
regex rather than calling the shared helper —

```python
cost_match = re.search(r"コスト(\d+)以上", text)
```

So a heart filter printed in that rule's text would be dropped, and a cost filter printed in
the `select_cards` builder's text would be picked up. Same printed card, different clause
survival, decided by which rule won.

**This is why my first A1 fix produced a zero-line diff.** I fixed the shared helper; the
card went through a different builder. **This is why A2 needed two hooks.** I added the score
threshold to `_finalize_effect` (zero-line diff) and then again to the `parse_action`
fallback tail (one line). The clause is one thing and it now has two application points
because the effect graph has two exits.

**To do**

1. Make enrichment **automatic**: after an effect tree is built, walk every node that has a
   `card_type` or a `select`-like role and run the clause extractors against that node's own
   `text`, once, in one place. Builders then never call helpers again and a new clause is one
   edit, not one edit per builder.
2. That automatically fixes the missing-`_finalize_effect`/missing-fallback symmetry that
   cost me both A1 and A2 — the whole class stops existing once there is no per-builder
   choice to forget.
3. Retire the duplicated ad-hoc extractors (F5) as part of this, so there is one
   implementation per clause and the walker can reach it.
4. Regression guard: for every card in `cards.json`, assert that each printed filter clause
   present in a node's `text` has a corresponding field on that node. That is the check that
   would have caught A1, A2 and D2 the day they were written.

---

## F5 — cost comparison: one concept, four implementations, two operator vocabularies

| implementation | where | notes |
|---|---|---|
| `extract_cost_limit` | `parser_utils.py:675` | ordered list of **8** patterns (`_COST_LIMIT_PATTERNS:152-164`); value only, **no operator** |
| `extract_cost_limit_with_operator` | `parser_utils.py:684` | a **separate hardcoded** regex `コスト(\d+)(以上\|以下\|より大きい\|より小さい\|未満)` + its own `op_map`; does **not** handle `元々のコスト` or `コストNの` |
| inline cost regex | `parser.py:9093` | `re.search(r"コスト(\d+)以上", text)` — neither helper |
| bespoke cost **range** | `parser.py:492` | `コスト(\d+)以上(\d+)以下` — a fourth concept, value-pair |

**Measured divergence** — the same six printed shapes through each:

| text | `extract_cost_limit` | `_with_operator` | inline @9093 | `extract_operator` |
|---|---|---|---|---|
| コスト3以上のメンバー | 3 | (3, '>=') | 3 | '>=' |
| コスト3のメンバー | 3 | **None** | **None** | **None** |
| 元々のコストが3以上のメンバー | 3 | **None** | **None** | '>=' |
| コスト3より大きいメンバー | **None** | (3, '>') | **None** | **None** |
| 3コスト以上のメンバー | 3 | **None** | **None** | '>=' |
| コスト3以上5以下のメンバー | 3 | (3, '>=') | 3 | '>=' |

No two of the four agree on all six, and on 「コスト3より大きい」 the primary helper misses
it while the secondary catches it. `extract_cost_limit` also returns a bare value for
「コスト3の」, which callers pair with `extract_operator(text) or "="` (`:2805`, `:7006`) —
so the *pair* of helpers is the unit that matters, and it is inconsistent about which shapes
it recognises.

**Honest status: latent, not currently producing wrong output.** I checked the corpus:

| printed form | cards |
|---|---|
| コストNより大きい | **0** |
| コストNより小さい | **0** |
| コストN未満 | **0** |
| Nコスト以上 | **0** |
| 元々のコストがN以上 | 1 — `PL!-bp6-019-L` Music S.T.A.R.T!! |

And I read that one card's parse: it is **correct** (`cost_limit: 17`,
`cost_limit_operator: ">="`, `original_value: true`). So nothing is wrong today. But two of
the four implementations miss 「より大きい/より小さい/未満」 entirely, so the first card
printed that way loses its filter with nothing to notice — the identical failure mode to A2.

**Prior evidence this has already broken once:** the comment at `parser.py:3791-3792` —
*"Parser previously missed this because `extract_cost_limit_with_operator` requires 以上/以下"* —
records the two helpers having diverged, discovered the hard way.

**To do**

1. One `extract_cost_comparison(text) -> {value, operator, is_original, is_range}` built on
   the union of all four vocabularies, with the ambiguity resolved deliberately (e.g. 「コスト3の」
   → `value: 3, operator: "="`; 「3コスト以上」 → `value: 3, operator: ">="`).
2. Delete `extract_cost_limit_with_operator`'s hardcoded regex, the inline regex at 9093, and
   the range regex at 492 — or express the range as a first-class return of the single
   function. `extract_cost_limit`/`extract_operator` become thin wrappers so the 19 existing
   call sites do not have to change in one commit.
3. Fold the `(\d+)\s*合計` pattern out of `_COST_LIMIT_PATTERNS` if it is really a
   score/heart total rather than a cost — **this one I have not verified**, it needs a corpus
   check before anyone touches it.
4. Add the seven-shape table above as a test on the merged function.

---

## F6 — count extraction: two implementations that disagree

* `extract_count` (`parser_utils.py:167`) — ordered `_COUNT_PATTERNS`, understands 枚/人/つ/個,
  prefers `N枚まで`. **17** call sites.
* `_count_from_cards` (`parser.py:5124`) — `re.search(r"(\d+)枚", text)`, default **1**.

Measured:

| text | `extract_count` | `(\d+)枚` / default |
|---|---|---|
| カードを3枚引く | 3 | 3 |
| **メンバーを2人選ぶ** | **2** | **1** |
| 最大5枚まで | 5 | 5 |
| カードを1枚見て_member_2人 | 1 | 1 |

They agree except on 人, where `_count_from_cards` silently returns its default **1** — a
count of 2 read as 1, with no error. Same concept, two implementations, one of which is wrong
for a whole counter class.

**To do.** Either delete `_count_from_cards` in favour of `extract_count` (checking its two
or three call sites first — the `default=1` behaviour may be load-bearing there), or give it
the full pattern list. Do not leave both.

---

## Convergence — deliberately NOT on the work list

Per the rule above, these are the good kind and should not be "cleaned up":

* **`_COST_LIMIT_PATTERNS` being an 8-row ordered list** funnelling many spellings into one
  `cost_limit` field. That is exactly N-inputs-→-1-output. Keep. (F5 asks to unify the
  *implementations*, not to flatten this list.)
* **Different connectors (`さらに`, `し`, `を得て`, `その後`) all producing `sequential`.**
  Many inputs, one output. Keep.
* **Shared helpers** — `as_int`, `deduped_groups`, `extract_all_groups`,
  `detect_require_all_hearts`, `_fill_defaults`, `make_conditional_on_optional`. Working as
  intended.
* **Redundant-but-consistent fields**, e.g. `PL!-bp6-019-L` carrying `cost_limit: 17`,
  `count: 17` and `original_count: 17`. Wasteful, not wrong, and not a divergence risk
  because the values agree. Leave unless something starts reading only one of them.

---

## Suggested order

| # | Item | Why first |
|---|---|---|
| 1 | **F4** — automatic central enrichment | Root cause of A1 and A2. Everything else is cheaper once clause extraction is one pass. |
| 2 | **F1** — one `exclude_self` answer | Worst measured divergence (5/7), and the concept that just caused D2. |
| 3 | **F3** — one reveal-and-take builder | Directly unblocks F4 for the look family, which is where A1 lived. |
| 4 | **F5** — one cost extractor | Latent, but two of four miss a whole operator class. |
| 5 | **F6** — one count extractor | Smallest, and `_count_from_cards` is outright wrong for 人. |
| 6 | **F2** — one `sequential` builder | Largest and riskiest; needs a golden diff run and touches 37 sites. |

Before starting F2 or the F5 wrapper migration: `git stash` the parser, regenerate, and diff
`cards/abilities.json` to establish how much of the current output is order-dependent. That
number should be the acceptance bar for the whole sweep — any consolidation that changes a
node the corpus did not already change by accident should be treated as a finding in itself.
