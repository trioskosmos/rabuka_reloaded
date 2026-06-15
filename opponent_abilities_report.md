# Opponent-Interaction Abilities — Full Report

## Structural Gaps Found

| Gap | Description | Severity |
|-----|-------------|----------|
| **G1** | When `handle_both_targets` / `execute_move_cards_both` runs the opponent half, any `Choice::SelectCard` created has `target_player_id = "opponent"` but `choice_player_id` on the queue entry stays as the ability owner (P1). P1 can make P2's choices. | **Critical** |
| **G2** | `ChoiceView.js` renders every `pending_choice` regardless of `choice_player_id`. Both players see every prompt. | **Critical** |
| **G3** | `action_by: opponent` sub-actions create choices without setting `choice_player_id = opponent`. | **High** |
| **G4** | `choice_player_id` IS injected into JSON (abilities.rs:645-650) — no extra fix needed. | OK |

### Fixes needed per file:

| File | Change |
|------|--------|
| `engine/src/core/game_state/abilities.rs` in `process_current_ability` | After `pause_for_choice`, check `target_player_id` on the choice; if "opponent" vs entry's `player_id`, set `entry.choice_player_id = opponent_id` |
| `engine/src/ability/effects/misc.rs` in `execute_choice` | When `choice_maker != "opponent"` but the context targets opponent (`spawn_context.target == "opponent"`), still set `choice_player_id = opponent` |
| `engine/src/ability/effects/mod.rs` in `execute_effect` | When processing `action_by: opponent`, propagate `choice_player_id = opponent` to any sub-choices |
| `web_ui/js/components/ChoiceView.js` | At top of `render()`, if `choice.choice_player_id` doesn't match viewer → show "Waiting for opponent..." and return |

---

## Category A: True Opponent Choice (`choice_maker: opponent`)
Opponent picks from free-form text answers. **2 abilities.**

### A1: PL!N-PR-022-PR エマ・ヴェルデ (Emma Punch)
**Trigger:** `登場`
**Text:**
> 直前のターンに相手がライブをし、それが成功していない場合、相手にエマパンチ打つ？と聞いてもよい。回答がお願いしますの場合、自分は相手にエマパンチする。ライブ終了時まで、相手のステージにいるすべてのメンバーは、ブレードを得る。回答がそれ以外の場合、何もしない。
**Flow:**
1. If opponent's last live failed → may ask "Emma Punch?"
2. Opponent answers (choice_maker: opponent)
3. "お願いします" → all opponent members get blade
4. Anything else → nothing
**Current:** `choice_player_id` set correctly by `execute_choice`. **G2** only (frontend must show it to P2).

### A2: LL-PR-004-PR 愛♡スクリ～ム！ (Ice Cream)
**Trigger:** `ライブ開始時`
**Text:**
> 相手に何が好き？と聞く。回答がチョコミントかストロベリーフレイバーかクッキー＆クリームの場合、自分と相手は手札を1枚控え室に置く。回答があなたの場合、自分と相手はカードを1枚引く。回答がそれ以外の場合、ライブ終了時まで、自分と相手のステージにいるメンバーはブレードを得る。
**Flow:**
1. Ask opponent "what do you like?"
2. 3 answer categories → both discard / both draw / both get blade
3. Each answer's sub-effects have `target: both`
**Current:** Same as A1. The `target: both` sub-effects after opponent answers need verification that they apply to both correctly. **G2.**

---

## Category B: Opponent-Initiated Action (`action_by: opponent`)
Opponent must perform an action (discard, select, etc.) or face a penalty. **8 abilities.**

### B1: PL!S-pb1-002-R/P+ 桜内梨子
**Trigger:** `登場`
**Text:**
> 相手は手札からライブカードを1枚控え室に置いてもよい。そうしなかった場合、ライブ終了時まで、「常時ライブの合計スコアを＋１する。」を得る。
**Flow:**
1. Opponent MAY discard a live card from hand (action_by: opponent)
2. If not → you get "constant: +1 score"
**Current:** `action_by: opponent` is parsed. Sub-action `move_cards` with `target: opponent`. When this creates a prompt, `choice_player_id` is the ACTING player, not the opponent. **G3, G2.**

### B2: PL!S-pb1-006-R/P+ 津島善子
**Trigger:** `起動`
**Cost:** Reveal a live card from hand
**Text:**
> 相手は手札を1枚控え室に置いてもよい。そうしなかった場合、ライブ終了時まで、ブレード＋４を得る。
**Flow:**
1. Cost: reveal live card
2. Opponent MAY discard 1 card from hand
3. If not → you get +4 blade
**Current:** Same pattern as B1. **G3, G2.**

### B3a: PL!-pb1-015-R/P+ 西木野真姫
**Trigger:** `ライブ開始時` / `登場` (if center is BiBi)
**Cost (optional):** Wait 1 BiBi in center
**Text:**
> 相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
**Flow:**
1. Optional cost: wait your BiBi center
2. Opponent chooses 1 of their ACTIVE members to wait
**Current:** `action_by: opponent`, `action: select` from opponent's stage. This should create `Choice::SelectCard` with `target_player_id = opponent` and `choice_player_id = opponent`. Currently `choice_player_id` is NOT set. **G1** (select from opponent's zone needs choice_player_id = opponent), **G2.**

### B3b: PL!-bp4-009-R/P 矢澤にこ
**Trigger:** `登場`
**Text:**
> 相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
**Flow:**
1. On appear → opponent chooses 1 active member to wait
**Current:** Same pattern as B3a. **G1, G2.**
**Note:** Listed as `opponent_action` untested (per test_coverage_report).

### B3c: PL!HS-bp6-007-R/P セラス 柳田 リリエンフェルト
**Trigger:** `自動` (when EdelNote member appears on your stage)
**Text:**
> 自分のステージに『EdelNote』のメンバーが登場したとき、相手は、自身のステージにいるアクティブ状態のメンバー1人をウェイトにする。
**Flow:**
1. When your EdelNote member appears → opponent chooses 1 active member to wait
**Current:** Same pattern as B3a. **G1, G2.**
**Note:** Listed as `opponent_action` untested.

### B4: (Unnamed — part of a selection chain)
**Text:**
> 相手はそれらのカードのうち1枚を選ぶ。
**Flow:**
1. You pre-select 2 cards → opponent chooses 1 of them
**Current:** `action_by: opponent`, `action: select` from `source: selected_cards`. The choice needs to go to opponent. **G3, G2.**

### B5: (Sequential draw)
**Text:**
> 相手はカードを1枚引く。
**Flow:** Opponent draws 1 card. **No interactive choice** — just applies the draw to opponent. No fix needed.

### B6: PL!S-bp6-024-L コワレヤスキ
**Trigger:** `ライブ成功時`
**Text:**
> 相手は余剰ハートをすべて失う。これにより相手が余剰ハートを2つ以上失っている場合、このカードのスコアを＋１する。
**Flow:** Opponent loses all surplus hearts. **No interactive choice.** No fix needed.

---

## Category C: "Both" Target with Interactive Choices
Both players independently select cards from their own zones. This is the user's reported issue and the most common pattern needing the fix. **~5 abilities need fixes.**

### C1: [Multiple cards] 共闘 (Both deploy from discard)
**Text:**
> 自分と相手はそれぞれ、自身の控え室からコスト2以下のメンバーカードを1枚、メンバーのいないエリアにウェイト状態で登場させる。
**Flow:**
1. P1's ability triggers
2. P1 picks 1 member card (cost ≤2) from P1's discard → deploys to empty area in wait
3. P2 picks 1 member card (cost ≤2) from P2's discard → deploys to empty area in wait
**Current:** `execute_move_cards_both` runs. Opponent half creates `Choice::SelectCard` with `target_player_id = opponent`. But `choice_player_id` stays as P1. **G1, G2.**
**Expected:** Step 2 shows choice prompt to P2. After P2 resolves, step 3 shows to P1.

### C2: N-bp4-007 variants (Both retrieve live card)
**Text:**
> 自分と相手はそれぞれ、自身の控え室からライブカードを1枚手札に加える。
**Flow:** Both pick 1 live card from own discard to hand.
**Current:** Same pattern as C1. **G1, G2.**

### C3: S-bp2-024 variants (Both discard to 3, then both draw 3)
**Text:**
> 自分と相手はそれぞれ自身の手札の枚数が3枚になるまで手札を控え室に置き、その後自分と相手はそれぞれカードを3枚引く。
**Flow:**
1. Both discard from hand down to 3 cards (each chooses which to discard)
2. Both draw 3 cards
**Current:** Sequential compound: first `discard_until_count(3)` for both, then `draw 3` for both. The discard step requires selection. If `discard_until_count` creates a `Choice::SelectCard` with `target_player_id = opponent`, same G1/G2 gap. **Check if `discard_until` creates choices with correct `choice_player_id`.**

### C4: S-bp5-011-N (Both draw 1, both discard 1)
**Text:**
> 自分と相手はカードを1枚引き、手札を1枚控え室に置く。
**Flow:**
1. Both draw 1 card
2. Both discard 1 card (each chooses which)
**Current:** Similar to C3. The discard step for opponent needs selection routing. **G1, G2 if discard creates a choice.**

### C5: SP-bp5-010 variants (Both position-change center)
**Text:**
> 自分と相手は、自身のステージのセンターにいるメンバーをポジションチェンジする。
**Flow:** Both swap their center member position.
**Current:** `target: both`, `action: position_change`. The position_change handler handles `both` internally. If it creates a choice (e.g., for selecting destination), that choice needs routing. **Check if choice is created; G1/G2 if yes.**

---

## Category D: Opponent as Target with Selection (`target: opponent` + `select`)
**You** choose from the opponent's zone. These are CORRECT — you are the chooser, not the opponent. **~25 abilities.**

### D1: (Generic wait-opponent pattern) — DO NOT BREAK
**Text (representative):**
> 相手のステージにいるコスト4以下のメンバー1人をウェイトにする。
**Flow:**
1. YOU select 1 of opponent's members to wait
**Current:** `target: opponent`, `action: change_state`. The selection (`Choice::SelectCard`) has `target_player_id = opponent` but `choice_player_id` is the acting player (you). This is CORRECT — you choose which opponent card to wait.
**Fix must preserve:** When `target_player_id = opponent` but the EFFECT is not a "both" or "action_by: opponent" context, `choice_player_id` should stay as the acting player. The G1 fix must be narrowly scoped: only override `choice_player_id` when the *ability's* `target` is `"both"` or `action_by` is `"opponent"`.

---

## Category E: Both with Global/Simultaneous Effects (No interactive choice)
~15 abilities. Examples: global restrictions, both-can't-do-X, conditions checking total energy, etc. **No fixes needed.**

---

## Summary of Changes Required

### Engine changes (in priority order):

1. **`process_current_ability`** (abilities.rs): After `pause_for_choice`, inject `choice_player_id = opponent` when:
   - The choice's `target_player_id == "opponent"` AND the queue entry's `player_id` differs from the implied chooser
   - OR the `spawn_context.target` was "opponent" when the choice was created

2. **`execute_choice`** (misc.rs): When `choice_maker != "opponent"` but the resolver's `spawn_context.target == "opponent"`, treat as opponent choice

3. **`execute_effect`** (effects/mod.rs): When processing `action_by: opponent` sub-actions that create choices, ensure `choice_player_id = opponent`

### Frontend change:

4. **`ChoiceView.js`**: At render-entry, check `choice.choice_player_id`:
   ```
   if choice_player_id is set AND doesn't match viewer's player → show "Waiting for opponent..."
   ```
