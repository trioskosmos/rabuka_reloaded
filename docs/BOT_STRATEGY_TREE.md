# The Winning Tree — from "3 cards in the Success Live zone" down to every leaf decision

Ground truth: official rules ver 1.02 (`engine/rules/rules.txt`), card database
(`cards/cards.json`, 2526 cards), and the play guides cited in
`BOT_STRATEGY.md`. Every claim carries its rule number or data source.

This document is the specification for bot play. A bot that executes this tree
leaf-perfect plays near-optimally by construction; any behavior not reachable
from this tree is a mistake.

---

## 0. THE WIN CONDITION (rule 1.2)

```
WIN  = my success_live_card_zone has ≥3 cards AND opponent's has ≤2   (1.2.1.1)
DRAW = both reach ≥3 simultaneously                                   (1.2.1.2)
```

Everything below is one recursive question: **how do I get cards INTO that
zone, and how do I stop them getting into theirs?**

```
GOAL: place 3 success cards
│
├── A. I place a card ONLY at the Live Victory Determination step (8.4.7):
│      a live check must be WON, then I pick ONE card from my live zone
│      └── so the game decomposes into: win 3 checks before they win 3
│          (with the tie/cap subtleties of 8.4.6/8.4.7.1)
│
├── B. Checks happen exactly once per turn cycle (live phase, §8)
│      └── a full turn = 先攻 normal phase → 後攻 normal phase → live phase
│          → expected game length ≈ 5–9 live phases (data: games end T5–T9)
│
└── C. Therefore EVERY decision is worth exactly its effect on:
        P(win this coming check) × value(placing now)
        + value(resources banked for later checks)
        − P(losing future checks caused by this action)
    There is no other scorecard. Stage cost, blades, energy, hand size are
    all intermediate variables serving A.
```

---

## 1. ANATOMY OF ONE LIVE CHECK (§8, in execution order)

The live phase runs: **Set (8.2) → 先攻 Performance (8.3) → 後攻 Performance
→ Victory Determination (8.4)**. Each performance belongs to one player.

### 1.1 SET PHASE (8.2) — "what goes face-down into my live zone?"

```
SET DECISION (up to 3 hand cards, may set zero; each placed card draws 1)
│
├── 1.1a  NON-LIVE cards set here are NOT wasted:
│          at performance start they are simply discarded to waitroom
│          (8.3.4) BEFORE any check happens
│          → setting a dead member = convert it into a fresh draw,
│            costs only a slot. Legal hand-filtering.
│
├── 1.1b  ⚠ ALL LIVES STAND OR FALL TOGETHER (8.3.15→8.3.16):
│          hearts are allocated to lives IN ZONE ORDER; if ANY life's
│          requirement can't be met from what remains, EVERY life in the
│          zone is discarded. One greedy high-score life can zero out
│          two safe ones.
│          → the set decision is a PORTFOLIO decision: total requirement
│            must fit inside the heart pool with order-aware allocation.
│
├── 1.1c  Setting ZERO is a first-class option (concede / 温存):
│          guides S3/S4 — don't throw a life into a comparison you'll
│          lose anyway; hold ammo for a multi-life high-score turn.
│          Exception: opponent at 2 successes ⇒ must contest (S4).
│
└── 1.1d  What you set is HIDDEN until your performance (8.2.2 裏向きで).
           The second attacker sets AFTER seeing the first attacker's
           development but still NOT their set cards.
```

### 1.2 PERFORMANCE (8.3) — "does my portfolio pass?"

Execution order, per rule:

| Step | Rule | What happens | Strategic lever |
|---|---|---|---|
| reveal | 8.3.4 | flip all set cards; non-lives → waitroom | filtering happens here |
| ライブ開始時 | 8.3.8, 11.5 | auto abilities trigger (521 cards carry it) | deck building / stage presence |
| yell count | 8.3.10 | sum BLADES of my ACTIVE members only | wait members give no flips |
| yell | 8.3.11 | flip that many deck tops into resolution zone | density of blade-hearts decides hit rate |
| draw icons | 8.3.12.1 | each Draw icon flipped = draw 1 | free card advantage |
| heart pool | 8.3.14 | pool = ALL my members' hearts (active AND wait) + flipped blade-hearts | wait members DO contribute hearts |
| allocation | 8.3.15 | per life, in order: satisfy need_heart from pool, subtract used icons | order matters when pool is tight |
| verdict | 8.3.16 | any failure ⇒ whole zone to waitroom | see 1.1b |

**Heart accounting subtleties (rule 2.11.3, 2.1):**

```
HEART POOL ALLOCATION RULES
│
├── specific colors Heart01–06: filled only by same-color hearts
├── All icons (icon_all, index 10): wildcard → any ONE specific color (8.3.15.1.1)
├── BAll blade-hearts (index 7): wildcard → any ONE specific color
├── Heart00 (colorless, index 0): fills ONLY grey requirements;
│   never a specific color (rule 2.1.1.2)
├── grey requirement "heart0: N" = TOTAL-COUNT bucket (2.11.3):
│   satisfied by colorless hearts + leftover specific/wild hearts
└── surplus after success is recorded as 余剰ハート (some cards read it)
```

### 1.3 VICTORY DETERMINATION (8.4) — "who wins the check?"

```
COMPARISON (8.4.2–8.4.7)
│
├── my score  = Σ scores of lives REMAINING in my zone after 1.2
│              + 1 per スコア+1 icon I flipped in yell (8.4.2.1)
├── neither side has lives        → no winner, nothing happens (8.4.6.1)
├── only I have lives             → I win regardless of score (8.4.3.2)
│                                    ← even a 0-score life wins alone!
├── both have lives               → higher total wins
├── equal totals                  → BOTH win (8.4.6.2), BUT:
│   ├── a player already holding 2 success cards does NOT place on a
│   │   tie win (8.4.7.1)  → tie ≈ loss when I'm at 2
│   ├── tie at 2-2 = BOTH reach 3 = DRAW GAME (1.2.1.2)
│   │   → tie ≈ suicide at 2-2, excellent below 2
│   └── tie places for both ⇒ turn order UNCHANGED (8.4.13)
└── sole placer becomes 先攻 next turn (8.4.13)
    → winning a check also buys initiative; conceding hands it over
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

Median ≈ `2·score + 1..2` — this is why the opponent's ceiling is
estimable from their public board.

---

## 2. WHAT FEEDS THE CHECK (normal phases, §7)

```
RESOURCES (each normal phase: Active → Energy → Draw → Main, 7.3.3)
│
├── 2.1 Active phase (7.4): ALL my wait energy re-activates AND all my
│   wait MEMBERS reactivate → last turn's board is fully live again
│
├── 2.2 Energy phase (7.5): +1 active energy
│   ⚠ energy REGENERATES: Active phase re-activates all wait energy (7.4.1),
│   so unspent energy carries over indefinitely — it is never "wasted" by
│   hoarding per se. The actual cost of holding is opportunity: board not
│   grown this turn.
│
├── 2.3 Draw phase (7.6): +1 card
│
├── 2.4 Main phase (7.7): repeatable, any order:
│   ├── play a member from hand (7.7.2.2)
│   │   cost = member cost in energy, MINUS baton touch:
│   │   sending the area's existing member to waitroom offsets cost by
│   │   ITS cost (9.6.2.3.2); sent member must have been on stage since
│   │   a previous turn (Q87)
│   │   Q70: an area played-to this turn can't receive another member
│   │   (but if it empties, it can — Q71)
│   └── play an activation ability (7.7.2.1) — 起動, 297 cards
│       ⚠ energy costs also appear on live-start / live-success and other
│       out-of-main-phase abilities — budget for those BEFORE spending down
│
└── derived quantities (the real scoreboard):
    hearts(t)  = Σ base_heart over ALL my stage members     (feeds 1.2)
    flips(t)   = Σ blade over ACTIVE members only            (feeds 1.2+1.3)
    hits(t)    ≈ flips × blade-heart density of MY deck      (own list = fair info)
    ceiling(t) ≈ largest s with median_hearts(s) ≤ hearts + hits
    energy(t)  = permanent pool; regenerates orientation each turn

ENERGY DOCTRINE (correcting the naive "spend down" advice):
├── higher-cost members on stage are simply better: more base hearts →
│   higher ceiling → harder checks to beat. Cost is a threshold, not a
│   resource burn.
├── so the rule is: play the LARGEST member whose net cost (after baton)
│   you can afford while reserving energy for known live-phase ability
│   costs (ライブ開始時/ライブ成功時 【自動】(コスト) abilities).
├── when the turn plan is life-only (no affordable upgrade), banking energy
│   is correct — it compounds toward next turn's big member.
└── never pass a baton-discounted upgrade just to "save": the sent member's
    hearts leave the pool too, but a bigger replacement usually nets more.
```

**Card-mechanic inventory (cards.json, what abilities actually exist):**

| trigger icon | cards carrying it | fires when | strategic meaning |
|---|---|---|---|
| 登場 (debut) | 629 | member enters an area | play members partly FOR these effects |
| ライブ開始時 | 521 | my performance starts (only if a life was set, 11.5.2.1) | free value every contested turn |
| icon_blade | 632 | flipped during yell | the 1.2/1.3 currency |
| icon_energy | 373 | flipped | energy acceleration |
| 常時 (constant) | 300 | while on stage | board-quality multiplier |
| 起動 (activation) | 297 | main phase, costs energy | repeatable engine pieces |
| ターン1回/2回 | 270 | limit keywords | cap ability spam |
| ライブ成功時 | 237 | my live succeeds (8.4.4) | payoff for contesting |
| 自動 (auto) | 140 | various | |
| center/leftside | 65/18 | position-gated | WHERE a member sits matters |

Position keywords (センター/左サイド, 11.7/11.8) mean the 3 areas are NOT
interchangeable: some engines only run in center. Baton planning must respect
that.

---

## 3. THE PER-PHASE DECISION TREE (bot executable form)

```
MAIN PHASE (repeat until pass):
│
├── M1. Can I place a member that raises ceiling(t) per energy better than
│       every alternative? (ceiling delta ≥ alternatives, baton-discounted)
│       ├── YES → play it (prefer baton where sent-cost covers most of new cost)
│       └── NO ↓
├── M2. Is there an activation ability whose effect raises P(win next check)
│       more than the energy it spends? (engines, retrieval, draw)
│       ├── YES → use it
│       └── NO ↓
├── M3. Energy doctrine: energy regenerates (7.4.1) — holding is compounding,
│       not waste. Play the largest baton-discounted member affordable while
│       reserving for known live-phase ability costs; bank only when no
│       upgrade is reachable. Higher cost on stage = more hearts = better.
│
LIVE SET PHASE (one decision, hidden):
│
├── L1. Estimate both ceilings (formula above; opponent public zones only).
├── L2. Opponent at 2 successes?
│       ├── YES → CONTEST: choose max-P(success) portfolio; only an outright
│       │         win saves you — treat tie as loss (8.4.7.1)
│       └── NO ↓
├── L3. Projected comparison vs their ceiling:
│       ├── clearly lost (≥2 bands below) → SET NOTHING, bank ammo (温存)
│       ├── roughly tied → set the minimal portfolio that WINS or TIES;
│       │   tie is good while I'm ≤1 success, terrible at 2
│       └── clearly won → set cheapest winning portfolio (steal tempo),
│           consider dumping a dead non-live card into spare slots (1.1a)
├── L4. Portfolio construction, in order:
│       1. candidate lives = hand lives whose reqs fit projected pool
│       2. sort by (P(all pass) desc, score desc)
│       3. add lives while P(all pass as a GROUP) ≥ stance floor
│       4. NEVER add a life that fails reqs (all-or-nothing, 8.3.16)
│       5. spare slots + ahead: dump dead NON-LIVE cards (hand filter)
│
PERFORMANCE/VICTORY: automatic (no decisions) — but log outcomes to update
opponent modeling (their flip luck, life counts in hand).
```

---

## 4. AUDIT — current bots vs this tree

| node | status in v3 today |
|---|---|
| L1 estimator | exists (`estimate_max_score`) but ignores hand lives' scores & スコア+1 icons |
| L2 match-point contest | inherited from v2 floors, not explicit |
| L3 concede | added, margin=2 bands; needs the tie-value table above |
| L4 portfolio | v2 MC ladder is close; **v3's dump previously violated rule 1.1b** (fixed for members; uncovered lives must also never be added) |
| M1 baton preference | weak — eval counts stage cost but not energy-efficiency of the swap |
| M2 activation usage | v3 uses far fewer 起動 than v2 (trace: 28 vs 60/game) — acquisition deltas undervalue them |
| position keywords | ignored entirely by eval |
| turn-order planning (node 3) | none |

## 5. Fix order (each is a leaf implementation, testable via bot_arena)

1. L4 hard rule: never include a failing life in the portfolio (any state).
2. L1/L3: replace band heuristic with exact table above (median column).
3. M2: make main-phase eval actually simulate 起動 effects (it clone-evals
   already — ensure UseAbility actions aren't pruned by acquisition deltas).
4. Tie-value table in L3 (tie = win at ≤1, loss at 2, suicide at 2-2).
5. M1: explicit baton-efficiency term (net energy per ceiling point).
