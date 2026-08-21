# Bot Strategy Research — Loveca (Love Live! Series Official Card Game)

Research findings from competitive Japanese play guides, synthesized into a
general strategy that the experimental bot (`engine/src/bot/strategy.rs`)
implements. Written 2026-08-21.

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
8. **公式ラブカポイントシステム**（環境カード評価 — 間接的な「何が強い」の指標）
   <https://llofficial-cardgame.com/lovecapointsystem/>

## Game facts that drive strategy

- Win: first player to 3 cards in their Success Live Card zone (rule 1.2.1.1).
  Both reaching 3+ simultaneously = draw (1.2.1.2). Engine:
  `TurnEngine::check_victory_condition` (`engine/src/turn/actions.rs:1317`).
- Live check: if only one player succeeded, they win regardless of score.
  If both succeeded, higher total score wins; **tie = both win**, but a player
  with 2 success cards already cannot place on a tie (8.4.7.1).
- Turn order: whoever *alone* placed a success card becomes 先攻 next turn
  (8.4.13). Going second lets you see the opponent's live before committing.
- Score comparison adds +1 per スコア+1 icon gained from yell.
- Baton touch: playing over a member already on stage reduces the new member's
  cost by the sent member's cost; the sent member must have been on stage since
  a previous turn.

## Synthesized "grand strategy" (deck-agnostic)

The deck-specific guides (Liella draw, Emarange, mirapax aggro) all reduce to
the same general principles:

### S1. The cost curve is the resource race
Every deck grows stage total cost on a schedule: T1 ≈ cost-2/4 member,
T2 baton-touch to ~9 (2→7), T3 ≈ 13–15, T4 aim for the big center (22).
Higher cumulative stage cost ⇒ more hearts ⇒ higher achievable live scores.
*Generalization:* maximize stage total-cost growth per turn while spending as
little energy as possible; prefer baton touches where the sent member's cost
covers most of the new member's cost (the guides' 「4→4」「4→9」 swaps).

### S2. Score bands are predictable from public info
Required hearts scale with score: 1点 ≈ 3–4 hearts, 2点 ≈ 5–6, i.e. roughly
`2·score + 1..2`. So the opponent's maximum achievable score this turn is
estimable from their visible hearts + blades + expected yell flips (source 3).
*Generalization:* before setting lives, estimate both sides' max score; don't
set a life you will almost certainly lose.

### S3. Live-card discipline (温存)
Don't waste low-score lives into a clearly losing comparison — hold them for
multi-card high-score turns late (sources 1, 3). Conversely, if the opponent
cannot win the check anyway, a cheap successful life steals tempo for free.

### S4. Concede-a-turn vs 千秋楽 urgency
If you can't outscore the opponent this turn, it is often correct to skip the
live entirely and bank resources for a reversal (sources 1, 2, 3). **But** if
the opponent already has 2 success cards (千秋楽 turn), failing to contest
loses the game — you must win the check outright (a tie does not save you,
rule 8.4.7.1).

### S5. Blade/yell economics
Yell count = total blades on your active members; each revealed blade heart is
+1 heart for success AND +1 score at comparison. Know your deck's blade-heart
density; guides warn against lives that need many flips when density is low
(source 3: ~1/2 hit rate example). Retrieving non-blade members before refresh
raises post-refresh yell quality (source 1).

### S6. Tempo / turn-order value
Winning the live check alone makes you 先攻 next turn — worth real value in
the eval, but going second on key turns lets you respond to the opponent's
score (source 3: 後攻が有利, RPS winner should pick second).

### S7. Mulligan = curve completion
Keep pieces that complete the T2/T3 curve (cost-7 for T2, cost-13 for T3,
retrieval members); mulligan unplayable hands aggressively (sources 1, 3).

## Mapping to the bot

Implemented in `engine/src/bot/strategy.rs` (experimental, not wired into the
web/console games):

| Principle | Implementation |
|---|---|
| S1 | `evaluate_state`: stage total-cost term (own − opp), energy-efficiency via active-energy term |
| S2 | `estimate_max_score(view)`: hearts + blades → score-band estimate |
| S3/S4 | Live-zone/hand-live terms + `urgency` multiplier when opponent has 2 success cards; terminal values dominate |
| S5 | blade totals term |
| S6 | first-attacker bonus |
| Search | UCB1 adaptive sampling over determinized rollouts (`ismcts.rs`), immediate-win short-circuit |

## Fairness policy

The old bot sampled the opponent's hidden hand/deck from their **actual deck
list** (`DeterminizationSampler::new(db, our_deck, opp_deck)`), which is
information a fair player does not have in casual play. Now:

- Default (`Bot::new_fair` / `open_decklists == false`): opponent hidden cards
  are sampled from an anonymous pool of all Member/Live cards in the database,
  minus what their public zones reveal. Only our own deck list is used.
- Open-list mode (tournament style, lists are published) remains available via
  `DeterminizationSampler::new` / `open_decklists(true)` for research.

Decisions are made from `PublicObservation` only; rollouts operate on
determinized states (standard PIMC practice — both sides then play with the
same information inside the simulation).
