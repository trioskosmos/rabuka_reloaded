# Parser defects — printed clause vs. `abilities.json`

Found by auditing `cards/cards.json` (printed text, ground truth) against
`cards/abilities.json` (parser output), effect family by effect family. Companion to
`docs/ABILITY_AUDIT_2026-09-30.md` and `docs/EDGE_CASE_FINDINGS.md`.

**Nothing here is fixed yet.** This is the standing list. One finding (D1) I verified myself
end-to-end; the rest are as reported and need the same confirmation before any fix.

Impact is graded: **S1** the engine will do the *opposite* of the printed text · **S2** a clause
is dropped or broadened, so the effect fires in cases it should not or misses cases it should
· **S3** lossy but not currently wrong.

---

## D1 — S1, VERIFIED: a restriction's target is inverted

**Card** `PL!S-bp7-003-P` / `-P＋` / `-R＋` / `-SEC` 松浦果南 (ab#1)

**Printed** (bullet 1 of 「以下から1つを選ぶ」):

> ライブ終了時まで、**自分のステージにいる**元々持つ{{icon_blade}}の数が3つ以下の『Aqours』の
> メンバーは、**相手の効果によっては**ウェイトしない。

**Parsed** (`unique_abilities` node for this clause):

```json
{ "text": "…自分のステージにいる…『Aqours』のメンバーは、相手の効果によってはウェイトしない。",
  "source": "stage", "count": 3, "target": "opponent",
  "group_names": ["Aqours"], "blade_limit": 3, "blade_limit_operator": "<=",
  "action": "restriction", "restriction_type": "cannot_wait_by_effect" }
```

`target: "opponent"` is the **exact inverse** of the printed text. The immunity should apply to
**my own** Aqours members; the parse grants it to the **opponent's**. `相手の効果によっては`
modifies the *verb* — "does not wait **because of** the opponent's effects" — and is not a
target at all.

I confirmed this by reading the card text and the emitted node directly (see the two blocks
above); it is not an inference.

**Correction to the original report:** it also claimed `count: 3` was a mis-pickup from
「ブレードが3つ以下」. That is wrong — `blade_limit: 3, blade_limit_operator: "<="` is present
and agrees with it, so the clause is represented twice consistently. The **only** defect on
this node is the inverted target.

**Suspected cause.** Two rules disagree on 「相手の」 vs 「自分」: `_set_action_023`
(`parser.py:1986-2014`) tests 相手 first, while `_set_restriction_fields`
(`parser.py:10589-10594`) orders 自分 first. One of them is wrong for this input.

**Next step:** confirm the engine actually honours `target` for `cannot_wait_by_effect` — if it
does, this is a live gameplay bug, not just a bad parse — then fix the rule ordering and add a
Rust test that stages an Aqours member on each side and asserts who is protected.

---

## S1 — reported, needs confirmation

### D2 — 「すべて」 becomes a hard `count: 3`
**Card** `LL-bp4-001-R＋` 絢瀬絵里&朝香果林&葉月恋 (ab#0)
**Printed:** 「…相手のステージにいる、これにより公開したカードのコスト以下で、かつ元々持つブレードが
3つ以下のメンバーを**すべて**ウェイトにする。」
**Parsed:** `followup_action` carries `state_change: "wait"`, `count: 3`,
`blade_limit: 3, blade_limit_operator: "<="` — and **no `all: true`**.
The `3` is not in the printed clause as a *count*; it is the blade bound, already correctly
captured. The identical wording parses correctly elsewhere (`PL!HS-pb1-008` → `"all": true`,
no `count`), so the quantifier is lost on the 「その後」 follow-up path specifically.
Effect: taps 3 members instead of all of them.

### D3 — an OR of two payment options parsed as a mandatory two-step sequence — **FIXED 2026-10-02**
**Card** `PL!-pb2-012-P＋` / `-R` 南ことり (ab#1)
**Printed:** 「このメンバーをウェイトにする：この能力を起動するための**追加コストとして、手札を2枚控え室に置くか**、**『Printemps』のメンバーを2人ウェイトにする**。自分の控え室から『Printemps』のライブカードを1枚手札に加える。」
**Was parsed as:** a `sequential` whose legs are discard-2 **then** wait-2 — so the player
had to do *both*. It had also fallen out of the `cost` object entirely, so it was
modelled as an effect step *after* the card's own cost rather than as a cost.
**Now:** `cost` is a `sequential_cost` of two steps — the declared
「このメンバーをウェイトにする」 self-wait, then a `choice_condition` with the two
printed payments as options. The effect is the recover step alone.

Two changes made this possible:
* `parser.py::split_cost_effect` now pulls a 「追加コストとして、」 clause out of the
  effect side and into the cost, ending the clause at the first depth-0 「。」. Without
  this the 「：」 convention sent the whole sentence to `parse_effect`.
* `cost_parser.py::_cost_additional_clause` builds the two-step cost. It runs ahead of
  `_cost_verb_choice`, which could not have repaired it: `_cost_verb_choice` reads the
  whole cost text, and at that point the text still begins with the declared
  「このメンバーをウェイトにする」, so its 「か」 is never a joiner.

**This exposed a matching engine gap, also fixed.** `ChoiceRoute::ChoiceCost` was written
by the cost handler but read nowhere, so answering the cost-option prompt resolved the
selection against the **cost root** — the self-wait step — and failed with "No matching
members on stage to change state". `handle_choice_condition` now locates the actual
`ChoiceCondition` step (`pending_choice_cost_options`) instead of assuming the root is
the choice, which is what nested-in-a-`sequential` costs require.

Tests: `engine/tests/.../cost/additional_cost_choice/pb2_kidou_additional_cost_discard_two_or_wait_two_printemps_pl_pb2_012_p_plus_test.rs`
(5 tests) and `cards/ability_extraction/tests/test_parser_defects.py::test_d3_additional_cost_choice_is_a_cost_not_effect_steps`.

Still open on this card: the recover step needs a `『Printemps』のライブカード`, and the
card pool has none — every `-SRL` live card is μ's or Liella!, so that step cannot be
exercised and the tests assert nothing about it.


### D4 — an effect inherits its own condition's area and character names
**Card** `PL!-pb2-042-L` PSYCHIC FIRE
**Printed effect:** 「**相手のステージにいる**元々持つハートの数が**4つ以下のメンバー1人**をウェイトに
する。」 — no position, no character names.
**Parsed:** that node also carries `position: "center"` and
`characters: ["矢澤にこ","西木野真姫","絢瀬絵里"]`, both inherited from the *preceding
condition* (「自分の**センターエリア**にコスト11以上の『BiBi』のメンバーがいる場合」).
Effect: can only tap a centre-area member, and only those three characters.

---

## S2 — reported

### D5 — two different card-property filters OR-ed into one; the live branch loses 必要ハート
**Card** `PL!S-pb1-013-N` 黒澤ダイヤ
**Printed:** 「ハートにheart04を2個以上持つ**メンバーカードか**、**必要ハートにheart04を2以上含む
ライブカード**を1枚…」
**Parsed:** one node with `heart_colors: ["heart04"]`, `heart_color_count: 2`,
`or_card_types: ["live_card","member_card"]`, and a **dangling
`cost_limit_operator: ">="` with no `cost_limit`**.
The two branches test *different properties* (owned vs required hearts), so a live with 2 owned
heart04 but 0 required passes. The `か` collapses to a bare type list, losing the per-branch
filters. This is the same card whose 個/つ threshold gap was A1 in the ability audit — the
*count* is now right, the *per-branch property* is still not.

### D6 — `代わりに` alternative condition loses the filter defining its card set
**Card** `PL!-pb1-004-P＋` / `-R` 園田海未
**Printed:** 「自分の成功ライブカード置き場に{{icon_score}}を持つ『μ's』のカードが1枚ある場合、…+1を
得る。**2枚以上ある場合、代わりに**+2」
**Parsed:** the `alternative_condition` copies `location`, `group_names`, `position`, `target`
but **not `card_property: has_score_icon`**. Two μ's cards *without* a score icon wrongly
grant +2.
Cause: `_ALT_INHERITED_KEYS` (`parser.py:7836-7843`) omits `card_property` (and `negation`,
`ability_filter`, `cost_limit`).

### D7 — OR branches 2 and 3 lose zone, owner and card type
**Card** `PL!-pb2-037-L` Shangri-La Shower
**Printed:** 「エールにより公開された自分のメンバーカードが**すべて『Printemps』か**、**すべて『lily
white』か**、**すべて『BiBi』のいずれかの場合**」
**Parsed:** branch 1 has `location: revealed_cards`, `target: self`, `card_type: member_card`;
branches 2 and 3 have none of those, so 「すべて『BiBi』」 becomes "a BiBi exists anywhere"
rather than "all revealed member cards are BiBi".

### D8 / D9 — activation-cost reductions demoted (and once duplicated) as post-effect steps
- `PL!-pb1-007-P＋` / `-R` 東條希: 「この能力を起動するためのコストは、自分の成功ライブカード置き場にある
  カード1枚につき、控え室に置く手札の数が1枚減る」 becomes step 2 of the **effect**, so it
  fires after the ability resolves rather than while its cost is paid. It also inherits
  `group_names` and `exclude_self` from the effect's gate.
- `PL!-bp5-004-AR` / `-P` / `-R＋` / `-SEC` 園田海未: the reduction is emitted **twice** —
  correctly on `cost` (`cost_reduction_per_group: 1`) **and** again as an effect step. The
  action cost can be paid at full 4 E by leaving step 2 until last.

---

## S3 — lossy but not currently wrong

### D10 — a value list narrowed by a scalar
`PL!SP-bp1-003-*` 嵐千砂都: 「コストの合計が、**10、20、30、40、50のいずれか**の場合」 parses to
`values: [10,20,30,40,50]` **and** `count: 10, cost_total: 10`. The list is correct; the
scalar narrows it. An engine keying on `cost_total` silently drops 20–50.

### D11 — an OR of two areas kept only as a joined string
`PL!SP-bp4-003-P` / `-R`: 「（この能力は**左サイドエリアか右サイドエリア**に登場した場合のみ発動する。）」
→ `activation_position: "left_side,right_side"` (a string, not a list) and a nested
`trigger_event.position` of only `"left_side"`. `is_zone_disjunction` (`parser.py:428`)
exists but was not consulted.

### D12 — a duration propagated onto a step that never printed it
`PL!-bp5-021-L` / `-SRL`: the third tier 「…さらにこのカードのスコアを＋１する」 prints no
「ライブ終了時まで」 — only the second tier does — yet the step carries `duration: "live_end"`.
The structurally identical sibling `PL!-bp4-021` emits no duration, so this is an artifact of
sibling order in `_propagate_effect_duration` (`parser.py:1343-1367`).

---

## Checked and found clean (so the negative is on record)

`set_blade_count` / blade-heart filters across `PL!SP-bp4-025`, `PL!N-bp7-004`,
`PL!SP-pb2-009`, `PL!HS-bp6-013` — value and direction preserved throughout;
`cost_from_revealed` conjunction on `LL-bp4-001`; `set_cost` on play (`LL-bp7-001`);
`cost_total` on appearance (`PL!S-bp2-006`); per-unit heart-colour negation
(`PL!-bp5-023` → `exclude_heart_colors`); the 3-way OR score gate (`LL-bp5-001`); the
「ないかぎり」 negation gates (`PL!-pb2-023`, `PL!HS-pb1-015`); and the 「いずれも」 value list
in D10.

---

## The pattern across D5–D9

Six of the nine S1/S2 findings are the same shape: **context that lives on one node is not
propagated to a sibling that needs it.** `_ALT_INHERITED_KEYS`, `_try_or`'s per-fragment
`parse_condition`, `_enrich_characters` applied to a node's inherited full text, the
follow-up builder not calling the `all`-detector. Each is a *different* missing key in a
*different* function, which is why the sweep's earlier F1/F4 lesson applies directly: these
are candidates for one shared "inherit the context this node's text did not print" pass,
rather than nine separate patches. Fixing them individually is the same mistake as the
nine `exclude_self` sites.
