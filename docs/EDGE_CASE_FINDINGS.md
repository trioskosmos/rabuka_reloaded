# Edge-case hunt — findings

Companion to `docs/ABILITY_AUDIT_2026-09-30.md` (ability defects found and fixed) and
`docs/PARSER_CONSOLIDATION_SWEEP.md` (parser fragmentation + F1/F4 work). This file is the
standing list of **abilities whose tests cannot detect a defect they claim to cover**.

Every entry is: card → verbatim printed clause → what the test asserts → why that is blind →
the concrete fixture that would close it. All card data was read from `cards/cards.json`,
which is the ground truth; test-file comments were never trusted without checking.

Grade: **FIXED** = closed in this pass. **OPEN** = not yet closed.

---

## The five recurring blindness classes

Discovered by auditing ~40 test files entry by entry. Once named, they account for almost
every finding below.

1. **Boundary never staged.** A bound `>= N` tested at `N` and at something clearly lower,
   never at `N-1`. Any bound from `N-1` upward passes.
2. **Multi-axis negative.** The rejecting fixture fails several printed clauses at once
   (wrong group *and* wrong cost *and* wrong type), so its rejection proves nothing about any
   individual clause.
3. **Inequality where the value is exact.** `>= 1` / `> 0` / `assert_ne!(x, 4)` standing where
   the printed text fixes a number. A double-granting or partial-granting defect passes.
4. **Only the positive asserted.** A position- or condition-gated effect where each case
   asserts the value it grants but never that the *other* cases grant nothing. An
   implementation that grants everything everywhere passes.
5. **Absence satisfied by a stalled prompt.** A decline path asserted only as "nothing
   appeared", which a prompt that was never answered also produces.

---

## FIXED in this pass

### `PL!HS-bp2-007-R＋` 百生吟子 — 13 inequalities where the print says exactly one
「同じ名前を持つメンバー1人は、ライブ終了時まで、heart04ブレードを得る」 — one blade. The heart
half was already `assert_eq!(.., 1)`; the blade half was `assert!(.. >= 1)` at **10** sites
plus 4 compound forms. A defect granting 2, 3 or 5 blades was invisible at every one of them.
All converted to `assert_eq!(.., 1)`; **21/21 pass**, so the engine grants exactly one.

### `PL!SP-bp5-011-R` 鬼塚冬毬 — three position-gated 常時, only the positive asserted
「左サイドheart02×3 / センターheart03×3 / 右サイドheart05×3」. Each case asserted its own colour
is +3 and nothing about the other two, so **an implementation granting all three in every area
passed all three assertions** — the position gate, the whole content of the card, was
unproven. Now each case pins the other two at 0, plus a new test asserting the three colours
total 3 in every area and never 9.

### `PL!SP-sd2-004-SD2` / `PL!SP-pb2-035-N` / `PL!SP-pb2-041-N` — position gates under-asserted
- `PL!SP-sd2-004-SD2` used `assert_ne!(blade, 4)` off-centre, which also accepts +1/+2/+3. Now
  `== 0` on **both** non-centre sides.
- `PL!SP-pb2-035-N` (left → ×2) and `PL!SP-pb2-041-N` (right → ×2) had **no negative case at
  all** — a position filter that was simply absent would pass. Now each asserts `== 0` at the
  other two positions.

### `PL!S-bp3-019-L` MIRACLE WAVE — the OR's false branch
「ブレードハートを持たないカードが0枚の場合**か**、または自分が余剰ハートを2つ以上持っている
場合、このカードのスコアは4になる」. The single existing test stages two blade-less 高海千歌 for
6/4/4 — an excess of 2 — so branch **B** fires and the score of 4 says nothing about branch A.
Neither A nor "both false" was covered.

Closed with two real-card pairs that sum to **exactly** the 4/4/4 requirement, so excess is 0
and only the blade clause can move the score:

| | pair | (h02,h04,h05) | blade | result |
|---|---|---|---|---|
| A true | `PL!S-bp3-001-P` + `PL!S-bp2-011-N` | 3,2,2 + 1,2,2 | both yes | score **4** |
| both false | `PL!S-sd1-001-SD` + `PL!S-bp3-003-P` | 3,2,2 + 1,2,2 | both no | score **7** (printed) |

Both pass — the engine is correct; the branch simply had no coverage.

**A trap worth recording:** my first attempt assigned `player1.stage_hearts` directly to force
an exact match. It is **recomputed from the members' base hearts**, so the override was
silently ignored, the excess stayed 2, and the test failed for the wrong reason. The fix was
real cards, not an override.

### `PL!-bp5-111-R` 綺羅ツバサ, `PL!S-bp2-001-R` 高海千歌, `PL!N-bp7-024-N` 鐘嵐珠, `PL!S-bp6-010-N`, `PL!SP-sd2-008-SD2`, `PL!-bp5-008-R`, `PL!S-bp7-014-N`, `PL!S-bp1-005-PR`, `PL!-bp5-011-N`, `PL!SP-bp7-020-N`, `PL!SP-pb2-030-N`, `PL!S-bp7-024-L`, `PL!S-bp6-002-SEC` — closed earlier in this pass
Detail in the "Test sensitivity added" table of `PARSER_CONSOLIDATION_SWEEP.md` and in
`ABILITY_AUDIT_2026-09-30.md` §C3.

---

## OPEN — `effects/gain/**`

**G1. `PL!SP-bp4-003-R` 嵐千砂都 — 常時 センター ブレード×2, asserted `>= 2`.**
Exactly 2 blades; 3/4/5 pass. The 登場 clause (左サイド/右サイド → 2枚引) is never touched and the
常時 is never evaluated off-centre, so a dropped position gate is invisible. Fix: `== 2` at
centre, `== 0` at left/right.

**G2. `PL!-bp5-111-R` 綺羅ツバサ — 『A-RISE』 filter never isolated.**
「このメンバー以外の『A-RISE』のメンバー**1人につき**heart05」. Only A-RISE fixtures are staged
(0/1/2 cases), so an implementation counting *any* other member satisfies all of them. Both the
unit filter and the self-exclusion are unproven. Fix: stage `PL!-sd1-010-SD` (Printemps) beside
her → expect 0.

**G3. `PL!S-bp2-001-R` 高海千歌 — two-clause AND tested only with both clauses failing.**
「自分の成功ライブカード置き場のカードが**0枚で**、**かつ**相手の成功ライブカード置き場にカードが
1枚以上ある場合」. The negative stages own=1, opponent=0 — both clauses fail at once. An
implementation that drops the "own must be 0" clause passes everything. Fix: put a live in
**both** success zones → own fails, opponent passes → expect 0 blades.

**G4. `PL!S-bp6-002-SEC` 桜内梨子 — two blind spots, both need care.**
「『Aqours』のみ**で**、**かつ**それらの必要ハートに含まれるheart02とheart04とheart05の**合計**が
12以上」. (i) The non-Aqours negative also fails the sum (3 < 12), proving nothing about the
group. (ii) Every positive fixture's `need_heart` is *entirely* 02/04/05, so "sum only these
three colours" is indistinguishable from "sum all need_hearts". Fixes: a 虹ヶ咲 live with 7 in
02/05 beside two Aqours 02/04/05 cards reaching 13; and 3× an Aqours live whose non-02/04/05
hearts push the all-heart total over 12 while the 02/04/05 total stays at 6.

**G5. `PL!N-bp7-024-N` 鐘嵐珠 — 「3人いる場合」 and 「自分のステージ」 both unproven, positive uses `> 0`.**
The positive is `> 0` (a double grant passes) and the negative leaves the opponent's stage
**empty**, so an implementation counting *both* players' R3BIRTH members passes both. Fix: 2
R3BIRTH on p1 + 1 on p2 → expect 0; and 4 on p1 → expect 0 (the print says 3人, exact).

**G6. `PL!S-bp6-010-N` 高海千歌 — heart02 sum ≥ 4 staged at 4 and 0 only.**
Fix: `PL!S-PR-023-PR` (need heart02 = 2) → expect 0.

**G7. `PL!-bp5-008-R` 小泉花陽 — スコアの合計 ≥ 6 staged at 0/3/6/9, never 5.**
Fix: `PL!-sd1-021-SD` (3) + `PL!-sd1-019-SD` (1) + `PL!SP-bp1-023-L` (1) = 5 → expect 0.

**G8. `PL!SP-sd2-008-SD2` 若菜四季 — コスト13以上 never staged at the bound, positive uses `> 0`.**
Staged at 15 and 4, so `>= 14` passes. **No cost-12 card exists in the corpus**, so the
tightest below-boundary fixture is cost 11, leaving 11-vs-13 ambiguous. Fix: `PL!-bp5-008-R`
(cost 13) → 1 heart03 exactly; plus a cost-11 fixture → 0.

**G9. `PL!S-bp7-014-N` / `PL!S-pb1-005-PR` 渡辺曜 — 「相手より多い」 never staged at a margin of 1.**
Margins tested are ≥ 2 and 0. A defect needing a 2-card lead passes. Fix: self 1 / opp 2 → exactly
1 heart02; 6/7 → exactly 3 blades.

**G10. `PL!-bp5-011-N` 絢瀬絵里 — 「成功置き場1枚につき」 untested at 1, and only the middle option chosen.**
A flat `+2` passes. Fix: 1 success card → `heart05 == 1`; 0 → 0; `select_option(0)` → heart04;
`select_option(2)` → heart06.

**G11. `PL!SP-bp7-020-N` / `PL!SP-bp4-009-R` 鬼塚夏美 — the tie is never staged.**
「自分のエネルギーが相手より**多い**かぎり」 and 「合計が相手より**低い**かぎり」. Equal totals are
never staged, so `>=` / `<=` pass. Fix: 2 vs 2 → 0 blades; opponent stage total equal → 0.

**G12. `PL!N-bp7-012-R` 鐘嵐珠 — test file mis-describes the cost, and never pays or declines.**
The file is named `paid_energy_color_choice_heart01_test` and comments an 「{{E}} gate」, but
the print's cost is **waiting a 『虹ヶ咲』 member**, not energy. It never asserts that a member
actually became waited, and never takes the other branch. Fix: assert the chosen member gains
`wait`; add a decline case → 0 hearts.

**G13. `PL!N-bp1-003-R＋` 桜坂しずく — comments say "discard cost", the print is 「{{E}}支払ってもよい」.**
The test takes the **skip** branch and still asserts the grant, and never pays. Fix: pay →
heart04 == 1 **and** the energy zone shrinks by 1; decline → 0.

**G14. `PL!SP-bp5-016-N` 葉月恋 — a negative advertised but never asserted.**
`ten_energy_constant_heart06_test.rs` ends with `// Negative: drop below threshold` and no
assertion at all. The sibling boundary file covers 10/5, so coverage exists — but the dead
negative should be completed or deleted.

**G15. `PL!S-bp7-024-L` ときめき分類学 — the 『Aqours』メンバー**1人** cap is never exercised.**
Exactly one Aqours member is ever staged, so recolouring *every* Aqours member passes. Fix:
stage a second Aqours member and assert the unselected one is untouched.

**G16. `PL!SP-pb2-030-N` 若菜四季 — the middle option of a 3-way か is never selected.**
Options 0 and 2 are exercised, 1 never, so a dropped or duplicated middle entry is invisible.

---

## OPEN — `effects/compound`, `effects/choice`, `effects/look_select`, `effects/cost`

**C1. `PL!HS-bp6-029-L` Proof — 「コストが合計20以上」 never staged at 20 or 19.**
Tested at 9, 22, 28, 30 — so `>=18` through `>=21` and `>20` all pass. (`>=30` *is* pinned at
30/28.) Fix: 9+9+2 = exactly 20 → look fires; 13+4+2 = 19 → does not.

**C2. `PL!HS-bp6-029-L` — a group filter the print does not have, and a test that asserts only the other half.**
The comment claims the look is 『蓮ノ空』-filtered; the print has no such clause. Both
`issue12_…_20plus` and `…_30plus` assert **only** the need-heart modifier, never the look
itself, so the 20以上→look-2/take-1 behaviour the names promise is unasserted.

**C3. `PL!S-bp2-005-R＋` 渡辺曜 — 「3枚まで」 indistinguishable from unlimited.**
Every "max" test stages exactly three qualifying cards, so the cap never excludes a 4th. And
all six tests **pay** the optional cost; **none declines**.

**C4. `PL!SP-bp7-003-R＋` 嵐千砂都 — 「コストが10か20」 probed only at 4/10/20, and the two-card premise is false.**
`game.id(COST20)` is called twice rather than `new_id`, so one physical instance is pushed
twice. `filtered_indices` is **never asserted**, and `select_indices(&[0])` is valid under *any*
filter — so the positive test passes against a completely unfiltered hand prompt. Fix:
`new_id` for the second copy, assert `filtered_indices` explicitly, and add cost-9
(`PL!HS-sd1-001-SD`) and cost-11 (`PL!HS-sd1-002-SD`) rejects.

**C5. `PL!-pb2-012-P＋` 南ことり — the optional WAIT is never executed.**
Six tests read only `get_cost_modifier`. The card is never played, so nothing is ever waited
and `してもよい` is never declined. A model that pays −2 while waiting nobody is fully green.

**C6. `PL!S-pb1-002-R` 桜内梨子 — dead `if let` assertions and no score assertion at all.**
`if let Some(e) = ability_queue.current_entry() { assert… }` runs *after* the choice is
answered; if the entry has been popped the assertion silently vanishes. And despite the header
quoting 「ライブの合計スコアを＋1する」, the file contains **no score assertion** — the score is
only checked in the sibling `_q130_edges_test.rs`. Fix: unconditional `expect`, plus
`p1_constant_total_score_bonus == 1` on decline and `0` on accept.

**C7. `PL!SP-pb2-048-L` ディストーション — heart02 = 8, one below the gate, never staged.**
Tests produce 6, 7, 7, 9 — so `>= 8` passes the whole file. Fix: two distinct CatChu! names
(`PL!SP-bp1-012-N` + `PL!SP-bp1-015-N`) → need heart02 == 8 **and** no score.

**C8. `PL!S-bp6-005-R` 渡辺曜 — a test named for a positive it never asserts, with a multi-axis negative.**
`you_q124_blade_heart_excluded_base_heart_included` stages a qualifying card but asserts only
that the blade-heart card is absent. The negative (`PL!SP-sd1-001-SD`) is missing heart04 **and**
heart05, so it cannot detect a bug folding `b_heartXX` into the colour filter. Also
`you_q124_two_plays_both_reject_blade_hearts` is named "two plays" and performs one. Fix:
`PL!S-sd1-003-SD` has heart02:1, heart04:2, heart05:1 — two colours at the exact minimum of 1,
so it also catches a `>=2` mis-parse of すべて持つ.

**C9. `PL!N-bp1-006-R＋`-class drain loops answer every prompt with index 0.**
`while has_pending_choice() { SelectAutoAbility => skip, _ => select_indices(&[0]) }` in
several files. A mistyped prompt is silently "accepted" and an optional prompt is answered as
if mandatory. Prefer explicit per-type arms with a fallback that fails loudly.

**C10. `decline_pending_choices_with_limit(&mut g, 30)` sends `select_indices(&[])` to EVERY prompt**
including non-skippable ones. Safe only where the final assertion is a hand-presence check that
a parked prompt could not fake — which is a fragile thing to rely on.

---

## Verified clean (recorded so the negative is explicit)

`PL!HS-bp6-008-R` 桂城泉 (スコア4以下 at 4/5, スコア2以下 at 2/3 — both boundaries genuinely at
the bound, `<=` correct); `PL!S-pb1-015-N` 津島善子 (both branches of the `か` at their bounds,
now including the live branch this pass added); `PL!SP-pb2-032-N` 6/8 stacking (0/5/6/7/8/10);
`PL!S-bp5-013-N` heart04 ≥ 4 (3 vs 4); `PL!S-bp7-009-R` コスト4以下 (4 vs 5); `PL!-PR-021-PR`
ちょうど7枚 (6/7/8); `PL!HS-pb1-025-L` 10枚以上 (9/10); `PL!N-sd2-026-P` 4-blade boundary with
wrong-group isolation; `PL!HS-bp2-006-R` per-unit (does stage a non-distractor);
`PL!N-bp7-007-R＋` ab#1 (6/7/8/12 exact differences); `PL!S-sd1-001-SD` /
`PL!N-sd2-005-SD2` choice-identity pinning; and `all_or_any_three_heart_member_filter_look_test`
+ `or_card_types_heart05_threshold_look_test` as the **exemplar** pattern — they read
`filtered_indices` directly, bracket every bound at its boundary, and assert that rejected
cards are *accounted for* in the waitroom rather than merely absent from hand.

---

## OPEN — `effects/score`, `effects/position`, `rules`

**S1. `PL!SP-bp5-026-L` "Let's be ONE" — 「ハートの総数が11以上」 staged at 17 and 4, never 11.**
`>= 12`, `> 11`, or summing only the first two members all pass. Fix: `PL!SP-pb2-005-R` (9)
+ `PL!SP-pb2-036-N` (2) = exactly 11 → +1; add `PL!SP-pb2-037-N` for 13 as the control.

**S2. `PL!S-pb1-020-L` トリコリコPLEASE!! — 「heart04が合計10個以上」 boundary only reached via two members.**
10 comes from two `PL!S-bp5-007-R` (heart04:5 each) and 5 from one, so **9 is never staged** —
the sharp separator. Fix: `PL!S-bp5-007-R` (5) + two `PL!S-sd1-003-SD` (heart04:2) = **9** → +0.

**S3. `PL!N-sd1-028-SD` Dream with You — 「ブレードの合計が10以上」 staged at 12 and 7, never 10.**
Same test's `assert!(initial_yell_revealed_cards.len() <= 9)` is an inequality where the Q116
text fixes 9 exactly. Fix: `PL!S-PR-014-PR` (blade 6) + `PL!HS-pb1-014-R` (blade 4) = **10** → +1.

**S4. `PL!SP-sd1-026-SD/SRL` 私のSymphony — the second printed clause has no assertion at all.**
「(エールをすべて行った後、エールで出たドロー1つにつき、カードを1枚引く。)」 is never checked for
either variant, although `9枚以上` itself is correctly bracketed at 9 and 8.

**S5. FIXED — `PL!SP-pb2-045-L` 絶対的LOVER — two inequalities where the value is exactly 1.**
「…メンバー1人につき、このカードのスコアを＋1」 with `assert!(>= 1)` and `assert!(<= 1)` as
separate arms and a comment conceding the bound "is harder to assert directly". Each defect was
caught by exactly one arm; a no-grant (0) passed the second and a double-grant (2) passed the
first. Now one `assert_eq!(score_mod, 1)`. 3/3 pass.

**S6. FIXED — `PL!S-bp7-012-N` — `assert!(total_blade >= 2)` where the print says ブレード×2.**
The fixture moves a SaintSnow member, so a per-member reading giving 4 was in reach and passed.
The file's two negatives already used `== 0`; the positive now matches. 3/3 pass.

**S7. `PL!S-bp7-022-L` 恋になりたいAQUARIUM — three independent colour clauses tested with one card holding all three.**
No fixture has two-of-three-but-not-the-third, and none splits the three colours across two
members, so an implementation requiring one card to hold all three passes. Fix: `PL!S-PR-015-PR`
(h02,h04, no h05) + `PL!S-sd1-011-SD` (h05 only) → all three across two members → +1; and the
same pair missing h02 → +0.

**S8. `PL!SP-bp4-026-L` Wish Song — the group clause AND the distinctness each never bound. FIXED.**
「名前が異なる『Liella!』のメンバーカードが**5枚以上**ある場合、このカードのスコアを＋１する。」
`revealed_distinct_liella_score_test.rs` had two tests — five distinct Liella → +1, four
distinct Liella → 0 — and both gaps were invisible:

* **every fixture in the file is a 『Liella!』 member**, so revealing five distinct Liella
  members was the only case ever staged. An implementation that dropped the **group** filter and
  counted any five distinct revealed members would pass both.
* **no duplicate names were ever staged**, so 「名前が異なる」 never bound. A filter counting all
  revealed cards while ignoring names entirely also passes both: 5 revealed = 5, 4 revealed = 4.

Added, each isolating one clause from the other:
* `wish_song_five_distinct_non_liella_members_are_not_counted` — five distinct **non**-『Liella!』
  members: same count, same distinctness, wrong group → 0. The "five different names" premise is
  asserted from card data, so a substituted printing cannot move a card into 『Liella!』 and invert
  the test.
* `wish_song_five_revealed_with_a_duplicate_name_is_only_four_distinct` — five cards but four
  distinct names → 0, with `(5, 4)` asserted as a precondition.

Both pass, so the engine applies both clauses correctly. The file already commented `REN` as
"distinct name" — distinctness was in mind but never staged.

### `PL!-bp5-333-R` 統堂英玲奈 — her 登場 is driven by NO test in the tree

Individual-entry audit. Erena has two clauses:

```
{{toujyou.png|登場}}このメンバーをウェイトにしてもよい：相手のステージにいるコスト9以下のメンバー1人をウェイトにする。
{{jyouji.png|常時}}このメンバーがウェイト状態であるかぎり、heart05を得る。
```

**All 21 tree references to her use her as a *fixture*, never as the subject.** Every one sets
her orientation **by hand** — `add_orientation_modifier(erena, "wait")` then
`recalculate_constants()` — across `waited_self_heart05_constant_test.rs`,
`erena_self_wait_constant_heart05_test.rs`, `wait_state_condition_constants_test.rs`,
`per_other_group_member_constant_heart_test.rs` and the rest. So:

* the **常時** half is genuinely covered from both sides (wait → +1 heart05, active → 0);
* the **登場** half is **never executed anywhere** — not the optional self-wait, not the chained
  opponent wait, and **not the 「コスト9以下」 bound**, which is therefore entirely unpinned.

Added `erena_debut_cost_nine_or_less_opponent_wait_test.rs` with two tests:

* `..._declining_the_optional_self_wait_does_not_wait_the_opponent` — **passes**, and is real
  coverage of the 「…てもよい：A。」 chain: declining the self-wait must cancel the mandatory
  opponent wait too, or an opponent member is waited for a choice the player refused.
* `..._offers_only_the_opponent_member_of_cost_nine_or_less` — **marked `#[ignore]`,
  UNRESOLVED.** It would pin 「コスト9以下」 with cost 9 (`PL!-bp5-017-N`) against cost 10
  (`PL!-bp5-005-AR`) — differing in cost by one and in nothing else the clause mentions. The
  debut's first prompt is an **options-less** `SelectTarget`
  (`target=pay_optional_cost:skip_optional_cost`): `select_option(0)` *declines* — which is why
  the first attempt produced no opponent prompt at all — and `select_option(1)` **panics** in
  `helpers/choices.rs:39`, because a choice with `options: None` has nothing to index. Other
  tests drive these gates, so the convention exists; I did not find it, did not guess, and did
  not fall back to asserting on the parse as though that closed it.

Fixture note, same class as the stale hunter card numbers: there are 38 ability-free members at
cost 9 and **zero** at cost 10, so the just-above-boundary fixture must carry an ability. That is
fine — an ability does not affect whether a member can be waited — and it is recorded in the
test so nobody later "fixes" the fixture and loses the bound.

## A targeted check that paid off: inequality assertions on exact printed amounts

Rather than read every test, I checked one specific hypothesis across the whole tree — **does any
test assert a printed amount with `>=` / `>` where the card fixes a number?** 51 sites matched.
Two I could fix immediately, because I had already read and verified the printed text:

* **`PL!SP-bp4-003-R` 嵐千砂都** — 「{{jyouji.png|常時}}{{center.png|センター}}**ブレード×2**」.
  `center_constant_two_blades_test.rs` asserted `>= 2`, and the test was even *named*
  `..._grants_at_least_two_blades`, so 3, 4 or 5 blades passed a card whose text says ×2. It
  also only ever staged the member in the **centre**, so the {{center.png|センター}} gate itself
  was never observed — a 常時 carrying the marker but no gate also passes "centre grants two".
  Now `== 2` at centre and `== 0` at left, right and off-stage. Passes.
* **`PL!S-bp7-014-N` 渡辺曜** — 「相手のエネルギーが自分より多いかぎり、**heart02**を得る」,
  asserted `> 0`, so a per-unit or double-counting defect on a 「かぎり」 constant was invisible.
  Now `== 1`, and the tied-energy sibling already asserted `== 0`, so the pair brackets it.

**The scan over-counts, and I should have said so up front.** Its pattern was `> 0`-shaped, so
it also matched `assert!(!... > 0)` — a *negation*, where "no blade" is naturally expressed as
an inequality. Worked example: `opponent_energy_lead_constant_blade_branches_test.rs` was on my
candidate list for `blade > 0`, but its real assertions are `assert_eq!(get_blade_modifier(you),
3)` and the `> 0` sits inside `assert!(!… > 0)` negative tests plus one `#[allow(dead_code)]`
helper. **That file is fine and I should not have listed it.**

So the 51 hits are a mix of true positives (a printed positive amount asserted with `>=`/`>`)
and false positives (negations and dead helpers). The candidate table below is therefore a
*starting set*, not a work list — each one needs the printed amount read before it is touched,
and several will turn out to be negatives or deliberate scalings.

**The remaining are NOT touched**, because I have not verified each printed amount and "exactly
N" would be a guess. Candidates, most-likely-exact first:

| file:line | assertion | likely clause |
|---|---|---|
| `center_constant_blade_bonus_test.rs:11` | `blade >= 2` | same card as above — the duplicate of the file just fixed |
| `success_score_comparison_pl_bp4_018_n_test.rs:108` | `blade >= 2` | verify first |
| `optional_energy_return_comparison_pl_sp_bp7_027_l_test.rs:71` | `score >= 1` | 「＋1」 |
| `subunit_stage_count_gated_debut_heart_test.rs:21` | `heart > 0` | 「heart01」, single |
| `combined_stage_count_constant_heart_test.rs:23,27` | `heart > 0` ×2 | two colours, verify amounts |
| `sibling_abilities_on_one_card_interact_test.rs:156,160,260` | `>= 1` ×3 | **do not touch** — two abilities stacking, so a sum above 1 may be correct |
| `mymai_tonight_test.rs:235,253,257` | `blade > 0` ×3 | **do not touch** — per-member, may scale |
| `real_card_phase_walkthrough_and_ability_suite_test.rs` | 12 mixed sites | walkthrough file, several scale |
| `debut_and_live_start_blade_grant_test.rs:39,43` | `blade >= 1` ×2 | two members, verify |
| `modifier_layer_characterization_test.rs:37` | `heart > 0` | characterization file — may be intentional |

### `PL!SP-bp5-009-R` 鬼塚夏美 Q222 — audited, no gap

Individual-entry audit of the 4-step repeat chain
(「…自分はこの手順をさらに**4回まで**繰り返してもよい」). 574 lines, 13 tests, an explicit
per-iteration decision tree in the header, and the repeat maximum **is** pinned
(`..._initial_plus_four_repeats_mills_five_lives` — five total iterations). The whole
`effects/compound/` directory has **zero** inequality assertions on gain amounts. Recorded as a
clean negative rather than left unmentioned.

**S9. `PL!S-bp7-023-L` 夜空はなんでも知ってるの？ — the 『Aqours』メンバーが2人以上 gate never bound. FIXED.**
「{{live_start.png|ライブ開始時}}自分のステージに『Aqours』のメンバーが2人以上いる場合、自分の
エネルギー1枚をエネルギーデッキに置いてもよい。そうしたとき、相手のエネルギーが自分より1枚多い
場合、このカードのスコアを＋１する。2枚以上多い場合、代わりにスコアを＋２する。」

`return_energy_deficit_score_pl_s_bp7_023_l_test.rs` always staged 高海千歌 + 松浦果南 — both
ラブライブ！サンシャイン!! — so the gate held in every case, and an implementation scoring purely
off the energy comparison would pass all three tests. Added
`pl_s_bp7_023_l_one_aqours_member_fails_the_two_member_gate`: one Aqours member plus one
ラブライブ！(μ's) member, reproducing the **+2 energy situation verbatim**. Only the gate differs,
and the expectation is **0**. Both fixtures' series are asserted from card data so a substituted
printing cannot move a card into サンシャイン and invert the test. Passes — the engine enforces
the gate correctly.

**S10. `PL!HS-bp1-022-L` AWOKE — 「10枚以上」 staged at 2 and ~9, never at 10 or 9.**
Fix: a deck of exactly 10 `PL!HS-PR-020-PR`, and of 9.

**S11. `PL!S-bp5-008-R` — 「相手の余剰ハートが2つ以上」 staged at 7, 2 and 0; never 1.**
So `> 1` mis-parsed as `>= 1` passes, and so does the reverse. Fix: a live needing exactly one
less than the stage provides.

**S12. `PL!SP-pb2-050-L` Jellyfish — the decline path DESTROYS members. REAL DEFECT.**

(The number in the earlier report, `PL!SP-bp2-050-L`, does not exist; the real card is
`PL!SP-pb2-050-L`, found from its printed clause. The hunters' `related_cards` are unreliable —
Q172 had the same problem, pointing at a nonexistent `PL!-pb3-026-L`.)

「{{live_start.png|ライブ開始時}}自分のステージに『5yncri5e!』のメンバーが2人以上いる場合、
自分のステージにいるメンバーをフォーメーションチェンジしてもよい。」

The file had three tests, and **every one that reached the formation prompt accepted it** — each
`choose_destination` call moves a member. So 「てもよい」 was never exercised, and a defect making
the change **mandatory** would have passed the whole file.

Added the decline case. It **fails, and it is a genuine defect**:

```text
stage before          : [5213, 5214, 17137]    ← two 『5yncri5e!』 members + the live
stage after declining : [17137, -1, -1]        ← both members GONE
hand=[]   waitroom=[]   deck_len=30             ← unchanged; they went nowhere
cards_moved_this_turn = [5214, 17137]          ← the engine logged them as MOVED
```

Declining an optional effect must be a **complete no-op**. Instead both members are removed from
the stage and recorded as moved while existing in no zone — dropped, not relocated.
`decline_all_remaining` asserts `allow_skip()` before declining, so the prompt genuinely is
optional; the failure is in what follows.

Kept as `#[ignore]`d `KNOWN ENGINE DEFECT` so the reproduction stays in code without reddening
the suite.

**Blast radius — measured on a second card, and it is NOT a single defect but two.**

`PL!SP-sd2-001-SD2` 澁谷かのん is one of the same five whole-stage formation changes, and its text
also reads 「…フォーメーションチェンジして**もよい**」. Running the same decline
(`formation_change_decline_blast_radius_probe`, `#[ignore]`d diagnostic) gives a different and
more serious result:

```text
kanono stage BEFORE decline = [22208, 7908, -1]
panicked: this SelectTarget prompt is not skippable: the test has to answer it,
          not decline it.
```

So:

| card | prompt skippable? | what happens |
|---|---|---|
| `PL!SP-pb2-050-L` Jellyfish | **yes** | declining **destroys** both members — gone from every zone, yet logged in `cards_moved_this_turn` |
| `PL!SP-sd1-001-SD2` 澁谷かのん | **no** | the prompt is **mandatory** at the formation-change stage, despite 「してもよい」 |

Two cards, two symptoms, one printed intent — 「してもよい」 is an **optional** formation change —
and it is honoured in neither. `decline_all_remaining` asserts `allow_skip()` before declining,
so the Kanono result is not the test forcing its way past something: her prompt genuinely does
not offer a skip.

### Localised: two distinct faults, both in `position_change` execution

**Neither is a parser fault.** Both nodes carry `optional = true`:

```
PL!SP-sd2-001-SD2  {{live_start}}カードを1枚引く。その後、…フォーメーションチェンジしてもよい。
    position_change @ .actions[1]      optional=True  multiple_targets=True  target='self'

PL!SP-pb2-050-L   {{live_start}}自分のステージに『5yncri5e!』のメンバーが2人以上いる場合、…してもよい
    position_change @ <root>           optional=True  multiple_targets=True  target='self'
```

Every `SelectTarget` in `execute_position_change` is built with
`allow_skip: effect.optional.unwrap_or(false)` (`position.rs:174, 214, 284, 407, 450, 502, 573`) —
the flag is threaded consistently. So the divergence is **where the node sits**:

**Fault A — `optional` is lost when a `position_change` is a leg of a `sequential`.**
Jellyfish's node is the **root** effect, so `effect.optional` reaches the choice and the prompt is
skippable. Kanono's node is nested at `.actions[1]`, and the prompt comes out with
`allow_skip == false` — the printed 「してもよい」 becomes mandatory. Localisation target: the
sequential leg dispatch (`compound/sequential.rs`) and how it forwards a leg to `execute_effect`.

**Fault B — skipping an optional `position_change` corrupts the stage.**
When the prompt *is* skippable and is declined, both members leave the stage, exist in no zone,
and are nonetheless recorded in `cards_moved_this_turn`. Localisation target: the resume path
for a skipped `SelectTarget` in `execute_position_change_with_destination` (`position.rs:927`).

**Not fixed, deliberately.** Fault A's fix is in the sequential dispatch, which reaches every
multi-clause ability in the corpus; Fault B's is in position-change resume, which reaches every
single-target move as well as this family. Either could regress a large surface, and I have not
got a localisation tight enough to land either with confidence. Both reproductions are kept as
`#[ignore]`d tests so the numbers stay checkable, and this section is the hand-off.

Remaining untested in the family: `PL!SP-bp4-027-L` Chance Day, `PL!SP-pb2-014-P＋`,
`PL!S-bp7-012-N`. All three accept-only in their tests, so **no decline path is covered for any
of the five.**

**A stale doc comment fixed in the same file.** It read: *"`exclude_self: true` keeps the
performing live card out of the move — it is on the stage, so a formation change that could
include it would be nonsense."* Wrong twice over: the parse carries **no** `exclude_self` on this
card (removed by the D2 fix, one of the five it applied to), and the rationale contradicts the
text, whose 「自分のステージにいるメンバー」 includes the live. Replaced with the real reason and
a pointer to the test that pins the activator's own area being offered.

**S13. `PL!SP-pb2-022-R` — a two-condition effect whose negative fails both conditions at once.**
「『5yncri5e!』のメンバーがセンターエリアに移動したとき」 — the negative moves a μ's member to
centre *and* moves the only 5yncri5e! member off centre, so a group-filter bug and a
centre-filter bug produce the same result. Fix: move the 5yncri5e! member off centre while the
μ's member is nowhere near centre.

**S14. `position_change_multi_test.rs` — two files hold opposite `exclude_self` assumptions, untested against each other.**
`PL!HS-bp2-006-R` is commented as "source card's position is not excluded when
`exclude_self = false`", while the same file's own 澁谷かのん test asserts the opposite convention
for a card printing the **same** それぞれ shape. Neither runs the self-offering case for
`PL!HS-bp2-006-R`, so the contradiction is untested. This is the D2 territory from
`PARSER_CONSOLIDATION_SWEEP.md` seen from the test side.

**S15. `live_start_group_member_four_heart_score_test.rs` fixture comments are ambiguous.**
They read "hearts=4 (2+2)" and "hearts=3 (1+2)", which reads as implying heart03 on both cards.
`cards.json` gives `PL!SP-sd1-001-SD` = heart02:2 + heart06:2 and `PL!SP-sd1-002-SD` =
heart02:1 + heart06:2 — the **totals** are right, so the boundary is genuinely exercised, but
the comment is not a reliable spec. `PL!SP-sd1-001-SD` also has an untested 登場 clause
「自分のエネルギー6枚につき、カードを1枚引く」.

---

## RETRACTED — `PL!HS-bp6-029-L` Proof: I reported a defect that does not exist

**The engine is correct. My test was wrong, twice, and I reported the second failure as a
live bug.** This section replaces my earlier "DEFECT FOUND" entry, which claimed an off-by-one
with the 合計20以上 threshold in 10..=19. That claim was false and is withdrawn.

The card prints:

> {{live_start.png|ライブ開始時}}自分のステージにいる『蓮ノ空』のメンバーの**コストが合計20以上**の
> 場合、デッキの上のカードを2枚見る。その分から1枚を手札に加え、残りをデッキの上に戻す。
> **30以上**の場合、さらにこのカードの必要ハートをheart0heart0減らす。

The parse is faithful — `count: 20, operator: ">=", comparison_type: "cost", aggregate: "total",
group_names: ["蓮ノ空"], cost_total: 20`, and `cost_total: 30` for the second tier.

### What I measured

`proof_threshold_diagnostic` reads the prompt **before** answering anything:

| total | prompt | look fired |
|---|---|---|
| 6 | `None` | no |
| 10 | `None` | no |
| 12 | `None` | no |
| 13 | `None` | no |
| 27 | `SelectCard`, 2 options | **yes** |
| 21 / 19 | `SelectAutoAbility` first | queued behind Proof's own 自動 |

The engine's own trace confirms the gate doing the right thing:

```
[EFFECT] activation check: action=look_and_select has_condition=true
[COST_TOTAL] sum=6 threshold=20 operator=>= result=false location=Some("stage") target=self
[CONDITION] action=look_and_select passed=false
```

Condition false, gate blocks, no execution trace follows. That is `can_activate_effect`
(`resolver.rs:629-688`) doing its job.

### How I got it wrong, twice

1. **First attempt asserted `hand.contains(prize)`.** It failed, and I reported an engine
   defect. But the LiveStart draw also writes to hand — the prize arrived there without this
   look ever running. The observable was shared with another rule, so it could not distinguish
   "the look ran" from "something else moved a card". That mistake produced a confident,
   specific, wrong bug report including a fabricated threshold range.
2. **Second attempt read `looked_at_cards.len()` after draining.** Answering the selection
   empties that pool, so *every* row read false, including 21 and 27.
3. **Correct observable: the `SelectCard` prompt, read before it is answered.** Neither the
   hand nor the looked-at pool is a safe signal; the prompt is.

All three are now recorded in the test file itself, so the next person does not re-derive them.

### What this episode actually establishes — and it is a testing lesson, not a bug

The real finding is about **observables**, and it applies to every test in this sweep. An
assertion is only evidence if the thing it inspects is written **solely** by the effect under
test. `hand`, `waitroom`, `deck`, `looked_at_cards` and `revealed_cards` are all shared
territory: draws, costs, reveals and selections write to them too. The prompt, the modifier
total, and the score are effect-specific.

Two of my three tests in this stretch passed for the wrong reason before a precondition caught
it; this one passed for the wrong reason and I initially treated the failure as a product bug.
**When a test fails, first ask whether the test is wrong.** That should have been the first
hypothesis here, not the last.

### Kept

* `proof_cost_total_of_exactly_twenty_triggers_the_look` — asserts the `SelectCard` appears
  with 2 options at a total of exactly 20, after draining Proof's own 自動 prompts.
* `proof_cost_total_of_nineteen_does_not_trigger_the_look` — asserts **no** `SelectCard` and an
  empty `looked_at_cards` at 19. Together they bracket the gate with one cost of difference,
  which the pre-existing suite (9, 22, 28, 30) never did.
* `proof_threshold_diagnostic` — the sweep above, as a permanent record.

**Still a real (minor) finding in this file**, unrelated to the gate:
`issue12_proof_look_and_select_cost_20plus` stages 13 + 15 = **28** and asserts **only**
`get_need_heart_modifier(proof, Heart00) == 0` — a statement about the **30** tier. It never
checks the look ran, so deleting the 20 clause entirely would leave it green. It is misnamed
against its body. The new pair above now covers what it claimed to.
## An engine finding — the yell-revealed branch contradicts the rules data (I had this backwards first)

`Card::has_blade_heart()` (`core/card.rs:4311-4317`) is

```rust
self.blade_heart.is_some()
    || self.special_heart.as_ref().is_some_and(|sh| !sh.hearts.is_empty())
```

I flagged this as a conflation of ブレードハート with a live's heart icons. **That was wrong,
and the data settles it the other way.**

**The ruling, from `cards/qa_data.json` entry `Q112` (2025.05.30):**

> 『…エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、…』
> について。{{icon_b_all.png|ALLブレード}}、{{icon_score.png|スコア}}、
> {{icon_draw.png|ドロー}} はブレードハートに含まれますか？
> **「はい、含まれます。」**

So the score and draw icons **do** count as ブレードハート, and `special_heart` is precisely
where they live. The loose `has_blade_heart()` is the correct reading, and the data supports the
structure: `blade_heart` (the B★/ALL icon) is on 872 members and 273 lives, while
`special_heart` is on 132 **lives only** — and for all 50 that have it without a `blade_heart`,
its entire content is `{'score': 1}`. The two fields model two different printed things and both
are needed.

**The remaining defect is the opposite one, and I could not demonstrate a fix.**
`has_blade_heart_strict()` (`card.rs:4319-4321`) is just `blade_heart.is_some()` — it excludes
exactly the icons Q112 says to count — and `ability/condition/card.rs:145-155` prefers it when
`location == "revealed_cards" && yell_trigger == true`.

I removed that special case and ran `no_blade_heart_reveal_gain/`: **22/22 pass identically with
and without it.** So the site does not evaluate these cards. I confirmed the three Q112 cards
(`PL!SP-bp2-015-N` / `-020-N` / `-021-N`) are the only parses carrying `has_blade_heart` +
`revealed_cards` + `yell_trigger` together, so they *should* take that branch — and yet nothing
moves. The ruled loose predicate is reached by four other sites (`condition/state.rs:801`,
`condition/compound.rs:46`, `util::check_card_property:1605`, and the followup path), all
already correct.

**The change is reverted and is not claimed as a fix.** The comment at the site now records the
measurement so it is not re-derived. What remains true:

* the ruled predicate is the loose one — certain, from Q112;
* `card.rs:145-155` still prefers the narrow one on the yell-revealed path, contradicting Q112
  in principle;
* **nothing observable changes**, so that branch is dead for these cards or serves an uncovered
  path;
* the real fix is the same one this sweep keeps reaching: **one answer for `has_blade_heart`,
  not five call sites choosing between two predicates** — exactly what F1 did for
  `exclude_self`. That needs a test that exercises that specific site first, or removal once
  the sites are consolidated.

### The ruling, now pinned by tests

* `yell_score_icon_live_counts_as_a_blade_heart_q112` — `LL-bp5-001-L` has
  `blade_heart: None` and `has_score_icon()`, so it is the discriminating fixture: only the
  ruled (loose) reading makes it a ブレードハート. The premise is asserted from card data,
  including `!has_blade_heart_strict()`.
* `a_draw_icon_live_carries_a_real_blade_heart_so_it_cannot_expose_the_q112_difference` —
  Q112's third icon is **untestable**, and the data says why: **0 of 82** draw-icon lives lack a
  `blade_heart`, so every one of them answers TRUE under both predicates. My first attempt used
  `PL!-bp3-021-L` and its own precondition caught that it *does* carry a blade heart. This test
  documents the obstruction so nobody "adds" a ドロー test that silently re-tests ALLブレード.
* The pre-existing ALL-blade test could never have caught this: an ALL-blade live has a real
  `blade_heart`, so it looks identical under either predicate.

### Other rulings in the same file

* **Q120 — CLOSED this pass.** 国木田花丸 `PL!S-bp2-007-R＋` ab#0:
  「…我的.liveが1枚以上[公開]されているとき、**自分の手札が7枚以下の場合**、カードを1枚引く。」
  Q120 (2025.06.06) rules on the ORDER: with 7 cards in hand and a ドロー blade-heart live
  revealed, her ability **cannot** draw — 「発動した自動能力を使うのは、エールで公開された
  ドローブレードハートの効果を解決したあとです。」 The draw blade heart resolves first, hand
  7 → 8, and only then is 「手札が7枚以下」 evaluated, which by then is false.

  The existing file (`live_reveal_hand_limit_draw_test.rs`) staged an empty hand and 8 — it
  straddled the printed bound with **nothing on it**, so `<= 7` and `< 7` were
  indistinguishable — and never covered the ordering at all. Added:
  * `live_revealed_in_yell_with_exactly_seven_hand_cards_still_draws` — the bound itself.
  * `q120_the_draw_blade_heart_resolves_before_the_hand_limit_is_checked` — hand 7 → 8 with
    **exactly one** card leaving the deck (asserted by deck depletion, not by identity: an
    identity assertion proved unreliable, since the card reaching hand is not the one staged
    at index 0).

  Both pass, so the engine honours the ruling. Note this family sets `revealed_cards` **by
  hand** (as `yell_real_yell_no_blade_heart_grants_heart_test.rs` also notes), so it proves the
  condition but never the real yell pipeline.

### The yell probe: two real findings, and two of my own tests corrected

Chasing Q182 meant instrumenting `miracle_wave_with_exact_hearts` and printing the state the
condition actually sees. `yell_real_yell_no_blade_heart_grants_heart_test.rs` documents that a
live requiring nothing produces **no yell at all**, and that turned out to be the key. Using
`initial_yell_revealed_cards` — the field that records the yelLED set, which I had not known to
exist — rather than `revealed_cards`, gives:

```
yell_occurred=false  initial_yell=[14772 ×7]  revealed=[]          (blade-hearted board)
yell_occurred=false  initial_yell=[14772 ×7]  revealed=[14772 ×7]  (blade-less board)
```

**Finding 1 — RETRACTED. The duplicate ids were a harness artefact, not an engine defect.**

I wrote this up as "CONFIRMED and reproducible" and as "the widest issue in the sweep". **That was
wrong**, and I am retracting it rather than softening it.

The observation was real:

```
stage members      = [18897, 31151, 12605]                    ← three distinct ids
yell_occurred      = false
initial_yell (raw) = [11329, 11329, 11329, 11329, 11329]      len=5  distinct=1
revealed_cards     = [11329, 11329, 11329, 11329, 11329]
```

The mechanism is the **test's own deck construction**, which I had read past twice. `TestGame`'s
id helpers document it themselves (`tests/helpers/mod.rs:234-243`):

> `new_id` — "Past the 11-copy pool: fall back to the shared template id (same as `id()`).
> … at the cost of aliasing: **instances past 11 are NOT distinct**."

And every fixture here fills its deck by pushing **one** id many times — `fill_decks` with
40+, and this file's `setup()` with 60:

```rust
let no_bh = game.id(NO_BLADE_HEART_CARD);
for _ in 0..60 { game.state.player1.main_deck.cards.push(no_bh); }
```

So the deck holds sixty entries all equal to one id. Drawing five of them correctly produces
`[11329 ×5]`. The same applies to MIRACLE WAVE's `[14772 ×7]` from `fill_decks`. The engine is
behaving correctly for the board it was given, and `phases.rs:402-403` is a plain snapshot of
`revealed_cards` taken at yell time — nothing exotic.

**What this costs me to accept:** the "duplicate id in a revealed set" story was clean, specific
and had a wide blast radius, and it was wrong for the simplest reason — I did not check how my
own test built its deck. It is the third artefact I have found this session after the Proof
observable and the MIRACLE WAVE `hand`/`looked_at` observables.

**The transferable rule, now stated in the tests themselves:** before drawing any conclusion
about the *contents* of a revealed, drawn or selected set, confirm the deck was built from
**distinct instances** (`new_id` per card, not one id pushed N times). Bulk-filler decks alias
by design, so any set drawn from one is aliased too.

**Finding 2 — the fixtures could never have yelled.** `PL!S-bp3-001-P` and `PL!S-bp2-011-N`
both print `score: None`. A performance only yells when it over-performs, so with zero-score
members **`yell_occurred` is always false** and the whole `revealed_cards` path is skipped.

### Two of my own tests were green on a false premise, and I corrected them rather than leaving them

Both MIRACLE WAVE tests I added earlier passed, and both messages described something that does
not happen:

* `..._a_lone_no_blade_heart_count_of_zero_scores_four_...` claimed *"the only cards yelled both
  have blade hearts"*. In fact **nothing was yelled** — `revealed_cards` is empty, so the count of
  blade-less cards is 0 vacuously. Renamed to
  `miracle_wave_zero_revealed_cards_satisfies_the_no_blade_heart_condition_q182`, because that
  is what it actually covers — and that is exactly Q182's question and answer. So Q182 is now
  pinned after all, by the board I had already built for the wrong reason.
* `..._when_both_conditions_are_false_...` claimed *"two blade-less cards were yelled"*. The
  revealed set is `[14772 ×7]`. The assertion and its outcome are right (A false, B false ⇒ the
  printed 7 stands); only the stated mechanism was wrong.

Neither was caught by a failure — they were caught by refusing to leave an unverified claim in
a message. **A green test whose comment lies is worse than no test**, because it teaches the next
reader a false fact about the game.

* **Q182 — now closed** by the renamed test, with the caveat above: it covers the zero-revealed
  route only. The non-vacuous case (a real yell revealing blade-less cards) still needs a board
  with **scored** members, which this fixture cannot provide.
* **Q172 — the two halves were untested; one is now pinned and one is an open question.**

  Card found by its printed clause rather than by QA's `related_cards`, which are unreliable —
  Q172 points at `PL!-pb3-026-L`, which **does not exist**; the real card is
  **`PL!-bp3-026-L`** Oh,Love&Peace! (a live, score 6). Its ライブ成功時 clause is the
  ハートの総数 comparison; all 11 tree references use the card as a *fixture*, not as the
  subject of this clause.

  * **Pre-existing test MISLABELLED.** `lovepeace_q172_ability_gained_hearts_count_but_not_blade`
    (`real_card_phase_walkthrough_and_ability_suite_test.rs:2590`) is named for Q172 and
    asserts **neither half**:
    - it stages `PL!-sd1-020-SD` as the live, so **Oh,Love&Peace! is never on the board** and the
      ハートの総数 clause does not exist in the fixture at all;
    - there are **no ability-gained hearts anywhere** in it;
    - its blade claim rests on a low-requirement live succeeding while cheered cards carry
      blades — a card with **no blade clause**, so nothing about blade hearts counting toward a
      heart total is demonstrated.
    It passes, and it teaches a false fact about the game. Same class as the Proof and
    MIRACLE WAVE messages I corrected.

  * **New coverage** in `q172_heart_total_includes_ability_but_not_blade_test.rs`, built on a
    16-vs-16 tie so each test moves exactly one thing:
    - `q172_a_tie_...` — a tie keeps the printed score. **Passes.**
    - `q172_blade_hearts_are_not_included_in_the_heart_total` — the card's own printed
      ライブ開始時 ブレード×3 grant does **not** move the heart total. **Passes**; Q172's second
      half 「ただし、エールによって得たブレードハートは含みません。」 is now pinned.
    - `q172_ability_gained_hearts_are_included_in_the_total` — Q172's first half 「はい、能力
      によって得たハートも含みます。」 **Fails (6, expected 7) and is marked `#[ignore]` as
      UNRESOLVED.** Not reported as a defect: `add_heart_modifier` carries no duration and the
      stage-heart pipeline does consume `heart_modifiers`, so the fixture is not obviously
      wrong, and I have already produced one false defect this session from a bad observable.
      The narrow open question is whether the comparison reads a live stage-heart total or a
      value captured before the modifier applied.

  * A fixture trap worth recording: `TestGame::id` consumes a pool slot **per call**, so a
    second `game.id(LOVEPEACE)` returns a *different* instance and `set_live_card` fails with
    "Selected card not found in hand". Ids must be threaded through, not re-looked-up.

---
## The one rule this sweep keeps re-learning

An assertion is only evidence if what it inspects is written **solely** by the effect under
test. `hand`, `waitroom`, `deck`, `looked_at_cards` and `revealed_cards` are all shared
territory — draws, costs, reveals and selections write to them too. The **prompt**, the
**modifier total** and the **score** are effect-specific.

Two of my own tests here passed for the wrong reason and a precondition assertion caught both;
a third failed for the wrong reason and **I initially reported it as a product bug** (see the
retracted Proof entry). When a test fails, the first hypothesis should be that the test is
wrong.

---


`Card::has_blade_heart()` (`engine/src/core/card.rs:4311-4317`) is

```rust
self.blade_heart.is_some()
    || self.special_heart.as_ref().is_some_and(|sh| !sh.hearts.is_empty())
```

I flagged this as a conflation of ブレードハート with a live's heart icons. **That was wrong,
and the data settles it the other way.**

**The ruling, from `cards/qa_data.json` entry `Q112` (2025.05.30):**

> 「…エールにより公開された自分のカードの中にブレードハートを持つカードがないとき、…」
> について。{{icon_b_all.png|ALLブレード}}、{{icon_score.png|スコア}}、
> {{icon_draw.png|ドロー}} はブレードハートに含まれますか？
> **「はい、含まれます。」**

So the score and draw icons **do** count as ブレードハート, and `special_heart` is precisely
where they live. The loose `has_blade_heart()` is the correct reading. The data supports this
structurally too: `blade_heart` (the B★/ALL icon) is on 872 members and 273 lives, while
`special_heart` is on 132 **lives only** and, for every one of the 50 that has it without a
`blade_heart`, its entire content is `{'score': 1}`. The two fields model two different
printed things and both are needed.

**The real defect is not where I first thought, and I could not demonstrate a fix.**
`has_blade_heart_strict()` (`card.rs:4319-4321`) is just `blade_heart.is_some()` — it excludes
the score and draw icons Q112 says to count — and `ability/condition/card.rs:145-155` prefers
it when `location == "revealed_cards" && yell_trigger == true`.

I removed that special case, regenerated nothing (engine-only change), and ran
`no_blade_heart_reveal_gain/`: **22/22 pass identically with and without it.** So the site is
not what evaluates these cards. I confirmed the three Q112 cards
(`PL!SP-bp2-015-N` / `-020-N` / `-021-N`) are the only parses in the corpus carrying
`has_blade_heart` + `revealed_cards` + `yell_trigger` together, so they *should* take that
branch — and yet behaviour does not move. The loose, ruled predicate is reached by other
sites (`condition/state.rs:801`, `condition/compound.rs:46`, `util::check_card_property:1605`),
all of which already use it.

**So the change is reverted and is not claimed as a fix.** What remains true and recorded:

* the ruled predicate is `has_blade_heart()` (loose), per Q112 — that part is certain;
* `condition/card.rs:145-155` still prefers the narrow one on the yell-revealed path, which
  contradicts Q112 in principle;
* **nothing observable changes**, so that branch is either dead for these cards or serves a
  path with no coverage at all;
* resolving it needs either a test that exercises that specific site, or the same consolidation
  F1 applied to `exclude_self` — one answer for `has_blade_heart` instead of five call sites
  choosing between two predicates. That is the real fix, and it is the same lesson as the other
  four findings in this file: **one concept, N sites, silently disagreeing.**

The comment at the site now records all of this, including that the edit was tried and
reverted, so the next person does not re-derive it.

**Also worth noting from the same file**, both currently untested and both precise:

* `Q182` (about MIRACLE WAVE): when **zero** cards are revealed by the yell, the score is **4** —
  the "0枚" condition is satisfied. My new A-true test stages *members*, not the zero case, so
  this documented case has no coverage.
* `Q172`: counting ハートの総数 includes ability-gained hearts but **excludes** blade hearts
  gained by the yell.
* `Q120`: a draw blade heart's effect resolves **before** the auto ability that references it.


`PL!HS-bp6-008-R` 桂城泉 (スコア4以下 at 4/5, スコア2以下 at 2/3 — both boundaries genuinely at
the bound, `<=` correct); `PL!S-pb1-015-N` 津島善子 (both branches of the `か` at their bounds,
now including the live branch this pass added); `PL!SP-pb2-032-N` 6/8 stacking (0/5/6/7/8/10);
`PL!S-bp5-013-N` heart04 ≥ 4 (3 vs 4); `PL!S-bp7-009-R` コスト4以下 (4 vs 5); `PL!-PR-021-PR`
ちょうど7枚 (6/7/8); `PL!HS-pb1-025-L` 10枚以上 (9/10); `PL!N-sd2-026-P` 4-blade boundary with
wrong-group isolation; `PL!HS-bp2-006-R` per-unit (does stage a non-distractor);
`PL!N-bp7-007-R＋` ab#1 (6/7/8/12 exact differences); `PL!S-sd1-001-SD` /
`PL!N-sd2-005-SD2` choice-identity pinning; and `all_or_any_three_heart_member_filter_look_test`
+ `or_card_types_heart05_threshold_look_test` as the **exemplar** pattern — they read
`filtered_indices` directly, bracket every bound at its boundary, and assert that rejected
cards are *accounted for* in the waitroom rather than merely absent from hand.
---

## Data findings: a card_no is corrupt, and 「R3BIRTH」 has no representation at all

Individual audit of `PL!N-bp7-024-N` 鐘嵐珠:

```
{{toujyou.png|登場}}自分のステージに『R3BIRTH』のメンバーが3人いる場合、ライブ終了時まで、heart01を得る。
```

Parse: `group_names: ["R3BIRTH"]`, `count: 3`, `location: stage`, `card_type: member_card`,
`gain_resource { count: 1, heart_colors: [heart01], duration: live_end }`.

**Finding 1 — `cards.json` contains a corrupt card number.** The key
`PL!N-bp1-023-PRproteinbar` resolves to **ミア・テイラー**. The number it looks like it was
meant to be, `PL!N-bp1-023-PR`, is **absent from `cards.json`** but still resolves through the
baked database, so the mangled identifier is the only way to reach that card — and a test file
depends on it. "proteinbar" spliced into a card_no is a data defect, not a naming convention.

**Finding 2 — no card in the corpus has an `R3BIRTH` series.** The only `series` values are
ラブライブ！スーパースター!! (395), 虹ヶ咲学園スクールアイドル同好会 (390), μ's (282),
サンシャイン!! (248), 蓮ノ空女学院スクールアイドルクラブ (239). R3BIRTH is a sub-group the
`series` field does not model, and the only `R3BIRTH` string anywhere in `engine/src` sits
inside the generated `core/cards_gen.rs` card-number table — there is no group→series alias
for it.

**Finding 3 — the test's stated premise is false, and the real discriminator is unresolved.**
The positive test's comment claims "3 R3BIRTH members on stage", but all four fixtures are
虹ヶ咲学園スクールアイドル同好会. The two behavioural boards differ by exactly one card —
positive `PL!N-bp1-024-PR` 鐘嵐珠, negative `PL!-sd1-010-SD` 高坂穂乃果.
`n_bp7_024_n_fixture_group_membership_is_documented` prints the fixtures, and why that is
suspicious rather than reassuring:

```
PL!N-bp7-024-N            鐘 嵐珠     series=虹ヶ咲学園   blade=Some(Heart01: 1)
PL!N-bp1-023-PRproteinbar  ミア・テイラー series=虹ヶ咲学園   blade=None
PL!N-bp1-024-PR           鐘 嵐珠     series=虹ヶ咲学園   blade=Some(Heart01: 1)
PL!N-PR-012-PR            三船栞子     series=虹ヶ咲学園   blade=Some(Heart04: 1)
```

The fixtures that satisfy the condition are the ones carrying a **heart01 blade heart**. That
may be the real discriminator or a coincidence of this fixture. **No defect is claimed:** either
an alias maps 『R3BIRTH』 somewhere and the comment is simply wrong, or the group is
unsatisfiable and `count: 3` is met vacuously — which would be severe, since every condition
gated on a sub-group name would then fire unconditionally. Settling it needs the group→series
resolution path, which I have not localised. Open.

**Also fixed:** the positive case asserted `> 0` for a heart the parse carries as `count: 1`.
Now `== 1`, with the sibling two-member case already at `== 0`, so the pair brackets the value.
---

## 13 phantom card entries: every contaminated card_no is a byte-exact duplicate

Blast radius of the `PL!N-bp1-023-PRproteinbar` finding I hit while auditing 鐘嵐珠.

`cards.json` holds **2639** keys. Thirteen contain a lowercase English fragment spliced into
the card_no — twelve `*PRproteinbar` and one `LL-PR-007-PRLoveLive!Days`. Both fragments look
like copy-paste out of card *text*: "proteinbar" out of an ability, "LoveLive" out of a name.

I checked each of the 13 against every other entry for an exact signature match (ability
text plus `cost`, `type`, `base_heart`, `blade_heart`, `need_heart`, `special_heart`, `series`):

```
contaminated keys total          : 13
  EXACT DUPLICATE of a clean key: 13
  no exact duplicate            : 0
```

Every one, without exception:

```
LL-PR-007-PRLoveLive!Days          == LL-PR-001-PR
PL!N-bp1-014-PRproteinbar          == PL!N-bp1-014-N
PL!N-bp1-015-PRproteinbar          == PL!N-bp1-015-N
PL!N-bp1-018-PRproteinbar          == PL!N-bp1-018-N
PL!N-bp1-019-PRproteinbar          == PL!N-bp1-019-N
PL!N-bp1-020-PRproteinbar          == PL!N-bp1-020-N
PL!N-bp1-023-PRproteinbar          == PL!N-bp1-023-N
PL!N-sd1-005-PRproteinbar          == PL!N-sd1-005-SD
PL!N-sd1-012-PRproteinbar          == PL!N-sd1-012-SD
PL!N-sd1-013-PRproteinbar          == PL!N-sd1-013-SD
PL!N-sd1-016-PRproteinbar          == PL!N-sd1-016-SD
PL!N-sd1-021-PRproteinbar          == PL!N-sd1-021-SD
PL!N-sd1-022-PRproteinbar          == PL!N-sd1-022-SD
```

So the corruption is **purely additive**: the legitimate entries are all intact and the 13
contaminated keys add 13 phantom copies of cards that already exist. Nothing is lost, and no
unique printing is unreachable — which is why the corpus still parses and the suite is green.

**Why it still matters.**

* The pool contains **13 phantom cards**, so any count over "how many cards are in the database"
  is wrong by 13, and `fill_decks`-style fixtures can resolve a printing through either key.
* A test that stages `PL!N-bp1-014-N` *and* `PL!N-bp1-014-PRproteinbar` would be staging the
  **same printing twice** and could mistake a duplicate-name interaction for a distinct-card one.
* The mangled strings are the kind of thing that makes a card_no look hand-typed when it is not.

**Fixed on my side.** `subunit_stage_count_gated_debut_heart_test.rs` had been reaching for
`PL!N-bp1-023-PRproteinbar` — a phantom. Its fixtures now use the clean keys
`PL!N-bp1-023-N` / `PL!N-bp1-024-PR` / `PL!N-PR-012-PR`, so they name real pool entries and stay
resolvable if the duplicates are ever deleted.

**Not changed in the corpus.** Removing 13 keys touches `abilities.json`, `abilities.bin`, the
generated `cards_gen.rs` and every test that references a duplicate, and that is a corpus-wide
edit for a defect with no behavioural consequence today. It needs to be one deliberate pass, not
a side effect of an audit.
---

## `PL!SP-bp7-027-L` — the +1 was an inequality; the second clause is properly covered

Individual audit, two clauses on one live card (printed score 5):

```
{{live_start.png|ライブ開始時}}エネルギー置き場にあるエネルギー1枚をエネルギーデッキに置いてもよい：
  自分のエネルギーが相手より多い場合、このカードのスコアを＋１する。
{{live_success.png|ライブ成功時}}自分のエネルギーデッキから、エネルギーカードを1枚ウェイト状態で置く。
  そのエネルギーカードは、次のターンのアクティブフェイズにアクティブしない。
```

**Clause 1 — fixed.** The score assertion was `assert!(get_score_modifier(live) >= 1,
"energy still ahead after paying -> score +1")`. The text is a flat 「スコアを＋１する」, not a
per-unit modifier, so applying the LiveStart gate **twice** passed. The decline sibling already
asserted `== 0`, so the pair now brackets the value: `== 1` / `== 0`. Passes.

**Clause 2 — already thorough, recorded as a clean negative.** `live_success_waited_pl_sp_bp7_027_l_test.rs`
pins the whole printed behaviour, including the part that is easy to miss:

* one energy placed, `active_count() == 0` (WAITED);
* `mods.is_delayed_cannot_active(placed_energy)` is true, and false for the live itself;
* after one `pass()` — a **new turn** — the lock is **still** set while the phase is `Active`;
* after the energy phase it has been consumed.

That last pair is the one worth naming: it distinguishes 「次のターンのアクティブフェイズに
アクティブしない」 (the lock survives exactly one active phase) from both 「never activates」
and 「expires immediately at turn start\". A file this thorough is worth pointing the next
auditor at as the model.

---

## Method note: the inequality scan keeps needing a manual pass

Third time this sweep that a targeted `>`-shaped scan produced a **false positive**, so the
pattern is worth stating plainly:

* `>=` / `>` on a **printed positive amount** → genuine gap, fix it;
* `>` appearing inside `assert!(!... > 0)` → a **negation**, correct as written;
* an amount that **legitimately scales** (per-member, per-unit, stacking) → correct as written.

So the scan is a *candidate generator* and never the finding. Each hit needs the printed amount
read before it is touched. Two genuine fixes and two false positives so far, from 51 raw hits.
---

## Two more amount fixes, and a boundary that was sitting ON the line

### `PL!S-PR-042-PR` / `PL!SP-PR-022-PR` — single heart icons asserted as `> 0`, and no 5-member case

Both print **合計6人いるかぎり** with **one** icon of each colour:

```
PL!SP-PR-022-PR  若菜四季  {{jyouji}}自分と相手のステージにメンバーが合計6人いるかぎり、{{heart_02}}{{heart_03}}を得る。
PL!S-PR-042-PR   小原鞠莉  {{jyouji}}自分と相手のステージにメンバーが合計6人いるかぎり、{{heart_02}}{{heart_04}}を得る。
```

* **Amount.** `get_heart_modifier(me, H02) > 0` accepted 2 or 3, so applying the 常時 twice was
  invisible. Two single icons side by side is exactly +1 each. Now `== 1` for both colours.
* **Boundary.** Every test in this file staged **3 + 3 = exactly 6** — sitting *on* the bound, so
  the inclusive side was the only thing provable. A `>= 5` reading, or a sum that counted the
  acting member twice, passes every existing test. Added
  `sp_pr_022_pr_five_combined_members_is_one_short_of_the_bound` staging **2 + 3 = 5**, with the
  staged total asserted as a precondition so the fixture cannot drift across the line.

Worth noting from the same file: `karin_exact_six_members_grants_heart02_and_heart05` and
`karin_counts_six_physical_instances_of_the_same_member` were already there, so the physical-
instance-vs-print distinction for a *combined* count is covered. That is the distinction a
per-card clause usually loses, and it was handled here.

The new test caught **my own** staging error on the first run — I wrote 3 + 3 while claiming 2 + 3,
and the precondition assertion stopped it. That is the third time this session a premise
assertion has caught me rather than a real defect, which is the argument for writing them even
when the code under test is not in doubt.

### `PL!-bp4-018-N` 矢澤にこ — two blade icons asserted as `>= 2`

```
{{jyouji.png|常時}}自分の成功ライブカード置き場にあるカードのスコアの合計が相手より高いかぎり、
  {{icon_blade.png|ブレード}}{{icon_blade.png|ブレード}}を得る。
```

Two blade icons printed side by side, so the grant is exactly 2 and **not** a per-point-of-lead
total. `>= 2` accepted 3 or 4, so a modifier scaled by the score difference would have passed.
Now `== 2`, with the equal-score case above already at `== 0`. Passes.

### A note on my own scan output

My candidate list said `PL!S-bp4-018-N`, which does not exist. The real card is
`PL!-bp4-018-N`. That is the third wrong card number I have typed in this sweep, alongside
`PL!SP-bp2-050-L` and `PL!-pb3-026-L` from the hunter reports. The rule holds: **read the number
out of the test file, then look it up in `cards.json` — never from a list or from memory.**
---

## `bp7_karin_wait_blade_limit_test.rs` — the central fixture is not the card the comment names

Individual audit of 朝香果林 `PL!N-bp7-004-R`:

```
起動：エネルギー置き場にあるエネルギー1枚をこのメンバーの下に置く：相手のステージにいる、
元々持つブレードの数がこのメンバーの下にあるエネルギーカードの枚数に1を足した数以下の
メンバー1人をウェイトにする。
```

This file is otherwise the best in the directory — 7 tests, and it pins the genuinely subtle
part: the limit is **dynamic**, proved by showing the *same* member going from not-waited to
waited once pre-seeded energy raises the limit. It also asserts `filtered_indices == [0, 1]` so
two eligible members really present a choice, and that the energy cost offers **all** energy
cards unfiltered. Excellent work.

**The problem is the fixture.** `ENEMY_BLADE4` is `PL!N-bp1-001-R` 上原歩夢, commented
「上原歩夢, original blade **4**」. From `cards/cards.json`:

```
PL!N-bp1-001-R  上原歩夢  cost=9  blade_heart = None  base_heart = {'heart01': 3}
```

**Its original blade count is 0, not 4.** Every print of 上原歩夢 with a cost-9 body has
`blade_heart: None`; her blade-bearing printings are different bodies (`PL!N-bp1-013-N`,
`-pb7-001-R`, `-pb3-001-R＋`, …) at different costs. So the file's discriminating member is a
blade-**0** member, and the two tests that carry the dynamic-limit proof describe it wrongly:

* `karin_does_not_wait_above_limit` — "blade-4 opponent (> limit 2) must NOT be waited";
* `karin_dynamic_limit_scales_with_energy_under` — "blade-4 member (<= limit 5) should be waited".

**Why this matters, and what I could not settle.** With limit 2 and a member whose ORIGINAL blade
count is 0, `0 <= 2` holds, so the printed clause says that member **should** be waited — yet
`karin_does_not_wait_above_limit` asserts it is not, and passes. So the engine is demonstrably
**not** comparing `cards.json`'s original blade value for this check, or is reading it from a
different source than the parse's `blade_heart`.

Two possibilities, and they are not equivalent:
1. the **comment** is simply wrong (the fixture is blade 0, the tests pass for some other reason),
   in which case the "above limit" case is not testing what it claims and the limit's upper edge
   is unpinned; or
2. the **engine** is wrong — 「**元々持つ**ブレードの数が…」 says ORIGINAL, and if the engine is
   consulting something mutable (a current blade modifier, or a default for a missing field) then
   every card using 「元々持つブレード」 is affected.

I have **not** determined which, and I am not guessing. Resolving it needs the engine's
blade-limit comparison read against this one fixture, which is a small, specific investigation
I would rather hand over named than half-finish.

**No test changed.** The file is excellent apart from this, and correcting it requires knowing
which of the two it is.

### A second, smaller note on the same clause

The boundary itself is unpinned even taking the fixture at face value. With one energy under her
the limit is **2**, and the tests stage blade 1 (waited) and blade 4 (not waited) — so `<= 2`
and `< 2` are indistinguishable. A blade-**2** member is the only value that separates them, and
no member in the corpus has an original blade total of 2 (the counts are 0 or 1 for the
ability-free cards available here), so the exact upper bound cannot be staged with real data at
this limit. It could be staged at a higher limit: with 4 energy under her the limit is **5**, and
`karin_dynamic_limit_scales_with_energy_under` uses blade 4 there — so **blade 5** is the missing
exact-bound fixture at *that* limit, and a blade-5 member does exist in the corpus.
---

## Karin localisation: where the 「元々持つブレード」 comparison actually happens

Continuing the 朝香果林 investigation I called a one-step investigation. Here is the trail.

**The filter field.** `util.rs:1267` declares `original_blade_limit: Option<u8>` on `CardFilter`,
populated at `util.rs:1801-1803` from `effect.blade_limit_any()` **only when
`original_value` is true** — which is what 「元々持つ」 sets. `card.rs:2415-2421` mirrors the same
gate for the JSON path, so 「元の」 and 「現在」 are deliberately distinguished in both readers.

**Where the number is compared.** `util.rs:1694` (`CardFilter::matches`) is the comparison, and
`util.rs:1746` carries the note that names its own ruling:

```
/// blade limits (Q195: original_blade_limit checks card.blade from DB)
```

So `original_blade_limit` reads `card.blade` **from the database**, not from a modifier — which
is the correct reading of 「元々持つ」 and rules out my hypothesis that it consults something
mutable.

**The dynamic part** lives in `effects/change_state.rs:85-116` (`resolve_blade_limit`), with
`blade_limit_from_energy_under` and `blade_limit_offset` producing
`energy_under + offset`, matching 「このメンバーの下にあるエネルギーカードの枚数に1を足した数」.
`card.rs:1014-1022` documents both dynamic forms.

**The one line that changes the operator** is `change_state.rs:156-158`:

```rust
if blade_limit_unreachable {
    blade_limit_operator = Some("<");
}
```

When the computed limit is unreachable the comparison is rewritten from `<=` to `<`. That is a
deliberate mechanism, and it is the one place where 「以下」 can silently become 「未満」.

**What I could not settle.** With `PL!N-bp1-001-R` 上原歩夢 at `blade_heart: None`, the printed
clause says `0 <= 2` is satisfied and she **should** be waited at limit 2, yet
`karin_does_not_wait_above_limit` asserts she is not and passes. Given that `util.rs:1746` says
the comparison reads `card.blade` from the DB, either `card.blade` is not derived from
`cards.json`'s `blade_heart` for this card, or the fixture behaves differently from the field I
read. Resolving it means comparing `Card::blade` against `cards.json`'s `blade_heart` for that
one card — a single function, but one I have not traced, and I will not guess at it.

**Two things this did establish**, both worth keeping:

1. 「元々持つ」 is genuinely implemented as a *distinct* predicate from 「現在」, at both the filter
   and the effect layer. That is the right shape and it is not accidental — it is gated on
   `original_value` in two independent readers.
2. The dynamic energy-under limit is a real engine feature with its own fields and docs, not
   something the tests happen to observe.
---

## Karin: narrowed to one untraced link

Picking the thread back up. The comparison is settled; one conversion is not.

**Settled — the comparison reads the database, not a modifier.**

* `util.rs:1267` — `CardFilter::original_blade_limit: Option<u8>`, a field **separate from**
  `current_blade_limit` (`util.rs:1279`).
* `util.rs:1801-1803` populates the *original* variant from `effect.blade_limit_any()`, gated on
  `effect.original_value_any()`; `util.rs:1811-1814` populates the *current* variant from the
  same value but on the other field. `card.rs:2415-2438` repeats the same gate for the JSON
  reader. So 「元々持つ」 and 「現在」 are separated **twice, independently** — that part is right.
* `util.rs:1694` is the comparison, and `util.rs:1746` records its own ruling:
  `/// blade limits (Q195: original_blade_limit checks card.blade from DB)`.

**Settled — the dynamic limit is a real feature, not a test artefact.** `resolve_blade_limit`
(`effects/change_state.rs:85-116`) implements both `blade_limit_from_energy_under` and
`blade_limit_from_cost_member` with `blade_limit_offset`, documented at `card.rs:1014-1022`,
matching 「下にあるエネルギーカードの枚数に1を足した数」.

**Flagged — `<=` can become `<`.** `change_state.rs:156-158`:

```rust
if blade_limit_unreachable { blade_limit_operator = Some("<"); }
```

This is the single place where a printed 「以下」 is rewritten to 「未満」. Deliberate, and
currently unpinned by any test.

**Not traced — the one link.** `Card::blade` is a plain `u8` (`card.rs:334`, and a second
declaration at `card.rs:728`). I traced the *read* of that field all the way to `util.rs:1694`,
but I did **not** find where `cards.json`'s `blade_heart` is converted **into** it. Searches for
`blade_heart` outside the `has_blade_heart` predicates and for any `count_blade` /
`blade_count` derivation returned nothing that performs the conversion.

**Why that single link decides the question.** 上原歩夢 `PL!N-bp1-001-R` has
`blade_heart: None` in `cards.json`. If that conversion yields `blade == 0`, then the printed
clause `0 <= 2` is satisfied and 朝香果林 **should** be able to wait her at limit 2 — and
`karin_does_not_wait_above_limit` is asserting the opposite of the printed text while passing.
If the conversion instead yields something `> 2`, the test's "blade 4" comment is simply wrong
and the limit's upper edge is unpinned.

Both readings are live, the file is otherwise excellent, and I am not picking one without
seeing the conversion. **The investigation is: find where `Card.blade` is assigned, and compare
it against `cards.json`'s `blade_heart` for `PL!N-bp1-001-R`.** That is one function.
---

## Karin RESOLVED: the test is right and I was wrong (fifth retraction)

`cards.json` carries **two independent blade fields**, and I read the wrong one.

```
PL!N-bp1-001-R  上原歩夢
  has a top-level "blade" key : True
  blade value                 : 4        ← the COUNT the comparison uses
  blade_heart                 : None     ← the blade TYPE/identity
```

* **`blade: u8`** — the blade **count**. `card.rs:328-329` declares it as its own top-level field
  with `serde(default = "default_blade")]`, and `default_blade()` is `0` (`card.rs:790-792`).
  **1465 of 2639** cards carry it.
* **`blade_heart: Option<BladeHeart>`** — the blade **icon identity** (which blade type, e.g.
  `{'b_heart03': 1}`). **1151 of 2639** cards carry it.

They are genuinely independent, and the corpus shows it: `PL!-PR-018-PR` is `blade=5` with a
single `b_heart01`; `PL!-PR-012-PR` is `blade=2` with `b_heart03`.

**So the engine is right and `bp7_karin_wait_blade_limit_test.rs` is sound.** `util.rs:1746`'s
note — `original_blade_limit checks card.blade from DB` — names the *count* field, and that is
exactly the one 「元々持つブレードの数が…」 means. 上原歩夢's blade is 4, so at limit 2
`4 <= 2` is false and 朝香果林 correctly cannot wait her; at limit 5 it is true and she can. The
dynamic-limit proof stands.

**What I got wrong, precisely.** I saw `blade_heart: None`, concluded "original blade 0", and
built a two-way engine-defect-or-bad-comment dilemma on it — without checking that the struct
has a *separate* `blade` field, which I had actually already read past at `card.rs:328` twenty
minutes earlier while looking for something else. This is the same failure as the Proof
`hand` observable and the aliased revealed set: **a conclusion drawn from one field, without
checking which field the code reads.** I had the answer on screen and didn't look at it.

**Residual from that file, which does stand:** the `<=` → `<` rewrite at
`change_state.rs:156-158` when the limit is unreachable is unpinned by any test, and at the
higher limit (4 energy → limit 5) a **blade-5** member is the missing exact-bound fixture, since
`PL!-PR-018-PR` is exactly that. That is a real, small gap — not the structural one I claimed.

### The transferable lesson, after five false alarms

Every wrong conclusion in this session came from the same move: **reading a plausible field and
assuming it was the one the engine uses.** `hand` (written by the LiveStart draw), `looked_at`
(emptied by answering), a bulk-fill deck (aliased by design), and now `blade_heart` beside
`blade`. In each case the right move was one more read of the field the code actually names.

The check that catches it is already in the tests: a precondition asserted from the data. It
caught me three separate times, and it would have caught this one too if I had written
`assert_eq!(card.blade, 4)` before building a theory on `blade_heart`.
---

## CORRECTION: the `<=` → `<` rewrite IS covered — and this file is the new exemplar

Last turn I said the operator rewrite at `change_state.rs:156-158` was "unpinned by any test".
**That was wrong.** I inferred it from reading `resolve_blade_limit` and never checked for a
test — the same move that produced the `blade_heart` error, and the reason I have now been
wrong six times in this session.

`bp7_q266_natsumi_blade_wait_test.rs` covers it explicitly, and covers the exact boundary at
**every** limit:

| costed member's blade | limit (`blade − 2`) | cases staged |
|---|---|---|
| **0** | −2 → unreachable | `q266_cost_b0_waits_nothing_incl_blade0` — a **blade-0** opponent must NOT be waited |
| **1** | −1 → unreachable | `q266_cost_b1_waits_nothing` — same |
| **2** | 0 | `q266_cost_b2_waits_blade0_not_blade1` — blade-0 **waited**, blade-1 not |
| **4** | 2 | `q266_cost_b4_waits_blade2_not_blade3` — blade-2 **waited**, blade-3 not |

The B=0 and B=1 rows are the unreachable guard — limit 0 with the operator forced to `<`, so
`blade < 0` and nothing qualifies, which is exactly what those two tests assert. And the B=2 and
B=4 rows each put a card **exactly on** the limit and the next one just above, so `<=` is pinned
on both sides at two different limits.

It also cites **Q266 by number** and quotes the official answer in the header, and documents
the `blade` values it relies on. It is a better exemplar than the one I had been pointing at.

### What this costs, stated plainly

Six of my conclusions in this session were wrong, and they share one cause: **I read a
plausible-looking piece of evidence and stopped.** `hand` (also written by the LiveStart draw),
`looked_at` (emptied by answering), a bulk-fill deck (aliased by design), `blade_heart` (the wrong
of two blade fields), the R3BIRTH group (no such series exists), and now an "uncovered" branch
that had four tests on it.

The first four would each have been caught by one premise assertion read from the data, and three
of them *were* — by tests I had just written. The last two were not, because I never wrote the
check.

The rule I'd hold myself to after this: **before calling anything uncovered, search for the test
that covers it.** Coverage claims are the easiest thing in this work to assert from reading code
instead of reading tests, and I made that mistake six times.
---

## R3BIRTH: re-checked on the RIGHT field this time

Having been wrong six times for reading a plausible field and stopping, I re-opened the R3BIRTH
question on the field I had **not** previously checked.

`core/card.rs:327` carries a `group` field distinct from `series`:

```rust
pub group: Box<str>,        // card.rs:327
```

with `#[cfg_attr(feature = "serde_support", serde(default = "default_group_from_series"))]`
(card.rs:325) — so `group` is absent from the source data and **derived from `series`** at load
time. My earlier claim was made having looked only at `series`, which is exactly the mistake I
just apologised for. So I checked `group` as well.

**Result: `group` is `None` for all 2639 cards.** The field does not exist in `cards.json` at
all; it is wholly derived. And the R3BIRTH fixture board:

```
PL!N-bp7-024-N     鐘 嵐珠     group=None   series=…虹ヶ咲学園スクールアイドル同好会
PL!N-bp1-023-N     ミア・テイラー  group=None   series=…虹ヶ咲学園スクールアイドル同好会
PL!N-bp1-024-PR    鐘 嵐珠     group=None   series=…虹ヶ咲学園スクールアイドル同好会
PL!N-PR-012-PR     三船栞子     group=None   series=…虹ヶ咲学園スクールアイドル同好会
PL!-sd1-010-SD     高坂 穂乃果   group=None   series=ラブライブ！
```

**So the finding stands, and this time it rests on having read both fields.** There is no
`R3BIRTH` in `series` and none in `group`; the derived `group` for every fixture is the
咲学園 string. Whatever satisfies 「『R3BIRTH』のメンバーが3人いる場合」 in
`subunit_stage_count_gated_debut_heart_test.rs` is therefore **not** a group match on the data
these cards carry, and the test's "3 R3BIRTH members on stage" comment remains false.

Unchanged and still open: whether that is a group-alias path I have not found, or the count
being met vacuously. The discriminator the fixtures actually show is the **heart01 blade
heart** on the two 鐘嵐珠 printings, not a group tag.

**The difference from the `blade_heart` error.** Both times I asserted a field's absence; the
first time I had only checked one of two fields, the second time I had checked both. The
conclusion happened to be the same, but only the second one was worth anything. That is the
whole difference, and it is why "check every field the struct declares" is the rule, not
"check the field you happened to see first".
---

# HANDOVER — open items, each with its next step

Consolidated so a next session can pick up without re-deriving anything. Every item states what
is **known**, what is **not**, and the single next action. Ordered by value.

Per the rule I learned six times the hard way: **before treating anything as uncovered, search
for the test that covers it.** Two items below are flagged as *possibly already covered* for
exactly that reason.

---

## 1. `PL!N-bp7-024-N` 鐘嵐珠 — what satisfies 「『R3BIRTH』のメンバーが3人いる場合」?  [OPEN]

**Known.** The parse is `group_names: ["R3BIRTH"], count: 3, location: stage`. `cards.json` has
no `R3BIRTH` in `series`, and `group` is `None` for all 2639 cards (derived from `series` via
`default_group_from_series`, `card.rs:325-327`). All four fixtures are 虹ヶ咲学園. The two tests
that pass and fail differ by one card, and the fixtures that satisfy the condition carry a
**heart01 blade heart**.

**Not known.** Whether a group-alias path resolves 『R3BIRTH』 somewhere I did not find, or
`count: 3` is satisfied vacuously — which would mean **every** condition gated on a sub-group
name fires unconditionally.

**Next step.** Find how a parsed `group_names` entry is matched against a card. Start at
`ability/util.rs:1605` (`check_card_property`) and the group comparison it calls, and follow it
to whatever reads `Card::group`. Then check one card with no matching series against a
group-gated condition directly.

---

## 2. `PL!-pb3-026-L` Oh,Love&Peace! — Q172 first half  [OPEN, possibly covered]

**Known.** 「ハートの総数を数えるとき、能力によって得たハートも含みますか？」→ 「はい、含みます。」
The blade half is **pinned and passing**
(`q172_blade_hearts_are_not_included_in_the_heart_total`). The ability-heart half scores 6
where 7 is expected and is marked `#[ignore]`d as UNRESOLVED.

**Not known.** Why. `add_heart_modifier` has no duration (`game_modifiers.rs:374`) and the
stage-heart pipeline does consume `heart_modifiers`, so the fixture is not obviously wrong.

**Possibly already covered.** `kasumi_bp5_002_heart_greater_test.rs` (lines 52, 95) sets a heart
modifier via `add_heart_modifier` and asserts a heart **comparison** — structurally the same
shape. **Read that first.** It may answer this outright, in which case my test is redundant
rather than the engine being wrong.

**Next step.** Read `kasumi_bp5_002_heart_greater_test.rs` before touching anything else here.

---

## 3. `PL!-bp4-018-N` 松浦果林 — the `<=` → `<` rewrite  [COVERED — do not redo]

`change_state.rs:156-158` rewrites the blade-limit operator to `<` when the limit is
unreachable. I claimed it was untested; **it is not** —
`bp7_q266_natsumi_blade_wait_test.rs` covers the unreachable guard at costed-blade 0 and 1, and
brackets the inclusive bound at two more limits. That file is the best-audited in the tree and
is the exemplar to point people at.

---

## 4. `PL!SP-pb2-050-L` Jellyfish — declining the formation change  [DEFECT, localised, unfixed]

**Known.** Declining the optional フォーメーションチェンジ leaves the stage
`[live, -1, -1]`: both members are gone from every zone yet recorded in `cards_moved_this_turn`.
Reproduced as `#[ignore]`d `KNOWN ENGINE DEFECT`.

**Localised to two faults** (both parses carry `optional: true`, so this is not a parser bug):
* **Fault A** — `optional` is lost when a `position_change` is a leg of a `sequential`.
  Jellyfish's node is the **root** effect (prompt skippable); 澁谷かのん's is nested at
  `.actions[1]` (prompt **not** skippable). Target: the sequential leg dispatch.
* **Fault B** — skipping an *optional* `position_change` corrupts the stage. Target: the resume
  path for a skipped `SelectTarget` in `execute_position_change_with_destination`
  (`position.rs:927`).

**Not done.** Fault A's fix reaches every multi-clause ability; Fault B's reaches every
single-target move. Neither was landed — both need a blast-radius pass first.

**Next step.** Instrument which dispatch path a nested leg takes versus a root effect. That
single measurement decides whether A is one line or a redesign.

---

## 5. `PL!S-bp7-003-P` 松浦果南 — inverted restriction target  [DEFECT, verified, unfixed]

**Known and verified by reading the card and the emitted node.** The print is
「**自分のステージにいる**…のメンバーは、**相手の効果によっては**ウェイトしない」; the parse emits
`target: "opponent"` — the exact inverse. 「相手の効果」 modifies the verb, not the target.

**Not done.** Not fixed, and the blast radius is unmeasured. Two parser rules disagree on
自分 vs 相手 ordering: `_set_action_023` (`parser.py:1986-2014`) tests 相手 first,
`_set_restriction_fields` (`parser.py:10589-10594`) orders 自分 first.

**Next step.** Fix the ordering, then regenerate and diff `abilities.json` to count affected
abilities before trusting it.

---

## 6. `PL!N-bp7-024-N` / 13 phantom card entries  [DATA, recorded, not changed]

13 `card_no`s have text fragments spliced in (`*PRproteinbar` ×12, `LL-PR-007-PRLoveLive!Days`).
**All 13 are byte-exact duplicates** of clean keys. Purely additive; no unique printing is
unreachable. One test I wrote had been reaching for a phantom — fixed to use the clean key.

**Next step.** One deliberate corpus pass: remove 13 keys and regenerate
`abilities.json` / `abilities.bin` / `cards_gen.rs`. Not a side effect of an audit.

---

## 7. The inequality scan — how to finish it  [PARTIAL]

51 raw hits. Distinguish before touching:
* `>=`/`>` on a **printed positive amount** → genuine gap, fix it. **Done for 6.**
* `>` inside `assert!(!... > 0)` → a negation, correct as written. **False positive seen.**
* a **scaling** amount (per-member, per-unit, stacking) → correct. **Do not touch**
  `sibling_abilities_on_one_card_interact_test.rs`, `mymai_tonight_test.rs`.

Unverified remaining: `debut_and_live_start_blade_grant_test.rs:39,43`,
`modifier_layer_characterization_test.rs:37`, and the 12 mixed sites in
`real_card_phase_walkthrough_and_ability_suite_test.rs`. **Read each card's printed amount
first** — two of the six fixes I made were correct, and every false positive came from not
doing that.

---

## 8. `PL!-bp5-333-P＋` 統堂英玲奈 — コスト9以下 bound  [OPEN, harness-blocked]

The 登場 is driven by **no** test; all 21 tree references use her as a fixture with her
orientation set by hand. The chained-optional decline path is now covered and passing. The
「コスト9以下」 bound is not, because the debut's `pay_optional_cost` gate is an **options-less**
`SelectTarget`: `select_option(0)` declines and `select_option(1)` panics in
`helpers/choices.rs:39`.

**Next step.** Find how other tests accept an options-less `pay_optional_cost` gate — several
exist in the tree. That is the whole blocker.
---

## Q172 first half — RESOLVED, and it is COVERED. My test is the suspect.

Followed my own handover note (read `kasumi_bp5_002_heart_greater_test.rs` before touching
anything) and it settles the question — in the engine's favour.

`PL!N-bp5-002-R` 中須かすみ prints:

> 常時：自分か相手のステージの中で、このメンバーがほかのすべてのメンバーより多くのハートを
> 持つ**かぎり**、ライブの合計スコアを＋１する。

and `kasumi_lower_than_opponent_no_gain`
(`tests/test_modules/effects/score/live_total/kasumi_bp5_002_heart_greater_test.rs:44-55`) does:

```rust
let high = game.new_id("PL!N-bp5-002-R");
game.state.player2.stage.stage = [high, -1, -1];
game.state.mods.add_heart_modifier(high, HeartColor::Heart03, 2);   // ← +2 ability hearts
game.state.recalculate_constants();
assert_eq!(game.state.mods.p1_constant_total_score_bonus, 0, ...);
```

Without the modifier the two 未来 tie, and `kasumi_tie_with_opponent_no_gain` (line 32) proves a
tie grants nothing. The `+2` then **flips that tie into a loss**. So the engine demonstrably
counts ability-gained hearts in the comparison — which is exactly Q172's
「はい、能力によって得たハートも含みます。」, already pinned by a passing test on a different card.

**So my `q172_ability_gained_hearts_are_included_in_the_total` — the `#[ignore]`d UNRESOLVED one
— is the likely defect, not the engine.** It asserts against `performance_snapshots[0].live.score`
after a full live, whereas the working proof reads `mods.p1_constant_total_score_bonus` straight
after `recalculate_constants()`. My test went through a phase pipeline the condition may not
observe the same way. **It should be re-specified against the modifier, not treated as an engine
question** — or deleted, since the behaviour is now proven elsewhere.

### A thing worth reading twice in that same file

Lines 69-86 carry a retraction someone already wrote in place:

> This test previously staged a second `PL!N-bp5-002-R` as the higher own-side member and
> asserted `bonus == 0 || bonus == 1` "to document as permissive". That was wrong twice over:
> the disjunction is over the same variable (true for any outcome) and the fixture made the
> "higher" member a 未来 herself, who is then the highest on the board and legitimately grants
> her own +1. **The engine was right throughout**; the +1 belonged to the second 未来, not to
> the one under test.

That is the same failure mode as mine — a permissive assertion and an identity-confused fixture
producing a conclusion about the engine that was actually about the test. It is worth knowing
that this file has already been through it and fixed it properly, including replacing a
different-printing fixture with a non-未来 one specifically so the bonus is attributable.
---

## Handover item 8 — BLOCKER FOUND, one difference to chase

I said the コスト9以下 bound was blocked because I couldn't accept the `pay_optional_cost` gate.
Searching for how other tests do it, the convention is unambiguous and there is a working
example.

`tests/.../sp_bp5_choice_energy_test.rs:39-55` handles the identical gate:

```rust
match game.get_pending_choice() {
    Choice::SelectTarget { target, allow_skip, .. } => {
        assert_eq!(target, "pay_optional_cost:skip_optional_cost", "prompt 1 is the pay-or-skip cost gate");
        assert!(*allow_skip, "the optional cost must allow skipping");
    }
    other => panic!("prompt 1: expected the cost gate, got {other:?}"),
}
game.select_option(1);
```

So: **assert `target == "pay_optional_cost:skip_optional_cost"`, assert `allow_skip`, then
`select_option(1)` to PAY.** The same shape appears in ~25 files, including
`bp7_q266_natsumi_blade_wait_test.rs:64` and `sd2_nijigasaki_abilities_gap_test.rs:49`.

**Why mine panicked, and the one thing left to check.** My Erena run produced the same
`target` string but with `options: None`, and `select_option(1)` then panicked in
`helpers/choices.rs:39` ("cannot select option on a choice with no options"). The working
example evidently gets `options: Some(…)`. So the gate is *sometimes* built with an explicit
option list and *sometimes* without, and the Erena debut produced the bare form.

**Next step, now one line of investigation:** compare how the `pay_optional_cost` choice is
constructed for `PL!-bp5-333-R` versus `PL!SP-bp5-004-R`. The target string is identical, so
the divergence is upstream of the target label — almost certainly whether the parse gave that
effect an `options`/`self_target` shape the builder turns into a list. Once `options` is `Some`,
`erena_debut_offers_only_the_opponent_member_of_cost_nine_or_less` should un-ignore and pass,
pinning 「コスト9以下」 with a cost-9 member against a cost-10 one.

The `#[ignore]` reason string on that test is now stale — it says the gate "cannot be accepted
from this harness", which is no longer true as a general claim. It should be corrected when the
test is revisited, so nobody else re-derives the search I just did.
---

## Erena: the parse is RIGHT, so the open question is execution, not parsing

Final narrowing on `PL!-bp5-333-R` 統堂英玲奈's 登場.

**Printed**
> {{toujyou.png|登場}}このメンバーをウェイトにしてもよい：相手のステージにいる**コスト9以下**の
> メンバー1人をウェイトにする。

**Parsed** — the chained opponent wait is present and faithful:

```json
{ "text": "相手のステージにいるコスト9以下のメンバー1人をウェイトにする",
  "source": "stage", "cost_limit": 9, "cost_limit_operator": "<=",
  "state_change": "wait", "count": 1, "card_type": "member_card",
  "target": "opponent", "action": "change_state" }
```

`コスト9以下` is `cost_limit: 9` with `<=` — the correct value **and** the correct direction,
which is half the thing this sweep has been auditing. The group clause needs none. The self-wait
gate and the `でもよい` optionality are handled by the executor, not the parse.

**So the remaining question is narrow and it is about execution:** having accepted
「このメンバーをウェイトにしてもよい」, no member-selection prompt opens. Given the parse above,
the candidates to investigate are the two ways the accept branch can fail — either the
`pay_optional_cost` resume doesn't advance into the chained step, or the chained step resolves
without offering a choice (auto-selecting, or finding no candidates because the
`source: "stage"` + `target: "opponent"` combination resolves against the wrong player's stage).

**And a near-miss worth recording.** Chasing this I searched for the card as
`PL!S-bp5-333-R` — which does not exist — and very nearly reported that *my own test constant*
was wrong and had been running against a fallback. The constant is in fact
`PL!-bp5-333-R` and is correct. What caught it was grepping the constant instead of trusting
what I remembered, having just spent a turn establishing that memory is exactly what has been
wrong ten times here.

That is the eleventh near-miss, and it is the cleanest illustration of the whole session's
lesson: every one of the ten wrong conclusions, and this eleventh avoided one, came from
asserting a fact I had not read. The `#[ignore]` on
`erena_debut_offers_only_the_opponent_member_of_cost_nine_or_less` now stands on a measurement
(`pending_choice_summary() == "none"`) plus this parse, and the next person starts with a
correct card number, a correct parse, and four causes already eliminated.
---

## Erena, one more datum: the effect node is a BARE `change_state`

The parse dump for `PL!-bp5-333-R` returned **only** this, with no wrapper:

```json
{ "text": "相手のステージにいるコスト9以下のメンバー1人をウェイトにする",
  "source": "stage", "cost_limit": 9, "cost_limit_operator": "<=",
  "state_change": "wait", "count": 1, "card_type": "member_card",
  "target": "opponent", "action": "change_state" }
```

No `optional`, no `sequential`, no `conditional_on_result`, no `self_target`, and no field
describing the self-wait that is supposed to be the cost. For a printed
「…しても**よい**：A」 the parser has to express three things somewhere — the self-wait **cost**,
the **optionality**, and the chained effect A. The only node visible carries A and its filters.

**What this does and does not tell me.** It does not yet establish a defect, because my dump
prints the first matching ability and truncates, so either (a) the self-wait and optionality
live in a wrapper that the dump did not render, or (b) they live in a **sibling** ability entry,
or (c) they were never extracted. Option (c) would be a genuine parser gap of the same family as
the two real ones found today — an optional cost silently absent, leaving the effect
unconditional — and it would explain the measurement exactly: **no prompt opens, because
nothing is optional.** The chained `change_state` then runs (or not) with no preceding gate.

That last reading is a hypothesis, not a conclusion, and I am flagging it as the single thing
worth checking:

> Does any node in `abilities.json` for this card carry the self-wait cost and its
> `optional`? If not, the printed 「このメンバーをウェイトにしてもよい：」 is parsed as a bare
> mandatory effect and the gate simply does not exist.

A cheap way to answer it without reading Rust: print **every** ability node for
`PL!-bp5-333-R` rather than the first, and check whether any carries `optional`, `self_target`
or a `change_state` aimed at `self`. The same question is worth asking of any other
「…てもよい：A。」 card, since A1 and A2 both turned out to be exactly this class — an optional
or a cost that the parse dropped.
---

## Erena: parse FULLY correct. My "bare change_state" was wrong — I printed `effect`, not `cost`.

Printing **every** ability entry for `PL!-bp5-333-R` (my earlier dump printed only
`a["effect"]`, which is why I saw a bare node and wrongly suspected a dropped gate):

```json
--- ab#0
effect: {"text": "相手のステージにいるコスト9以下のメンバー1人をウェイトにする",
         "source": "stage", "cost_limit": 9, "cost_limit_operator": "<=",
         "state_change": "wait", "count": 1, "card_type": "member_card",
         "target": "opponent", "action": "change_state"}
cost:   {"text": "このメンバーをウェイトにしてもよい",
         "state_change": "wait", "card_type": "member_card",
         "optional": true, "self_cost": true, "type": "change_state"}

--- ab#1  (常時, correct too)
effect: {"condition": {"type": "state_condition", "state": "wait", "self_target": true, ...},
         "duration": "as_long_as", "conditional": true,
         "action": "gain_resource", "resource": "heart", "heart_colors": ["heart05"], "count": 1}
cost:   null
```

**All three things the printed text requires are present and correct:**

| printed | parsed |
|---|---|
| このメンバーをウェイトにしても**よい** | `cost.optional: true` |
| このメンバーをウェイトにする (the cost) | `cost.state_change: "wait"`, `cost.self_cost: true` |
| 相手のステージにいる**コスト9以下**のメンバー1人をウェイトにする | `cost_limit: 9` + `cost_limit_operator: "<="` + `target: "opponent"` + `count: 1` |

So the "dropped optional/cost" hypothesis is **dead** — the gate is present, correctly shaped, and
carrying exactly the right three fields. That kills the reading I recorded two turns ago, and it
was wrong for a reason I should name precisely: **I concluded "the gate is missing" from a dump
that only rendered half the ability.** The absence of a field in my *output* was not evidence of
its absence in the *data*. That is the same failure as `has_blade_heart` and the aliasing — reading
one artifact and inferring — and it is the eleventh retraction, this one caught by going back to
the source when my own two notes disagreed.

**Where the open question now stands, and it is narrow.** The parse is right, the gate is right,
and yet after accepting the cost no member-selection prompt opens. So the fault is in execution,
not parsing: either the optional-cost path for a `self_cost: true` `change_state` cost never
presents its gate, or accepting it does not advance to the chained effect. Both are engine
questions, and neither is a parser question — which reverses where I thought this bug lived.

`erena_debut_offers_only_the_opponent_member_of_cost_nine_or_less` stays `#[ignore]`d, now on a
correct card number, a confirmed-correct parse including the gate, four causes eliminated, and a
measurement (`pending_choice_summary() == "none"`) rather than any of the theories that came before.
---

## The harness fails loudly on a bad card number — verified

`livesuccess_revealed_distinct_name_threshold_test.rs` (the strongest file in the tree) carries
this warning about a pre-existing test:

> its third member id `PL!SP-bp1-014-PR` **DOES NOT EXIST** — the lenient card-id fallback
> substituted a different print, so the "three DISTINCT names" premise was never actually pinned

That describes a **silent substitution**, and if it were still true it would be the most
dangerous thing in the harness: a test naming one card and silently exercising another, with
every assertion still green. Several of my own retractions involved card numbers that do not
exist, so I checked whether that hazard is live.

**It is not.** `tests/helpers/mod.rs:104-107`:

```rust
pub fn card_id(db: &CardDatabase, card_no: &str) -> i16 {
    db.get_card_id(card_no)
        .unwrap_or_else(|| panic!("Card {card_no} not found in database"))
}
```

`game.id()` routes through this (`mod.rs:230` → `consume_id` → `card_id`), so **an unknown card
number panics at the first call**. There is no fallback that substitutes a different print.

**What this means, precisely:**

* Every test that passed this session used a **valid** card number. A wrong one could not have
  produced a false green — it would have panicked.
* All eleven of my retractions were caught; **none slipped through as a silently-wrong test.**
  They surfaced as panics, as failed preconditions, or as a mismatch I chased down — never as a
  green suite proving something false.
* The comment above describes a **historical** helper, or was written against a different
  revision. Either way it is now **stale and misleading**: it warns about a hazard that no
  longer exists, in the file most likely to be read as the model.

**The correct safety statement, for the record.** The protection is not the fallback — it never
existed in current form. It is **`assert_card_identity`** (and `assert_card_in_group`), which this
file calls on **every** fixture. A test that names its cards and asserts their identity cannot be
silently retargeted; a test that does not assert is relying on nothing but the panic.

So the rule that replaces the stale warning: *a card-number mistake fails loudly; a card-
**identity** mistake does not.* `game.id()` protects the first. Only `assert_card_identity`
protects the second — and that is the check whose absence made this file's author distrust its
own premise in the first place.

**Suggested follow-up (small, safe):** update that header comment to say the fallback no longer
exists and that `card_id` panics, so the next reader does not build a theory on a hazard that
has been removed.
