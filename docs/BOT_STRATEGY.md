# Bot Strategy & Winning Tree — Loveca (Love Live! Series Official Card Game)

Merged specification: play research (`BOT_STRATEGY.md`) + executable decision
tree (`BOT_STRATEGY_TREE.md`), formerly separate files. Ground truth: official
rules ver 1.02 (`engine/rules/rules.txt`), card database (`cards/cards.json`,
2526 cards), and the guides cited below. Every claim carries its rule number
or data source. A bot that executes §6 leaf-perfect plays near-optimally by
construction; any behavior not reachable from the tree is a mistake.

Written 2026-08-21, merged + updated 2026-08-22 after the log-driven session
that fixed the arena stall and rewrote the live-set portfolio doctrine (§8).

## Sources

1. **ミウミ@ラブカ — ドローLiella!徹底ガイド**（マリガンとステージ展開）
   <https://note.com/miumi_loveca/n/nef437b0e6d18>
2. **ミウミ@ラブカ — エマランジュ徹底ガイド**（実戦テクニック集）
   <https://note.com/miumi_loveca/n/n5109f59a9944>
3. **ミヤ — ラブカの基本的なプレイについて**（先攻/後攻・スコア帯・ライブ温存）
   <https://note.com/miya_duel/n/n1cefd065b054>
4. **ミウミ@ラブカ — ライブで迷いやすいルールまとめ**
   <https://note.com/miumi_loveca/n/nf0af19204837>
5. **ゴンズ様 — 総合ルールをわかりやすく解説**（チェックタイミング/リフレッシュ）
   <https://note.com/gonzu_sama/n/neb30b8b3e9e7>
6. **ラブカ非公式 Wiki — ライブフェイズ / バトンタッチ**
   <https://wikiwiki.jp/llocardgame/>
7. **公式総合ルール ver 1.02 (PDF)**
   <https://llofficial-cardgame.com/wordpress/wp-content/uploads/2025/03/18104002/loveca_rule_ver102.pdf>
8. **公式ラブカポイントシステム**（環境カード評価）
    <https://llofficial-cardgame.com/lovecapointsystem/>
9. **クリフ — ライブチェック成功率計算ツール**（Binomial 命中率シミュレータ）
    <https://note.com/cliff1811/n/n3b4eba399aa1>
10. **岡崎みどぅー — 環境デッキのターン毎の打点**（先攻/後攻ごとの想定ダメージ）
    <https://note.com/msttakahashi/n/n0cacc1311272>
11. **たつどら — 先攻革命**（先攻のライブ成功時効果・主導権）
    <https://note.com/tatsudora/n/nd5a0b7a7b1ac>
12. **かしわもちチャンめる — マリガン・展開ガイド**
    <https://note.com/kasiwamoti_y/n/n6bb70f5f28bc>
13. **かしわもちチャンめる — バトンタッチ・キープ方針**
    <https://note.com/kasiwamoti_y/n/n390430fd20f4>
14. **グリリバ — ヨール・ブレードハート運用**
    <https://note.com/guririba21/n/n3734ce079186>
15. **ミウミ@ラブカ — 1ヨールあたりのブレードハート命中率 (~73%)**（X 投稿）
    <https://x.com/miumi_loveca/status/2056661387326190061>

---

## 0. BOT GENERATIONS AT A GLANCE (updated 2026-08-27)

| Bot | Main phase | Live set | Fair? | Head-to-head vs v5 |
|---|---|---|---|---|
| v1 | heuristic | heuristic | yes | n/a (early) |
| v2 | MC-ladder scalar eval | MC-ladder | yes | loses ~59% |
| v3 | archetype + clone-eval | archetype | yes | ~parity |
| v4 | hearts fundamentalism | deterministic pass | yes | loses ~43% |
| v5 | v4 delegate | binomial + gamble + junk | yes | baseline (100%) |
| **v6** | **tempo: deploy while useful, else Pass** | **binomial + gamble + junk (reused)** | **yes** | **wins ~76% (217–68 / 300)** |

v6 is the current strongest heuristic bot. It keeps v5's binomial-aware live
set (the part that actually places cards) and replaces only the Main-phase
policy, which was the source of the "do-nothing" turns. See §8.5.

## 1. EXPECTED GAME LENGTH (guide data — re-researched 2026-08-22)

From source 3's tournament match logs (mirapax aggro, shop wins) and sources
1/2, the standard development curve is:

| Turn | Board | Live check | Notes |
|---|---|---|---|
| T1 | cost-4 member (solo) | optional 1点, or skip | aggro Liella can double 2-cost → 1×2点 |
| T2 | baton 4→9 (~cost 9) | **2点 both sides is the standard** | ties common; turn order unchanged |
| T3 | baton 9→11/13 (+extra member) | 3点 | first real separation |
| T4 | baton to 13/15 + second member | 5–6点 multi-life (13梢 constant + Eutopia) | |
| T5 | big center online | closeout: Eutopia + identity ×2 = 6–8点 | most games END here |

- Aggro lists (紫アグロ, blade-heart 52–59枚) can 3-kill by **T3** with a
  2点–3点–5点 line; they lose long games instead.
- Grind mirrors stretch to T7–T8+; anything past T10 means both sides are
  failing checks, not playing a slow archetype.
- **Placement expectation: ~1 success per live phase per side from T2 on.**
  A placement rate of ≲0.33/live-phase (measured for older bots) produces
  T12+ games and is a defect, not an archetype.
- Healthy bot targets: median game length T5–T8, hard band 5–9.

## 2. GAME FACTS THAT DRIVE STRATEGY (rules)

```
WIN  = my success_live_card_zone has ≥3 cards AND opponent's has ≤2   (1.2.1.1)
DRAW = both reach ≥3 simultaneously                                   (1.2.1.2)
```

Everything decomposes into one recursive question: **how do I get cards INTO
that zone, and how do I stop them getting into theirs?**

- Live check: if only one player succeeded, they win regardless of score.
  If both succeeded, higher total score wins; **tie = both win**, but a player
  with 2 success cards already cannot place on a tie (8.4.7.1).
- Every won check places exactly ONE card no matter the margin (8.4.7).
  Score only matters inside a contested comparison — so P(win the comparison)
  is the ONLY quality metric of a live portfolio.
- Turn order: whoever *alone* placed a success card becomes 先攻 next turn
  (8.4.13). Going second lets you see the opponent's live before committing.
- Score comparison adds +1 per スコア+1 icon gained from yell (8.4.2.1).
- Baton touch: playing over a member already on stage reduces the new member's
  cost by the sent member's cost (9.6.2.3.2); sent member must have been on
  stage since a previous turn (Q87).
- Refresh: when the main deck empties it recycles the waitroom (10.2.2.1),
  so life ammunition circulates indefinitely — hoarding ammo has low value,
  tempo has high value.

## 3. ANATOMY OF ONE LIVE CHECK (§8)

The live phase runs: **Set (8.2) → 先攻 Performance (8.3) → 後攻 Performance
→ Victory Determination (8.4)**.

### 3.1 SET PHASE (8.2) — "what goes face-down into my live zone?"

```
SET DECISION (up to 3 hand cards, may set zero; each placed card draws 1)
│
├── 3.1a  NON-LIVE cards set here are NOT wasted: discarded at performance
│          start (8.3.4) BEFORE any check → setting a dead member = a fresh
│          draw, costs only a slot. Legal hand-filtering.
│
├── 3.1b  ⚠ ALL LIVES STAND OR FALL TOGETHER (8.3.15→8.3.16): hearts are
│          allocated IN ZONE ORDER; if ANY life fails, EVERY life in the
│          zone is discarded. One greedy high-score life can zero out two
│          safe ones. Portfolio sizing must budget flip VARIANCE (binomial),
│          not the mean.
│
├── 3.1c  Setting ZERO is a first-class option (concede / 温存) — but note
│          every empty live phase guarantees zero placements, and ammo
│          recycles via refresh; folding is usually worse than swinging.
│          Exception worth keeping: opponent at 2 successes ⇒ must contest.
│
└── 3.1d  Set cards are HIDDEN until your performance (8.2.2 裏向きで).
           The second attacker sets AFTER seeing the first attacker's set
           COUNT (not contents) — an empty opponent zone at that point means
           ANY sole passer places regardless of score (8.4.3.2 free win).
```

### 3.2 PERFORMANCE (8.3) — execution order

| Step | Rule | What happens | Strategic lever |
|---|---|---|---|
| reveal | 8.3.4 | flip all set cards; non-lives → waitroom | filtering happens here |
| ライブ開始時 | 8.3.8, 11.5 | auto abilities trigger (521 cards carry it) | deck building / stage presence |
| yell count | 8.3.10 | sum BLADES of my ACTIVE members only | wait members give no flips |
| yell | 8.3.11 | flip that many deck tops into resolution zone | density of blade-hearts decides hit rate |
| draw icons | 8.3.12.1 | each Draw icon flipped = draw 1 | free card advantage |
| heart pool | 8.3.14 | pool = ALL members' hearts (active AND wait) + flipped blade-hearts | wait members DO contribute hearts |
| allocation | 8.3.15 | per life, in order: satisfy need_heart, subtract used icons | order matters when pool is tight |
| verdict | 8.3.16 | any failure ⇒ whole zone to waitroom | see 3.1b |

**Heart accounting subtleties (rule 2.11.3, 2.1):**

```
├── specific colors Heart01–06: filled only by same-color hearts
├── All icons (icon_all): wildcard → any ONE specific color (8.3.15.1.1)
├── BAll blade-hearts: wildcard → any ONE specific color (deterministic)
├── Heart00 (colorless): fills ONLY grey requirements (rule 2.1.1.2)
├── grey requirement "heart0: N" = TOTAL-COUNT bucket (2.11.3): colorless +
│   leftover specific/wild hearts
└── surplus after success is recorded as 余剰ハート (some cards read it)
```

### 3.3 VICTORY DETERMINATION (8.4)

```
COMPARISON (8.4.2–8.4.7)
├── my score  = Σ scores of lives REMAINING after 3.2 + スコア+1 flips
├── neither side has lives        → nothing happens (8.4.6.1)
├── only I have lives             → I WIN regardless of score (8.4.3.2)
├── both have lives               → higher total wins
├── equal totals                  → BOTH win (8.4.6.2), BUT:
│   ├── a player at 2 successes does NOT place on a tie (8.4.7.1)
│   │     → tie ≈ loss when I'm at 2
│   ├── tie at 2-2 = BOTH reach 3 = DRAW GAME (1.2.1.2)
│   └── tie places ⇒ turn order UNCHANGED (8.4.13)
└── sole placer becomes 先攻 next turn (8.4.13)
    → winning a check buys initiative; conceding hands it over
```

**Score bands (all 291 lives in cards.json):**

| score | min hearts | median | max | notes |
|---|---|---|---|---|
| 0–1 | 2 | 2–3 | 4 | tempo/free wins |
| 2 | 5 | 5 | 8 | the T2 standard |
| 3 | 6 | 7 | 9 | the T3 standard |
| 4 | 8 | 10 | 12 | |
| 5 | 10 | 12 | 14 | the T4 standard |
| 6 | 14 | 14 | 15 | jump band |
| 7 | 12 | 16 | 18 | |
| 8 | 17 | 19 | 21 | endgame multi-life territory |
| 9 | 20 | 21 | 21 | |

Median ≈ `2·score + 1..2` — the opponent's ceiling is estimable from their
public board.

## 4. WHAT FEEDS THE CHECK (normal phases, §7)

```
RESOURCES (each normal phase: Active → Energy → Draw → Main, 7.3.3)
├── Active (7.4): all wait energy AND wait MEMBERS reactivate
├── Energy (7.5): +1 active energy; regenerates each turn (7.4.1) — holding
│   is compounding, not waste; the cost of holding is board not grown
├── Draw (7.6): +1 card
└── Main (7.7):
    ├── play a member (7.7.2.2): net cost = own cost − baton-sent member's
    │   cost; Q70 area played-to can't receive another member (Q71 exception)
    └── activation ability 起動 (7.7.2.1), 297 cards — budget energy for
        out-of-main-phase ability costs BEFORE spending down

DERIVED QUANTITIES (the real scoreboard)
    hearts(t)  = Σ base_heart over ALL my stage members     (feeds 3.2)
    flips(t)   = Σ blade over ACTIVE members                (feeds 3.2+3.3)
    hits(t)    ~ Binomial(flips, blade-heart density of MY deck)
                 ← hits are a DISTRIBUTION, not the mean (own decklist =
                   fair info; calibration showed mean-sized portfolios fail
                   ~half the time)
    ceiling(t) ≈ largest s with median_hearts(s) ≤ hearts + hits
```

**ENERGY DOCTRINE:** higher-cost members are simply better (more base hearts
→ higher ceiling). Play the largest baton-discounted member affordable while
reserving energy for known live-phase ability costs; bank only when no
upgrade is reachable; never pass a discounted upgrade just to "save".

**Card-mechanic inventory (cards.json):**

| trigger icon | cards | fires when | strategic meaning |
|---|---|---|---|
| 登場 (debut) | 629 | member enters an area | play members partly FOR these effects |
| ライブ開始時 | 521 | my performance starts (only if a life was set, 11.5.2.1) | free value every contested turn |
| icon_blade | 632 | flipped during yell | the 3.2/3.3 currency |
| icon_energy | 373 | flipped | energy acceleration |
| 常時 (constant) | 300 | while on stage | board-quality multiplier |
| 起動 (activation) | 297 | main phase, costs energy | repeatable engine pieces |
| ターン1回/2回 | 270 | limit keywords | cap ability spam |
| ライブ成功時 | 237 | my live succeeds (8.4.4) | payoff for contesting |
| 自動 (auto) | 140 | various | |
| center/leftside | 65/18 | position-gated | WHERE a member sits matters (NOT yet used by any bot eval) |

## 5. GRAND STRATEGY S1–S7 (deck-agnostic synthesis)

- **S1 Cost curve**: maximize stage total-cost growth per turn while spending
  little; prefer batons where the sent member covers most of the new cost
  (guides' 「4→9」 swaps). Curve: T1=4, T2≈9, T3≈13, T4 aim big center.
- **S2 Predictable bands**: before setting lives, estimate BOTH sides' max
  score from public info (`2·score+1..2`); don't set a life you will almost
  certainly lose — unless you must contest.
- **S3 Live discipline (温存)**: don't waste low-score lives into clearly
  losing comparisons; hold for multi-card high-score turns. Conversely steal
  tempo when the opponent cannot win the check anyway (free win 8.4.3.2).
- **S4 Concede vs 千秋楽 urgency**: if you can't outscore them this turn,
  skipping is often right — BUT at opponent match point (they have 2) failing
  to contest loses outright; a tie does not save you (8.4.7.1).
- **S5 Blade/yell economics**: yell = active blades; know your deck's
  blade-heart density; avoid lives needing many flips at low density
  (source 3's ~1/2 hit-rate example).
- **S6 Tempo / turn order**: winning alone makes you 先攻 (worth eval value);
  going second on key turns lets you respond (RPS winner picks second).
- **S7 Mulligan = curve completion**: keep pieces completing T2/T3
  (cost-7, cost-13, retrieval); mulligan unplayable hands aggressively.

## 6. PER-PHASE DECISION TREE (bot-executable form)

```
MAIN PHASE (repeat until pass):
├── M1. Play the member raising ceiling(t) per net energy best (baton-
│       discounted; reserve for live-phase ability costs).
├── M2. Use activation abilities whose effect raises P(win next check)
│       more than the energy spent.
└── M3. Bank only when no upgrade is reachable (energy regenerates).

LIVE SET PHASE (one decision, hidden):
├── L1. Estimate both ceilings (formula above; public zones only).
├── L2. Opponent at 2 successes? → CONTEST: max-P(success) portfolio;
│       tie counts as loss (8.4.7.1).
├── L2b. Second attacker + opponent zone empty → cheapest passer places
│       FREE (8.4.3.2) — never fold here.
├── L3. Projected comparison vs their ceiling: clearly lost → consider
│       温存; tied → minimal portfolio that WINS or TIES (tie good ≤1,
│       bad ≥2); won → cheapest winning portfolio + junk filter (3.1a).
└── L4. Portfolio construction:
        1. candidate lives = hand lives fitting projected pool
        2. rank by TOTAL SCORE among those clearing the pass-probability
           stance floor (binomial, not the mean!)
        3. NEVER include a failing life (all-or-nothing, 8.3.16)
        4. spare slots + ahead → dump dead non-lives (hand filter)

PERFORMANCE/VICTORY: automatic — log outcomes to update opponent modeling.
```

## 7. MAPPING TO THE BOTS

| Principle | Implementation |
|---|---|
| S1/S2 | v2 `evaluate_state` stage-cost terms; v5 `estimate_opp_score` (median-table ceiling from public board) |
| S3/S4 | v5 stance floors: default 0.55, self-closeout 0.65, opponent-match-point 0.40 gamble |
| S5 | v4 `flip_stats` (blades, deck density) + binomial pass filter in v5 `best_portfolio` |
| S6 | v2 first-attacker bonus; RPS winner takes second attacker (arena harness) |
| S7 | v4/v5 mulligan (keep lives ≤3, dump expensive non-lives) |
| L4 | v5 exhaustive subset search ranked `(score desc, count asc)` behind `binom_ge(blades, relied_hits, density) ≥ floor` |
| Search | v2 UCB1 determinized rollouts (`ismcts.rs`); v4/v5 one-ply clone-eval with no-op/hand-reserve breakers |
| **v6 Main** | tempo deploy-while-useful policy (§8.5): `Pass` ranked below any value>0 action, hearts+blades dev, baton priority, anti-clog |

Version history:

- **v1** `strategy.rs` — original heuristic bot (experimental).
- **v2** `strategy_v2.rs` — MC-ladder live-set + hand-tuned scalar eval;
  still the strongest *heuristic* baseline.
- **v3** `strategy_v3.rs` — archetype plans + clone-eval main phase;
  parity with v2 head-to-head (see `V3_WHY_IT_SUCKS.md` for its autopsy).
- **v4** `strategy_v4.rs` — "success-zone fundamentalism": hearts-based
  development, deterministic-pass portfolios, junk filtering.
- **v5** `strategy_v5.rs` — v4 execution + comparison awareness: binomial
  stance floors, score-max portfolios, free-win rule, committed gamble,
  starvation hand-filtering.
- **v6** `strategy_v6.rs` — **current strongest heuristic** (2026-08-27).
  Keeps v5's binomial-aware live set verbatim (the component that actually
  places success cards) and replaces only the Main-phase policy. Diagnosis of
  the "do-nothing" turns: `choose_action_v4` (v5's Main delegate) scored
  `Pass` (end Main phase) at ~0 and selected with a STRICT `>`, so any member
  play that did not immediately raise `passable`/`ammo`/`stage` counts tied at
  ≈0 and lost to an earlier `Pass` in the actions list — the bot ended Main
  having deployed nothing even with affordable members on board. Fix: rank
  `Pass` strictly below any action with value > 0, but ALSO below a
  value-0 member is allowed to lose (anti-clog: a 0-contribution waiting member
  must not fill the 5-slot stage). Result (300-game arena, untraced, mirrored
  `5CP3Z idou` deck): **v6 beats v5 217–68 (~76%)**, live-set fold rate
  7.6%→0.8%, median game length ~5.8 turns. v6 still uses ONLY fair
  information (own hand/deck + opponent public board via `estimate_opp_score`).
   Do-nothing root cause and the exact mechanism are documented in §8.5.
- **v7** `strategy_v7.rs` — **v6 alias; does NOT improve on v6** (2026-08-27).
  Two intuitive improvements were attempted and both regressed against v6 in
  cross-deck arena:
  1. *Aggressive match-point live set* (lower binomial floors + mandatory 1-life
     attempt at match point) — on `fade deck` v7 fell to **70–100 vs v6**
     (v6 was 142–39 vs v5), stalls rising 17→24. Every all-or-nothing failure
     *wastes a life* without placing; lives are not infinitely recycled mid-game
     (refresh only fires on deck-out, which these games never reach). Forcing
     attempts just depleted ammo → more stalls. Re-confirms the V3_WHY_IT_SUCKS
     correction: "folding is usually worse than swinging" is **false** when ammo
     is effectively finite.
  2. *Color-aware Main development* (bonus for playing members whose `base_heart`
     colors match our own hand lives' `need_heart`) — v7 then lost to v6 on 6 of
     8 decks (liella 58–84, fade 47–81, muse 59–78, aqours 60–68, 5CP3Z 64–79…)
     and only won on hasunosora. Steering development toward *specific* life
     colors trades board power for a false target — what the yell needs is total
     hearts+blades, because flips supply the needed colors stochastically.
  **Conclusion: v6 is at a heuristic plateau.** Marginal term surgery does not
  move win rate, and aggressive variants actively hurt. v7 is therefore kept as a
  v6 alias so it never regresses. The genuine path to >v6 is the *structural*
  fix the doc prescribes: simulation/ISMCTS-backed live-set (and main) decisions
  via the existing `ismcts.rs` + `DeterminizationSampler` + `PublicObservation`
  infrastructure — not more scalar terms (see §8 / §10).

## 8. POST-MORTEM 2026-08-22 — why it looked terrible and what was fixed

### 8.1 The great stall (92% draws)

Every v2-vs-v5 arena run scored **92% draws at avg 2.4 turns** — all 0-0.
Cause chain:

1. Engine `SelectLiveCard` is a **toggle** (`phases.rs`
   `handle_live_card_selection`: present index → deselect, else select).
2. v5's gamble fallback selected one near-miss life but **never confirmed**;
   the next stateless tick saw `desired` still empty and `emit()` deselected
   it → infinite select/deselect oscillation inside `LiveCardSet`.
3. The arena's stuck-counter aborted the game and the scorer called it a draw.

Consequence: **every historical benchmark was garbage** — win rates measured
who stalled less, not who played better. Fixed with the COMMITMENT RULE: any
gamble target joins `desired`, so emit() confirms instead of toggling back.

### 8.2 Pace diagnosis (reading the played games)

After un-stalling, trace analysis (150-game samples, mirrored deck) showed:

- stage-cost curve ON guide (T1≈4, T2≈9, T3≈12, T4≈17) — development fine;
- live-set phases set exactly ONE card whenever non-empty (753/753);
- **63% of live phases conceded empty**, T1–T5 placements ≈ 0;
- placements ran ~0.1/live-phase vs the guide's ~0.5–1 → games died at the
  loop cap mid-game, again scored as fake draws.

Root cause: v4/v5 ranked portfolios "fewest passing lives first" ("zerg")
and treated full-mean yell flips as guaranteed. Since a won check places one
card regardless of margin (8.4.7), minimum-count single low-score lives just
lose comparisons; and mean-sized reliance on flips fails ~half the time
(all-or-nothing 8.3.16).

### 8.3 Doctrine fixes implemented

- `best_portfolio` now maximizes **TOTAL SCORE** among subsets whose group
  pass probability clears a stance floor, using exact
  `P(Binomial(blades, density) ≥ relied_expected_hits)`; only reliance on
  expected-hit wildcards is stochastic (board BAll is deterministic).
- Stance floors calibrated empirically: **0.55** default / **0.40** opponent
  match point / **0.65** own closeout. First attempt (0.75/0.30/0.85) folded
  76% of phases and stretched games past T15 — a portfolio sized near the
  mean sits at P(pass)≈0.5–0.7 by construction, so demanding 0.75+ folds
  nearly every contested turn.
- Free-win rule: second attacker + empty opponent zone → cheapest
  deterministic passer (8.4.3.2), even 0-score.
- Always-contest gamble fallback (ammo recycles via refresh; folding
  guarantees nothing), with the commitment rule above.
- Opponent ceiling estimate (`estimate_opp_score`) from public board via the
  median table, for future 温存 decisions.

Results (untraced 10s arenas, mirrored deck): **v5 beats v2 ~130–99, beats
v4 128–97, crushes random 321–23; avg game length 8.3–9.3 turns; draws
~5%** (from 92%). Within the tree's 5–9 band, though still above the
guides' T5–T7 sweet spot — remaining gap lives in comparison conversion,
not development.

### 8.4 ⚠ NEW CRITICAL ENGINE BUG — logging perturbs gameplay

Running the arena with `--logs`/`--trace` changes game outcomes against the
same seed: plain runs measure avg 8.2–8.5 turns while identical-seed logged
runs measure 13–30; even **random-vs-random diverges** (16.2 vs 31.3 avg
turns over the same 100 games). Pure string/file allocations shift heap
layout, and something in the unsafe pool/bytecode layers
(`core/pool.rs`, `ability/vm.rs`, generated `abilities_gen.rs`) behaves
allocation-layout-dependently. Until root-caused:

- **Benchmark and tune bots UNTRACED only.**
- Traced replays are qualitatively useful but quantitatively distorted.
- This also retroactively taints any analysis session that read traced games
  (including parts of `V3_WHY_IT_SUCKS.md`).

## 8.5 Do-nothing turns — root cause & fix (2026-08-27)

Symptom reported: bots "play many turns of nothing." Two distinct failure
modes were measured in the arena (untraced, 300 games, mirrored `5CP3Z idou`):

1. **Empty Main phases** — a Main phase ending in `Pass` with zero member
   deploys. For v5 this was ~4.3% of Main-phase ends; most were *legitimate*
   (no affordable/useful member), but a measurable slice came from the tie
   bug below.
2. **Live-set folds** — a Live Card Set that sets zero lives (only junk or
   nothing), guaranteeing zero placements that turn. For v5 this was **7.6%**
   of live phases; combined with comparison losses this is the real pace
   killer (it is what stretches games past the guide band — §1).

### 8.5.1 The Main-phase tie bug (primary do-nothing cause)

`strategy_v4::choose_action_v4` (which v5's `choose_action_v5` delegates to)
scores every candidate action and keeps the max with a STRICT `>`. `Pass`
("Pass - End Main Phase") is always present in the actions list (generated
first in `generate_main_phase_actions`) and scores ≈0 when ending the phase
changes no tracked zone. Any member play that does not *immediately* raise
`passable`/`ammo`/`stage` counts (e.g. a member with no `base_heart`, or one
whose blades only matter at the yell flip) also scores ≈0. Because the
comparison is strict and `Pass` is seen first, the bot kept `Pass` and deployed
nothing — despite affordable members being on board. This is the mechanism
behind "several turns of nothing."

### 8.5.2 The fix (v6)

v6 evaluates all actions, finds `best_nonpass` = the highest value among
non-`Pass` actions, then sets `Pass`'s value to `−∞` when `best_nonpass > 0`
(so a useful deploy is always taken), and to `0` otherwise (so a useless
0-contribution waiting member is NOT force-played into the 5-slot stage —
anti-clog). Development is valued in **HEARTS + BLADES** (not cost), because
both feed the yell check (§4): every extra active blade is a fresh Binomial
trial (source 9 / source 15's ~73% per-blade-heart hit rate) that can supply
the hearts a check needs, and baton touches (9.6.2.3.2, source 13) are
prioritized as discounted power-piece upgrades (guide curve 4→9→13, source 1).

Effect (v6 vs v5, 300 games): live-set fold 7.6%→**0.8%**, empty Main phases
shift to "pass only when nothing useful" and v6 wins **217–68 (~76%)**.

### 8.5.3 Fairness of the fix

v6 adds no hidden-information use. Its Main-phase eval reads only our own
hand/stage/energy/deck; its live-set still calls `estimate_opp_score`, which
estimates the opponent ceiling from their **public** board and our own deck
density (both fair per §9). No peek at opponent hand/deck.

## 9. FAIRNESS POLICY

The old bot sampled the opponent's hidden hand/deck from their **actual deck
list**, which a fair player does not have. Now:

- Default (`Bot::new_fair` / `open_decklists == false`): opponent hidden
  cards sampled from an anonymous pool of all Member/Live cards minus what
  public zones reveal. Only our own deck list is used (this includes our
  blade-heart density — fair).
- Open-list mode remains available via `DeterminizationSampler::new` /
  `open_decklists(true)` for research.

Decisions come from `PublicObservation` only; rollouts operate on
determinized states (standard PIMC practice).

## 10. OPEN FIX ORDER (testable via bot_arena, untraced)

1. Root-cause §8.4 allocation-layout sensitivity (poisons all measurement).
2. Tie-value table in L3/L4: explicit tie scoring (win ≤1 success, loss at 2,
   suicide at 2-2) instead of floor approximations.
3. ~~M2: main-phase eval undervalues 起動 engines~~ — v6 now values any
   value>0 main action (including UseAbility/baton) above Pass, so activation
   engines are no longer pruned by acquisition deltas. (Do-nothing Main-phase
   bug fixed in §8.5; v6 beats v5 ~76%.)
4. Position keywords (センター/左サイド) ignored entirely by every eval.
5. M1: explicit baton-efficiency term (net energy per ceiling point).
6. Turn-order planning (sole placer becomes 先攻, 8.4.13) — no eval term yet.

---

## Why V3 Sucks (V3_WHY_IT_SUCKS.md)

# Why v3 still sucks — post-mortem after the log-driven iteration

> ## ⚠ CORRECTION (post-refresh-analysis)
> An earlier version of this document claimed **"ammo exhaustion"** as a
> structural cause (v4/v3 spending 12 deck-lives by turn 5). That diagnosis
> was **wrong**: rule 10.2.2.1 REFRESH cycles the waitroom back into the
> deck whenever the main deck empties, so lives are never permanently lost —
> they circulate indefinitely (deck → hand → live zone → waitroom → deck).
>
> The real reason games ran to 12 turns for 3 placements:
> **only ONE card places per WON check (8.4.7), and winning requires beating
> the opponent's total score (8.4.6)** — passing your hearts is necessary
> but not sufficient. Measured placement rate was ~0.33/turn, i.e. two
> thirds of live phases produced nothing: lost comparisons, all-or-nothing
> escalations at match point, or double-passes. The bottleneck is
> **P(win the comparison)**, which is exactly the dimension the heuristic
> stack cannot see (opponent's portfolio is unknown until set).
>
> This is why v4 — built to ignore scores entirely — dominates random
> (~92%) yet sits at ~43–47% vs v2: execution wins uncontested games;
> comparisons decide contested ones.

State at time of writing: v3 ≈ v2 head-to-head (~47–51% over large samples),
draws fixed (40% → <8%), no more infinite activation loops. All the
session's patches are in. And it still doesn't play better than the version
it was built on. This document is the honest why.

## The five structural causes

### 1. One-ply greedy cannot see sequences — and this game IS sequences

Every v3 decision clones the state, executes ONE action, evaluates the
resulting position, picks the best. But the guides' winning lines are
multi-turn compositions:

    T2: baton 4→9 (sets up hearts)
    T3: baton 9→11 + play extra member (sets up 3点)
    T4: baton to 13/15 + second member → 5–6点 multi-life check

Each individual action on such a line looks MEDIOCRE in isolation (spending
energy, thinning your hand for +4 stage cost) while its value lands two turns
later in a check you were previously guaranteed to lose. A one-ply eval
scores the photo, not the trajectory. It will always prefer three cheap
independent gains over one enabling move — which is exactly why trace
analysis showed v3 playing members without purpose and passing winnable
checks.

**Consequence:** no heuristic weight can fix this. The information "this
action enables a winning line" does not exist in any single resulting state.

### 2. The eval measures a snapshot; states with equal snapshots diverge wildly

`evaluate_state_v2` sums stage_cost, hearts, blades, energy, hand size,
lives-in-hand. Two positions can tie on every term while one has:

- a curve-complete hand (cost 2/5/11 reachable each turn), and the other an
  unplayable pile of cost-15s;
- lives matching the colors ON STAGE versus lives needing colors it can
  never produce;
- blades on active members versus blades sitting in wait.

The color-coverage term patches one case; the session's failures (hand
liquidation, ammo starvation, dead-life hoarding) were each a *different*
case of the same disease: aggregate statistics hide composition. We patched
composition cases reactively — six times — and each patch re-allocated the
blindness somewhere else (the energy penalty literally converted "hoards
energy" into "spams abilities").

### 3. Ten hand-tuned scalars with zero objective

stage_cost=8, heart=12, blade=6, active_energy=3, hand_size=4,
hand_live_card=15, first_attacker=20, +60/+25/+150 desperation, −1000 no-op,
−40 reserve… Nobody ever defined what these multiply INTO. There is no
target like "predicted placement probability by turn N"; there is only
"number went up". This session proved the failure mode empirically: every
new term measured neutral-to-negative because it perturbs a balance whose
correct setting is unknown. Tuning ten interacting weights through a
one-number fitness (win rate) with ±5% sampling noise is not optimization —
it's dice.

### 4. The opponent exists only as a pressure scalar

S2 of our own strategy doc: "estimate the opponent's max score from their
public board before committing lives." The implementation: when opp has 2
successes, multiply my development terms by 1.6. That is not estimation —
it's panic scaling. v3 never asks "can they outscore my portfolio THIS
turn?", which is THE decision input for contest/concede (tree node 1.4).
Result: v3 contests lost checks and concedes winnable ones at rates a human
beginner would beat.

### 5. Placements happen in the live phase, so live-phase quality caps strength

All three success cards enter via won checks. Boards are fine now (trace
data: both bots track the guide curve within 10%). Yet placements run at
~0.33/turn in bad games because:

- portfolios minimize risk instead of maximizing placed-score expectation
  (v2's ladder ranks pass-probability first);
- life ammunition (12 per deck) is invisible to main-phase choices — v3
  spent its arsenal by turn 4 in game 6 and could not contest for 8 turns;
- retrieval engines exist in the card pool but acquisition deltas valued
  them below a single stage upgrade until the hand was already empty.

## What the fixes taught us (method summary)

Every successful change this session came from reading transcripts:
flip-buffer, no-op breakers, hand-reserve, free-win fallback, ammo rules.
Every failure came from predicting behavior without them. But even the
successful fixes plateaued at parity, because items 1–4 above are not bugs —
they are the architecture.

## The only two moves left that change strength structurally

1. **Simulation-backed decisions.** Clone → execute candidate → let the
   ENGINE resolve (all 900+ abilities handled correctly by construction) →
   drive N turns forward with both sides playing guide-heuristics → score
   outcomes by success-zone delta. Replaces term-guessing with measurement.
   Live-set first (highest leverage, bounded depth), then main phase.

2. **ISMCTS with the current eval as leaf/guide.** The infrastructure
   (`ismcts.rs`, `DeterminizationSampler`) already exists and already
   implements fair-information determinization. Search directly solves
   cause #1 and subsumes #3 (no weights needed except leaf eval), and
   determinized rollouts give #4 for free (opponent plays out their hidden
   info samples).

Between them, neither requires a single new hand-tuned scalar.
