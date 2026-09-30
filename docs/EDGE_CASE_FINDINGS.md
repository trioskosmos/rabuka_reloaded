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

**S8. `PL!SP-pb4-026-L` Wish Song — only Liella cards staged, so the group clause never binds.**
「名前が異なる『Liella!』のメンバーカードが5枚以上」 with 5 distinct Liella and 4, never a
non-Liella on stage and never duplicates mixed with distinct ones. Fix: 5 cards including a
second copy of one print → distinct 4 → +0; plus a non-Liella on stage.

**S9. `PL!S-bp7-023-L` 夜空はなんでも知ってるの？ — the 『Aqours』メンバーが2人以上 gate is never varied.**
Every fixture stages two Aqours, so a bug ignoring the group clause is undetectable. Fix: same
energy split with one Aqours and one outsider → +0.

**S10. `PL!HS-bp1-022-L` AWOKE — 「10枚以上」 staged at 2 and ~9, never at 10 or 9.**
Fix: a deck of exactly 10 `PL!HS-PR-020-PR`, and of 9.

**S11. `PL!S-bp5-008-R` — 「相手の余剰ハートが2つ以上」 staged at 7, 2 and 0; never 1.**
So `> 1` mis-parsed as `>= 1` passes, and so does the reverse. Fix: a live needing exactly one
less than the stage provides.

**S12. `PL!SP-bp2-050-L` Jellyfish — the formation change is never declined.**
「フォーメーションチェンジしても**よい**」 and every test accepts, so a defect making it
mandatory passes. Fix: 2× `PL!SP-bp1-014-N`, decline, assert the stage array is unchanged.

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

## An engine question found while auditing (not a test gap)

`Card::has_blade_heart()` (`engine/src/core/card.rs:4311-4317`) returns
`blade_heart.is_some() || special_heart…is_some_and(|sh| !sh.hearts.is_empty())`, and a **live
card's printed heart icons live in `special_heart`** — so the predicate is `true` for every
live. 49 cards print 「ブレードハートを持たない」; 45 pair it with メンバーカード so a live is
excluded regardless, but the 4 that do not are reachable — notably `PL!S-bp3-019-L` MIRACLE WAVE
(counts over everything yelled) and `PL!HS-bp6-027-L` 月夜見海月. Both readings of the printed
text are defensible, so this needs a **rules decision**, then a test. Left unchanged.
See `PARSER_CONSOLIDATION_SWEEP.md` for the full note.
