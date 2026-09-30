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

# Execution log — 2026-09-30 (later session)

Same day, second pass. The rule applied throughout: **the ~4000 Rust tests are the guard, and
the work is to make them sensitive.** A green suite that cannot detect a defect is the defect.

## Done

### F1 — one `exclude_self` answer (was 9 sites, 5 of 7 sample clauses disagreed)

`detect_exclude_self` rewritten in `parser_fields.py` to hold the **union** of every form any
call site recognised, minus two that name something other than the activator: `『X』以外`
(a group, handled by `apply_group_exclusions`) and `「X」以外` (a character, handled by
`_split_include_exclude_chars`). The nine per-site widenings collapsed to plain calls.

The nine-clause table now resolves correctly, including the two the old code failed in
**opposite** directions: `『μ's』以外のメンバーを1人選ぶ` used to get a bogus `exclude_self`
from the bare `"以外" in text` site, and `これを除くメンバーを1人選ぶ` used to be missed by it.

**Honest outcome: latent.** I checked the corpus — zero cards exercise the shapes where the
sites disagreed, so no gameplay test can be made to fail. This is a robustness refactor, not a
bug fix, and the 4011 tests were green before and after.

### F4 — one central enrichment walk (root cause of A1 and A2)

`_enrich_card_pool_filters(node, inherited_text)` runs once over the finished tree at both
`parse_effect` exits and applies `heart_colors` / `heart_color_count` / `card_property` /
`exclude_self` / `cost_limit` to card-pool nodes, adding only what is absent.

**The design detail that made it work.** A first attempt read only each node's own `text` and
changed nothing. Tracing showed why: on `PL!S-bp5-007-AR` the nested `select_action` has
**no `text` at all** (`text_len=0`) — the printed clause lives once on the parent
`look_and_select` node. A walk that reads each node's own text therefore skips exactly the
nodes that need help most. `inherited_text` carries the nearest ancestor's prose down.

Verified as a genuine consolidation, not a wrapper: the per-builder
`_add_heart_color_threshold` call in `_build_reveal_add_discard` (the A1 fix) was **deleted**,
regenerated, and the discriminating test still passes with an unchanged parse. One place, not
four.

## Test sensitivity added — each proven red without its clause

| Card | Gap found by reading the test | Proof |
|---|---|---|
| `PL!-pb1-001-R` 高坂穂乃果 | The live branch of 「ライブカード**か**コスト10以上のメンバーカード」 had no below-threshold test. Every member fixture cost **exactly 10** (the bound) and the only live fixture is score 1, so no fixture ever contradicted the filter. | Deleting the `コスト(\d+)以上` extraction turns exactly **one** test red and leaves the other 10 green. |
| `PL!S-pb1-015-N` 津島善子 | The live branch's `必要ハートにheart05を2以上` threshold had **no** negative test — its only live fixture sits well above the bound. | Added `PL!HS-bp2-019-L` (need heart05 = **1**). Also added: rejected looked-at cards must reach the **waitroom**, not merely be absent from hand; and declining the optional cost must leave the deck **untouched**. |
| `PL!-bp6-002-R` 絢瀬絵里 | 「能力を持たない『μ's』**か**常時能力を持つ『μ's』」 — the group governs **both** branches, but the only non-μ's fixture had *no* ability, so it was rejected by branch 1 and branch 2's group check was never exercised. | Added `PL!HS-bp1-003-R＋` (蓮ノ空, with a 常時) beside a qualifying μ's card, asserting the exact `filtered_indices` **and** its index. |
| `PL!N-sd2-005-SD2` 宮下愛 / `PL!N-bp3-002-R` 中須かすみ | Six-way 「好きなハートの色を1つ指定する」, but both tests read `boosted[0]` — *whatever* colour won — so an engine that ignored the selection and applied a fixed colour passed. かすみ's carried a comment claiming its identity was checked when it was not. | Now pinned by name, with **paired** tests selecting two different options. |
| `PL!-bp6-016-N` 東條 希 (reorder) | A deck holding **fewer than 3** cards reached no test; `live_success_look_three_deck_size_q36_test` stages 15 but drives a different card's live. | Added deck sizes 0/1/2, reversing the block so a no-op cannot pass. |
| `PL!-bp3-007-R` 東條 希 (three-way split) | The test asserted two counts and **`hand_now <= 3`**, which passes for 0, 1, 2 or 3 — so a routing bug that sent the hand card nowhere passed. Nothing located *which* card went where, which is the whole content of the text. | Rewritten to select by card identity and assert each card's destination, plus a swap test. |
| `LL-bp5-001-L` Live with a smile! | 「5種類以上」 collectively — the suite staged **5** types (qualifies) and **3** (does not), so `>= 4`, `>= 5` and `> 4` all pass. | Added exactly **4** types (`PL!S-PR-015-PR` h02+h04 with `PL!-sd1-010-SD` h01+h03, disjoint), with the union asserted as a precondition. |
| `PL!-bp5-002-R` 絢瀬絵里 | 「コスト9以上のμのメンバー」 — every negative used `PL!-sd1-010-SD`, which fails **both** halves at once (not μ's *and* cost 4), so no test distinguished the cost bound from the group clause. | Added `PL!-bp4-012-N` (μ's, cost **7**, no ability) beside a cost-9 card: the below-bound cards pass 『μの』 and fail only on cost. There is **no cost-8 μ's member** — the μ's cost histogram is 2,4,5,7,9,10,11,13,15,17,22 — so 7 is the closest reachable card below 9. |
| `PL!S-bp7-010-N` 高海千歌 | The **decline** test pushed a **one-card deck** and then asserted `deck.contains(&x)` and `deck.last() == x` — both trivially true, since the deck held exactly one card and it was `x`. Nothing about "stays at the bottom" was checked. The accept test confirmed `get(3) == x` but never that the deck kept its length, its other cards, or their order. | Decline now asserts the deck is **byte-for-byte identical** to a 7-card pre-state. Accept asserts length, the three cards above the insertion point, the shifted tail, and that `x` appears exactly once. Added a test pinning that the card that moves is specifically the **bottom** one, with a distinct top card. |
| `PL!S-bp7-008-R` 小原鞠莉 | 「好きな枚数を好きな順番で」 — 好きな枚数 includes **zero**, and the two tests only pick 1 and 2. | Added the 0 case. The real behaviour is a **third** outcome, discovered by observation after two wrong guesses: declining applies 残りを…デッキの下に置く to all three at once, in look order, with **no** ordering prompt — not a cancelled ability, and not an ordering choice. |

### An engine question this sweep turned up: `has_blade_heart()` is true for every live

Found while auditing `PL!N-bp7-018-N` 近江彼方's
「ブレードハートを持たない『虹ヶ咲』のメンバーカード」. I wanted a fixture isolating the
メンバーカード clause — a 虹ヶNash live would serve, since a live prints no ブレードハート.
No such fixture exists, and the reason is in the engine
(`engine/src/core/card.rs:4311-4317`):

```rust
pub fn has_blade_heart(&self) -> bool {
    self.blade_heart.is_some()
        || self.special_heart.as_ref().is_some_and(|sh| !sh.hearts.is_empty())
}
```

`special_heart` is where a **live card's printed heart icons** are stored, so the predicate is
`true` for every live. Two consequences:

* **Untestable today.** A live is rejected by the 持たない clause before メンバーカード is
  consulted, so no card in the corpus can separate those two clauses. Confirmed by trying both
  candidates: my precondition assertion caught `PL!N-pb1-040-L` and `PL!N-sd1-025-SD` both
  "having" a blade heart. The new test pins the conflation as a fact rather than pretending
  to isolate the clause.
* **Potentially observable.** 49 cards print 「ブレードハートを持たない」. 45 of them pair it
  with メンバーカード explicitly, so a live is excluded either way and the conflation is
  invisible. The 4 that do **not** name a member are where it can matter, and two of those are
  live-facing:
  * `PL!S-bp3-019-L` MIRACLE WAVE — counts ブレードハートを持たないカード over **everything
    yelled this turn** (members *and* lives) and sets its own score to 4 when that count is 0.
    Yell only a live and the two readings diverge: the live has no ブレードハート, so a strict
    reading counts it as blade-less and the score stays 7; the current `has_blade_heart()`
    counts it as having one, the count is 0, and the score becomes 4. **Its only test
    exercises the other OR branch** (excess hearts), so this path is uncovered either way.
  * `PL!HS-bp6-027-L` 月夜見海月 — 「ブレードハートを持たない『蓮ノ空』のカード」, no member
    scope, and 蓮ノ空 has lives.

  **Not changed.** Whether a live's heart icons should read as a ブレードハート is a rules
  question, not a refactor, and both readings are defensible on the printed text. It needs a
  decision and then a test on each of those two cards. Recorded, not silently resolved.

### A missing OR branch, found in the same place

`PL!S-bp3-019-L`'s condition is an **OR** — 「ブレードハートを持たないカードが0枚の場合**か**、
または自分が余剰ハートを2つ以上持っている場合」. The single existing test
(`live_success_no_blade_heart_or_two_excess_hearts_scores_four_test.rs`) drives only the
second branch, and its two cheered cards are `PL!S-sd1-001-SD` 高海千歌, who prints **no**
blade heart — so the first branch is *already false* in that test and the score of 4 comes
entirely from excess hearts. Nothing tests the first branch, and nothing tests the OR being
false.

### Twice I guessed the interaction instead of tracing it

Both cost a red run and both were resolved by observing, not by reasoning:

* **`PL!-bp3-007-R`** I assumed the engine offers a destination prompt, so I asserted one
  appears. It never does. Traced output: two `SelectCard` prompts over `looked_at` with 3 then
  2 options, no `SelectTarget` at all. The previous file's `Some("SelectTarget") =>
  game.select_option(1)` branch was therefore **dead code that had never run**.
* **`PL!S-bp7-008-R`** I first expected declining to place the three at the bottom *with an
  ordering prompt*, then expected it to cancel the ability and restore the deck. Only the
  third attempt — the actual behaviour, no prompt at all — was right.

The lesson generalises: on an unfamiliar ability, dump the prompt sequence first. A wrong
guess about the shape produces a test that looks rigorous and asserts nothing, which is worse
than the `<= 3` it replaced.

### A third recurring class: an inequality standing in for an exact value

Three separate files asserted a bound with `<=` where the arithmetic fixes the answer:

| file | assertion | why it was vacuous |
|---|---|---|
| `paid_look_three_splits_hand_top_and_waitroom_test.rs` | `hand_now <= 3` | paying 2 of 2 and taking 1 back leaves exactly 1; 0, 1 and 2 all pass |
| `sequential_cost_look_five_take_cost9_member_test.rs` | `hand.len() == 0` with the note "'<= 1' passed both" | already hardened by an earlier pass |
| `live_success_look_three_deck_size_q36_test.rs` | `deck_after <= deck_before`, message "should draw cards" | a *different card's* live is driven; the reorder ability is never exercised, and `<=` cannot detect the deck growing when it should not |

The third is the worst of the three: the filename says `look_three` and the assertion is about
START:DASH!!'s draw, so the file named for one ability tests another.

### The dominant gap class, now quantified

Nine of the eleven sensitivities added this session were the **same shape**: a printed bound with
its two ends staged and the boundary between them never staged. `N-1`-or-below is what
separates `>= N` from `>= N-1`, and it is exactly what a test suite skips, because the
interesting positive case is the one that satisfies the bound and the interesting negative is
the one that obviously fails everything at once.

The second recurring class is a **negative fixture failing several clauses at once**, so
rejection proves nothing about any single clause — the 絢瀬絵里 cost case above, and the same
pattern in `look_four_take_member_with_double_heart04_test.rs` (§B1 of the ability audit).

## Two of my own wrong calls, corrected

* I listed `PL!S-bp3-008-R` as "unhandled". It is handled and had 6 tests; only the exact-6
  boundary was missing (now added).
* I claimed `PL!-bp1-001-R` had a live-branch cost bug. The parse *looks* wrong —
  `(live OR member) AND cost>=10` — but the engine applies the clause per branch, so it is
  correct. The real gap was only that nothing tested it.

## Notable engine reading

`check_ability_filter` (`ability/util.rs:1582-1597`) gates **only** the ability branches; the
group is a separate `CardFilter` field, so the combination is group AND (branch₁ OR branch₂) —
which is why 絢瀬絵里's new test verifies a real structural property rather than an accident.
Both the `or_ability_filters` check and the single `ability_filter` check are wrapped in
`if let Some(card) = db.get_card(id)`, so a card id absent from the database **fails open**.
Not reachable in real play; noted, not changed.

## Still open

* **F2** — 37 `sequential` producers. Untouched; highest regression risk, needs a golden diff.
* **F3** — four output shapes for the reveal-and-take mechanic. Partly addressed by F4 (filter
  extraction is now central); the four shapes themselves remain.
* **F5** — four cost-comparison implementations, two operator vocabularies. Latent: the forms
  they disagree on appear on **0** cards, and the one real card using a divergent form parses
  correctly.
* **F6** — `_count_from_cards` returns its default `1` for 人-counts. Latent: all five cards in
  that family print `1枚`, where both extractors agree.
* `group_names` is set at **25** `extract_all_groups` call sites; dropping one changed nothing,
  which is how the per-branch group test in 絢瀬絵里's file proved inconclusive. Worth folding
  into the same one-place treatment as F1.

## State

**engine 4026 passed / 0 failed** · **parser 124 passed / 0 failed**.
`cards/test_inventory.py` regenerated. The 13 `--check` ratchet failures remain in files this
work never touched (concurrent bot/C-port work).

## New open item from this pass

**`Card::has_blade_heart()` returns `true` for every live card** (`core/card.rs:4311-4317`),
because a live's printed heart icons live in `special_heart`. It makes 「ブレードハートを
持たない」 untestable in isolation (a live is rejected before メンバーカード is consulted) and
potentially wrong on the 4 of 49 such clauses that do not name a member — notably
`PL!S-bp3-019-L` MIRACLE WAVE, whose only test drives the *other* OR branch, and
`PL!HS-bp6-027-L` 月夜見海月. Both readings of the printed text are defensible, so this needs a
**rules decision**, then a test on each card. Left unchanged deliberately.

---

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
