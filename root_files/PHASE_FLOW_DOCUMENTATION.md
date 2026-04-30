# Phase Flow Documentation

This document describes the complete game phase flow with references to `rules.txt` and `qa_data.json`, and verifies the implementation in the code.

## Game Preparation (Pre-Game Phases)

### 1. RockPaperScissors Phase
**Rules.txt Reference:** Rule 6.2.1.4
- Each player randomly selects which player will be the first attacker
- This is implemented as Rock-Paper-Scissors

**qa_data.json Reference:** Q16
- "ゲームの準備での先攻・後攻はどのように決めますか？"
- Answer: "じゃんけんで勝ったプレイヤーが先攻か後攻を決めます。"

**What Happens:**
- Player 1 chooses Rock, Paper, or Scissors
- Player 2 (AI) randomly chooses Rock, Paper, or Scissors
- Winner is determined
- If Player 2 wins, they automatically choose to be first attacker (AI strategy)
- If Player 1 wins, they proceed to ChooseFirstAttacker phase

**Code Implementation:**
- `src/turn.rs`: `handle_rps_choice()` - handles RPS choice and determines winner
- `src/game_setup.rs`: Generates RockChoice, PaperChoice, ScissorsChoice actions
- `src/web_server.rs`: RockPaperScissors is a manual phase (requires player input)

**Phase Transition:**
- If Player 2 wins: → Mulligan (Player 2 auto-chooses first attacker)
- If Player 1 wins: → ChooseFirstAttacker

---

### 2. ChooseFirstAttacker Phase
**Rules.txt Reference:** Rule 6.2.1.4
- RPS winner chooses whether to be first or second attacker

**qa_data.json Reference:** Q16
- Same as above - RPS winner chooses turn order

**What Happens:**
- Only reached if Player 1 won RPS
- Player 1 chooses to go first or second
- Sets `is_first_attacker` flags accordingly

**Code Implementation:**
- `src/turn.rs`: `handle_attacker_choice()` - sets first/second attacker flags
- `src/game_setup.rs`: Generates ChooseFirstAttacker, ChooseSecondAttacker actions
- `src/web_server.rs`: ChooseFirstAttacker is a manual phase (requires player input)

**Phase Transition:**
- After choice: → Mulligan

---

### 3. Mulligan Phase
**Rules.txt Reference:** Rule 6.2.1.6
- "先攻プレイヤーから順に、各プレイヤーは自身の手札のカードを任意の枚数選んで裏向きに脇に置き、置いた枚数と同じ枚数のカードを自身のメインデッキ置き場の上から自身の手札に移動し、脇に置いたカードをメインデッキ置き場に移動し、1 枚以上移動した場合はシャッフルします。"

**qa_data.json Reference:** Q17, Q18, Q19
- Q17: "ゲームの準備での手札の入れ替えは、先攻と後攻どちらのプレイヤーから行いますか？" - Answer: "先攻のプレイヤーから行います。"
- Q18: "ゲームの準備での手札の入れ替えは、何回も行うことはできますか？" - Answer: "いいえ、プレイヤーごとに1回までです。"
- Q19: "ゲームの準備での手札の入れ替えは、必ず行う必要がありますか？" - Answer: "いいえ、入れ替えずにそのまま手札にすることもできます。その場合、メインデッキはシャッフルしません。"

**What Happens:**
- First attacker goes first (current_mulligan_player_idx = 0 for P1, 1 for P2)
- Each player can:
  - Select cards to mulligan (any number, including 0)
  - Confirm mulligan (draw new cards, shuffle mulliganed cards back)
  - Skip mulligan (keep all cards)
- After both players complete (current_mulligan_player_idx >= 2), advance to Active

**Code Implementation:**
- `src/turn.rs`: 
  - `handle_mulligan_selection()` - selects cards for mulligan
  - `handle_mulligan_confirmation()` - confirms and processes mulligan
  - `handle_mulligan_skip()` - skips mulligan
  - `advance_phase()` - checks if current_mulligan_player_idx >= 2, then sets up initial energy and advances to Active
- `src/game_setup.rs`: Generates SelectMulligan, ConfirmMulligan, SkipMulligan actions
- `src/bot/ai.rs`: AI prefers SkipMulligan (line 29-34)
- `src/web_server.rs`: Mulligan is a manual phase, but AI can auto-play during it

**Phase Transition:**
- After both players complete: → Active (first attacker's turn)

---

## Normal Phase (First Attacker)

### 4. Active Phase
**Rules.txt Reference:** Rule 7.4
- "手番プレイヤーは、自身のエネルギー置き場とメンバーエリアのウェイトのカードをすべてアクティブにします。"
- "'ターンの始めに'および'アクティブフェイズの始めに'の誘発条件が発生します。また、これがゲームにおける最初のターンである場合、'ゲームの始めに'の誘発条件が発生します。"
- "チェックタイミングが発生します。このチェックタイミングで行うべき処理がすべて終了したら、アクティブフェイズが終了します。"

**What Happens:**
- Automatic phase - no player actions
- Activate ALL players' energy and stage cards (not just active player)
- Trigger "turn start" and "active phase start" abilities
- Check timing
- Auto-advance to Energy

**Code Implementation:**
- `src/turn.rs`: `advance_phase()` - activates energy for both players, triggers abilities, advances to Energy
- `src/web_server.rs`: Active is an automatic phase (auto-advances)

**Phase Transition:**
- Auto-advance: → Energy

---

### 5. Energy Phase
**Rules.txt Reference:** Rule 7.5
- "'エネルギーフェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
- "手番プレイヤーは、自身のエネルギーデッキの一番上のカードを、自身のエネルギー置き場に移動します。"
- "チェックタイミングが発生します。このチェックタイミングで行うべき処理がすべて終了したら、エネルギーフェイズが終了します。"

**What Happens:**
- Automatic phase - no player actions
- Trigger "energy phase start" abilities
- Active player draws 1 energy card from energy deck to energy zone
- Check timing
- Auto-advance to Draw

**Code Implementation:**
- `src/turn.rs`: `advance_phase()` - triggers abilities, draws energy, advances to Draw
- `src/web_server.rs`: Energy is an automatic phase (auto-advances)

**Phase Transition:**
- Auto-advance: → Draw

---

### 6. Draw Phase
**Rules.txt Reference:** Rule 7.6
- "'ドローフェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
- "手番プレイヤーはカードを 1 枚引きます。"
- "チェックタイミングが発生します。このチェックタイミングで行うべき処理がすべて終了したら、ドローフェイズが終了します。"

**What Happens:**
- Automatic phase - no player actions
- Trigger "draw phase start" abilities
- Active player draws 1 card from main deck to hand
- Check timing
- Auto-advance to Main

**Code Implementation:**
- `src/turn.rs`: `advance_phase()` - triggers abilities, draws card, advances to Main
- `src/web_server.rs`: Draw is an automatic phase (auto-advances)

**Phase Transition:**
- Auto-advance: → Main

---

### 7. Main Phase
**Rules.txt Reference:** Rule 7.7
- "'メインフェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
- "手番プレイヤーにプレイタイミング（9.5.2）が与えられます。このプレイタイミングでは以下のいずれかが実行できます。"
  - "自分のカードが持つ起動能力を 1 つ選び、それをプレイする。"
  - "自分の手札のメンバーカードを 1 枚選び、それをプレイする。"
- "メインフェイズが終了します。"

**What Happens:**
- Manual phase - player actions required
- Trigger "main phase start" abilities
- Active player can:
  - Play member cards from hand to stage
  - Use activation abilities
  - Pass to end phase
- When player passes or no actions available, advance to next phase

**Code Implementation:**
- `src/turn.rs`: 
  - `handle_play_member_to_stage()` - plays member to stage
  - `handle_use_ability()` - uses activation ability
  - Pass action handler - advances phase
- `src/game_setup.rs`: Generates PlayMemberToStage, UseAbility, Pass actions
- `src/bot/ai.rs`: AI prioritizes PlayMemberToStage and UseAbility over Pass (lines 66-81)
- `src/web_server.rs`: Main is a manual phase (requires player/AI action)

**Phase Transition:**
- After first attacker's Main: → Active (second attacker's turn)
- After second attacker's Main: → LiveCardSet

---

## Live Phase

### 8. LiveCardSet Phase
**Rules.txt Reference:** Rule 8.2
- "'ライブフェイズの始めに'および'ライブカードセットフェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
- "先攻プレイヤーは、自身の手札のカードを 3 枚まで選んで裏向きに自身のライブカード置き場に置き、置いた枚数と同じ枚数カードを引きます。"
- "チェックタイミングが発生します。"
- "後攻プレイヤーは、自身の手札のカードを 3 枚まで選んで裏向きに自身のライブカード置き場に置き、置いた枚数と同じ枚数カードを引きます。"
- "チェックタイミングが発生します。このチェックタイミングで行うべき処理がすべて終了したら、ライブカードセットフェイズが終了します。"

**What Happens:**
- Manual phase - player actions required
- Trigger "live phase start" and "live card set phase start" abilities
- First attacker sets up to 3 cards face-down to live card zone, then draws equal number
- Check timing
- Second attacker sets up to 3 cards face-down to live card zone, then draws equal number
- Check timing
- Both players done → advance to FirstAttackerPerformance

**Code Implementation:**
- `src/turn.rs`:
  - `handle_set_live_card()` - places card to live zone or signals completion
  - `current_live_card_set_player` tracks progress (0=P1, 1=P2, 2=both done)
  - Pass action handler - advances to next player or marks both done
- `src/game_setup.rs`: Generates SetLiveCard, Pass actions
- `src/bot/ai.rs`: AI has 30% chance to pass even when SetLiveCard available (lines 49-69)
- `src/web_server.rs`: LiveCardSet is a manual phase (requires player/AI action)

**Phase Transition:**
- After both players complete: → FirstAttackerPerformance

---

### 9. FirstAttackerPerformance Phase
**Rules.txt Reference:** Rule 8.3
- "パフォーマンスフェイズとは、いずれかのプレイヤーがライブの一連の処理を実行するフェイズです。"
- "各パフォーマンスフェイズでは手番プレイヤーを 1 人指定し、そのプレイヤーを基準として各フェイズを実行します。"
- "パフォーマンスフェイズには、先攻プレイヤーが手番プレイヤーである'先攻パフォーマンスフェイズ'と、後攻プレイヤーが手番プレイヤーである'後攻パフォーマンスフェイズ'があります。"
- "手番プレイヤーの自動能力の'パフォーマンスフェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
- "手番プレイヤーは自身のライブカード置き場のカードをすべて表向きにし、その中のライブカードでないすべてのカードを自身の控え室に置きます。"
- "手番プレイヤーが'ライブできない'状態である場合、表向きにしたすべてのカードを自身の控え室に置きます。"
- "チェックタイミングが発生します。"
- "この時点で手番プレイヤーのライブカード置き場にカードが無い場合、パフォーマンスフェイズを終了します。"

**What Happens:**
- Automatic phase - no player actions
- First attacker is active player
- Trigger "performance phase start" abilities
- Reveal all live cards face-up
- Send non-live cards to waiting room
- If "cannot live" state, send all cards to waiting room
- Check timing
- If no live cards, end performance phase
- Otherwise, perform live (cheer, calculate hearts, determine success)
- Auto-advance to SecondAttackerPerformance

**Code Implementation:**
- `src/turn.rs`: `advance_phase()` - handles live performance logic
- `src/web_server.rs`: FirstAttackerPerformance is an automatic phase (auto-advances)

**Phase Transition:**
- Auto-advance: → SecondAttackerPerformance

---

### 10. SecondAttackerPerformance Phase
**Rules.txt Reference:** Rule 8.3
- Same as FirstAttackerPerformance, but for second attacker

**What Happens:**
- Automatic phase - no player actions
- Second attacker is active player
- Same process as FirstAttackerPerformance
- Auto-advance to LiveVictoryDetermination

**Code Implementation:**
- `src/turn.rs`: `advance_phase()` - handles live performance logic
- `src/web_server.rs`: SecondAttackerPerformance is an automatic phase (auto-advances)

**Phase Transition:**
- Auto-advance: → LiveVictoryDetermination

---

### 11. LiveVictoryDetermination Phase
**Rules.txt Reference:** Rule 8.4
- "'ライブ判定フェイズの始めに'の誘発条件が発生し、チェックタイミングが発生します。"
- "ライブカード置き場にカードがあるプレイヤーは、自身のライブカード置き場のカードのスコアを合計します。"
- "その際、各プレイヤーは自身のエールのアイコン 1 つにつきスコアの合計に 1 を加算します。"
- "ライブの合計スコアを比較する場合、それは以下の手順で実行します。"
- "両方のプレイヤーのどちらのライブカード置き場にもカードが無い場合、両プレイヤーの合計スコアは 0 です。"
- "一方のプレイヤーのライブカード置き場にカードがあり、もう一方のプレイヤーのライブカード置き場にカードが無い場合、カードがあるプレイヤーがライブに勝利します。"
- "両方のプレイヤーのライブカード置き場にカードがある場合、合計スコアが高いプレイヤーがライブに勝利します。"
- "合計スコアが同じ場合、両プレイヤーがライブに勝利します。"

**qa_data.json Reference:** Q49, Q50, Q51, Q52
- Q49: Neither player wins → first/second attacker unchanged
- Q50: Both players win and both place cards → first/second attacker unchanged
- Q51: Both players win but only one places cards → winner becomes first attacker
- Q52: Both players win but neither places cards → first/second attacker unchanged

**What Happens:**
- Automatic phase - no player actions
- Trigger "live victory determination phase start" abilities
- Calculate scores (including cheer icons)
- Compare scores
- Determine winner(s)
- Winners move live cards to success live card zone (max 3, or 1 for half deck)
- Update first/second attacker for next turn based on who placed cards
- Check for game end condition (3+ success cards vs 2-)
- Auto-advance to Active (next turn)

**Code Implementation:**
- `src/turn.rs`: `execute_live_victory_determination()` - handles victory determination
- `src/web_server.rs`: LiveVictoryDetermination is an automatic phase (auto-advances)

**Phase Transition:**
- If game continues: → Active (next turn, first attacker)
- If game ends: Game over

---

## Turn Structure Summary

**Rules.txt Reference:** Rule 7.1.2
- "各ターンは、'先攻通常フェイズ'、'後攻通常フェイズ'（7.3）、'ライブフェイズ'（7.8）を順に実行することで進行します。"

**Turn Flow:**
1. First Attacker Normal Phase:
   - Active → Energy → Draw → Main
2. Second Attacker Normal Phase:
   - Active → Energy → Draw → Main
3. Live Phase:
   - LiveCardSet → FirstAttackerPerformance → SecondAttackerPerformance → LiveVictoryDetermination
4. Repeat from step 1

---

## Code Verification

### Manual Phases (Require Player/AI Action)
- RockPaperScissors ✓
- ChooseFirstAttacker ✓
- Mulligan ✓
- Main ✓
- LiveCardSet ✓

### Automatic Phases (Auto-Advance)
- Active ✓
- Energy ✓
- Draw ✓
- FirstAttackerPerformance ✓
- SecondAttackerPerformance ✓
- LiveVictoryDetermination ✓

### Phase Tracking
- `current_phase` in `GameState` ✓
- `current_turn_phase` in `GameState` (FirstAttackerNormal, SecondAttackerNormal, Live) ✓
- `current_mulligan_player_idx` for Mulligan phase ✓
- `current_live_card_set_player` for LiveCardSet phase ✓

### Active Player Determination
- `active_player()` in `GameState` correctly handles:
  - Mulligan phase (uses current_mulligan_player_idx) ✓
  - Normal phases (uses first/second attacker) ✓
  - Live phase (uses first/second attacker) ✓

---

## Issues Found and Fixed

### Issue 1: Phases Being Skipped
**Problem:** RockPaperScissors and ChooseFirstAttacker were auto-advancing, skipping to LiveCardSet
**Fix:** Added these phases to manual phase list in `web_server.rs`

### Issue 2: Mulligan Phase Looping
**Problem:** Mulligan phase was not advancing after both players completed
**Fix:** Modified `turn.rs` to continue auto-advancing through Active, Energy, Draw after Mulligan completes

### Issue 3: LiveCardSet Phase Looping
**Problem:** AI was always setting cards, never passing, causing infinite loop
**Fix:** Modified AI to have 30% chance to pass even when SetLiveCard is available

### Issue 4: Main Phase AI Behavior
**Problem:** AI was passing immediately without playing cards
**Fix:** Modified AI to prioritize PlayMemberToStage and UseAbility over Pass

---

## References

- **rules.txt**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\rules\rules.txt`
- **qa_data.json**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\cards\qa_data.json`
- **Phase Implementation**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\turn.rs`
- **Game State**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\game_state.rs`
- **Action Generation**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\game_setup.rs`
- **AI Logic**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\bot\ai.rs`
- **Web Server**: `c:\Users\trios\OneDrive\Documents\rabuka_reloaded\engine\src\web_server.rs`
