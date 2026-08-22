# Untested Abilities Batch 7 — plan & edge-case matrix

Goal: cover the `depth=none` inventory gaps with assertions derived strictly from
the printed ability text (`cards/abilities.json`) plus rulings in
`cards/qa_data.json` and `engine/rules/rules.txt`. One ability per section,
implemented and run one at a time. Test file:
`engine/tests/test_modules/untested_abilities_batch7_test.rs`.

Support data (verified against `cards/cards.json`):

| card_no | name | relevant facts |
|---|---|---|
| PL!-bp5-008-R | 小泉花陽 | member cost 13, 常時 heart03×2 while own success-zone SCORE total ≥6 |
| PL!S-pb1-009-R | 黒澤ルビィ | member cost 11, 常時 blade×3 while COMBINED success-zone CARD count ≥3 |
| PL!S-bp5-008-R | 小原鞠莉 | member cost 9, 常時 live-total +1 while opponent surplus hearts ≥2 |
| PL!N-bp4-009-R | 天王寺璃奈 | member cost 13, ライブ開始時 strict `<` cost-total compare |
| PL!SP-bp4-001-R | 澁谷かのん | member cost 4, 登場 Liella-only AND energy≥7 → energy-deck→zone WAIT |
| PL!S-pb1-007-R | 国木田花丸 | member cost 9, ライブ成功時 yell-revealed live → energy-deck→zone WAIT |
| PL!S-bp3-008-R | 小原鞠莉 | member cost 4, 起動 self→waitroom : fetch live; Aqours score≥6 → activate 4 |
| PL!N-bp5-016-N | 朝香果林 | member cost 2, ライブ成功時 draw1 then discard1 |
| PL!N-sd1-005-SD | 宮下愛 | member cost 11, 起動 ターン1回 discard2 : retrieve 虹ヶ咲 member |

Lives: START:DASH!! (PL!-sd1-019-SD, score 1) / これからのSomeday (PL!-sd1-021-SD,
score 3) / Sing！Shine！Smile！ (PL!SP-bp1-027-L, score 6) / Next SPARKLING!!
(PL!S-pb1-023-L, Aqours score 9) / Butterfly (PL!N-bp1-028-L, 虹ヶ咲 score 5) /
勇気はどこに? (PL!S-PR-024-PR, Aqours score 5) / 僕らのLIVE 君とのLIFE
(PL!-bp3-019-L, score 0). Hearts bag: PL!S-sd1-001-SD (base hearts 3+2+2 = 7,
Aqours, cost 17).

## A1 PL!-bp5-008-R 小泉花陽 — score-SUM threshold constant

Parser: condition comparison_condition location=success_live_card_zone
comparison_type=score aggregate=total >=6; effect gain_resource heart03×2,
duration as_long_as.

| # | edge case | expectation |
|---|---|---|
| 1 | own zone total 3 | modifier 0 |
| 2 | own zone total exactly 6 (3+3) | +2 heart03 (>= boundary inclusive) |
| 3 | own zone total 9 (single card) | still exactly +2 (value fixed, does not scale) |
| 4 | bonus active then zone cleared | modifier returns to 0 (as_long_as is dynamic) |
| 5 | opponent zone total 9, own 0 | 0 (自分の… ignores opponent) |
| 6 | six score-0 lives in own zone (card count 6, score sum 0) | 0 (aggregate=score, NOT card count) |
| 7 | Hanayo not on stage | 0 regardless of zone |
| 8 | two copies of Hanayo on stage | each copy independently +2 |
| 9 | other heart colors stay untouched | heart01 modifier stays 0 |

## A2 PL!S-pb1-009-R 黒澤ルビィ — COMBINED card-count constant

Contrast pair with A1: card count across BOTH players, not score sum.

| # | edge case | expectation |
|---|---|---|
| 1 | both zones empty | 0 |
| 2 | 1 own + 1 opponent = 2 | 0 (< 3) |
| 3 | 2 own + 1 opponent = 3 | +3 blade |
| 4 | own 0 + opponent 4 | +3 blade — own zone alone may be empty |
| 5 | drop back under 3 | modifier removed |
| 6 | score-0 lives count toward the count | they do (unlike A1) |
| 7 | two Ruby copies on stage | each +3 |

## A3 PL!S-bp5-008-R 小原鞠莉 — opponent surplus-hearts live-total constant

Surplus definition per Q142: stage base hearts (+blade hearts) exceeding the
live cards' need hearts. Engine fallback path computes exactly this outside a
live; asserted via `mods.p1_constant_total_score_bonus`.

| # | edge case | expectation |
|---|---|---|
| 1 | opponent stage empty | bonus 0 |
| 2 | opponent stages 7-heart member, no live cards set | surplus 7 ≥ 2 → our live TOTAL +1 |
| 3 | surplus lands exactly on 2 | +1 (boundary) |
| 4 | opponent hearts fully consumed by need (surplus 0) | 0 |
| 5 | bonus is the player live-TOTAL accumulator, NOT per-card score | get_score_modifier(mari) == 0 |
| 6 | opponent member leaves | bonus removed |

## A4 PL!N-bp4-009-R 天王寺璃奈 — ライブ開始時 strict-lower cost total

| # | edge case | expectation |
|---|---|---|
| 1 | own 13 < opp 17 | draw 2, then choose exactly 1 hand card → deck TOP; hand net +1 |
| 2 | chosen card really sits at deck index 0 (top) | main_deck.front() == chosen |
| 3 | equal totals 13 vs 13 | no fire (「低い場合」strict <) |
| 4 | own higher than opponent | no fire |
| 5 | both stages empty (0 vs 0) | no fire (equal) |
| 6 | own stage empty (0) vs opponent any | fires |
| 7 | put-back choice is mandatory | allow_skip == false |
| 8 | totals sum ALL stage areas both sides, not just center | covered implicitly by multi-member setups |

Deck must be pre-filled (helpers::fill_decks) so Rule 10.2.2.1 refresh does not
consume the waitroom mid-draw (lesson from the sumire double-baton bug).

## A5 PL!SP-bp4-001-R 澁谷かのん — 登場 Liella-only AND energy ≥7

| # | edge case | expectation |
|---|---|---|
| 1 | kanon + Liella! friend, 7 active energy, deck stocked | +1 energy card, ACTIVE count unchanged (WAIT state), deck −1 |
| 2 | μ's member also on stage | blocked (『Liella!』のみ) |
| 3 | exactly 6 energy | blocked (boundary) |
| 4 | mixed active/wait totalling 7 | passes — text counts energy cards, not active ones |
| 5 | condition met but energy deck EMPTY | graceful no-op, no crash |
| 6 | kanon alone on stage (trivially all-Liella!) | works |

## A6 PL!S-pb1-007-R 国木田花丸 — yell-revealed live ライブ成功時

| # | edge case | expectation |
|---|---|---|
| 1 | revealed_cards contains a live card | +1 energy, WAIT state |
| 2 | only member cards revealed | nothing |
| 3 | revealed empty | nothing |
| 4 | two lives revealed | STILL exactly 1 energy (count fixed) |
| 5 | live + members mixed | works |
| 6 | energy deck empty | graceful no-op |

## A7 PL!S-bp3-008-R 小原鞠莉 — 起動 self-to-waitroom fetch

Rulings: Q123 — usable even with NO live in the waitroom (cost still paid);
when ≥1 live exists, adding one is MANDATORY.

| # | edge case | expectation |
|---|---|---|
| 1 | fetch Aqours score 9 (2 active / 4 wait energies) | Mari to waitroom, area emptied (Q79), live to hand, all 6 active |
| 2 | fetch 虹ヶ咲 score 5 | no activation (wrong group) |
| 3 | fetch Aqours score 5 | no activation (score < 6) |
| 4 | waitroom has no live at all (only members) | activation legal, cost paid, nothing fetched, no crash (Q123) |
| 5 | live present | selection choice is MANDATORY (allow_skip false, Q123) |
| 6 | choice lists only live cards | member cards in waitroom are filtered out |
| 7 | fewer waiting energies than 4 (2 active / 1 wait) | activates the 1 that exists (partial resolution) |

## A8 PL!N-bp5-016-N 朝香果林 — ライブ成功時 draw1 discard1

| # | edge case | expectation |
|---|---|---|
| 1 | normal path | net hand 0, chosen card in waitroom, other stays |
| 2 | deck empty, waitroom has cards | Rule 10.2.2.1 refresh feeds the draw → still draws |
| 3 | deck AND waitroom empty | draw no-ops; effect must remain consistent (no crash); document actual discard behavior |
| 4 | discard choice operates on post-draw hand | drawn card is selectable |

## A9 PL!N-sd1-005-SD 宮下愛 — 起動 ターン1回 discard2 : retrieve 虹ヶ咲

| # | edge case | expectation |
|---|---|---|
| 1 | happy path | 2 exact hand cards to waitroom, 虹ヶ咲 member to hand |
| 2 | μ's member also in waitroom | offered choices exclude it (group filter) |
| 3 | second activation same turn | rejected (use_limit 1) — Q58: per copy; Q59: re-deployed copy is fresh |
| 4 | waitroom lacks any 虹ヶ咲 member | either clean rejection or cost-paid fizzle per Q154-style partial resolution — assert consistent, no crash |
| 5 | hand has fewer than 2 cards | activation impossible without paying cost → error, state unchanged |

## Process

For each section: implement tests → `cargo test --test run_all
untested_abilities_batch7_test::<name> -- --nocapture` with RUST_LOG=debug →
diagnose failures against the TEXT (fix engine when the engine contradicts the
text/rulings; fix test when the setup misrepresents the scenario) → next.
Finally rerun full suite + `python cards/test_inventory.py --check`.
