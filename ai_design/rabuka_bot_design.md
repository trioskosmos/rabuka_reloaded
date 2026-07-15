# Rabuka Reloaded — AI Bot Design Document

> *How to build a legitimate, non-cheating bot for a complex card game with imperfect information, massive action spaces, and significant RNG.*

---

## 1. Problem Characterization

### 1.1 Game Profile

| Property | Value |
|---|---|
| Players | 2 (fixed) |
| Information | **Imperfect** — deck order is hidden (set once per game by shuffle) |
| Stochasticity | Yes — deck shuffle at game start; yell draws are deterministic given deck order |
| Action space | **Massive** — 2280 unique cards, 1727 abilities, ~12 phase types |
| Win condition | First to 3 live cards in Success Live Card Zone (or opponent concedes) |
| Turn structure | Asymmetric (first/second attacker alternate each normal turn) |
| Time constraints | Real-time via web server; bot should respond in <2s per action |

### 1.2 What the Bot Knows vs. Doesn't Know

```
KNOWN TO BOT (same as human):           UNKNOWN (hidden):
───────────────────────────────         ───────────────────────
Both players' hand sizes                Opponent's exact hand contents
All public zones (stage, energy,        Opponent's main deck order
  waitroom, live, success)              Own main deck order (only top card after yell)
All card abilities (from database)      Opponent's energy deck order
Game phase and turn number              Cards in opponent's resolution zone (during yell)
All modifiers and counters              Future draws for both players
Both players' blade/heart counts
```

**Key insight:** The only hidden information is the order of each player's main deck and energy deck. Everything else is fully observable. This makes the game closer to a "partially observable" problem than a "deep hidden information" one like poker.

### 1.3 RNG Sources

1. **Initial deck shuffle** — sets the entire deck order for the game (no further shuffles unless a card effect says so)
2. **Mulligan** — partial hand redraw; reshuffles the deck
3. **Yell draws** — deterministic given deck order; top N cards are revealed
4. **Card effects that shuffle** — rare, but some abilities reshuffle deck(s)

**Core RNG insight:** The randomness is concentrated in the initial shuffle. After that, the game is almost entirely deterministic with the hidden deck order being the sole uncertainty. This is ideal for determinization-based search.

---

## 2. Why MCTS / ISMCTS

### 2.1 Why Not Minimax / Alpha-Beta

- Branching factor is enormous (often 50–200+ legal actions per turn)
- Game length is medium (15–30 turns) — too deep for full-width search
- Imperfect information means standard minimax is not directly applicable

### 2.2 Why MCTS

- Handles large branching factors gracefully via selective tree growth
- Anytime algorithm — can return a move at any deadline
- No hand-crafted evaluation function required (rollouts substitute)
- Naturally supports stochastic environments via chance nodes

### 2.3 Why ISMCTS (Information Set MCTS)

Standard Perfect-Information MCTS (PIMC) suffers from **search pathology** in hidden-info games: each determinization searches as if the player knows the exact hidden state, leading to overconfidence and bad play.

**ISMCTS** (Cowling et al. 2012) fixes this:
- Tree nodes represent **information sets** — all states consistent with observations
- Each traversal uses a single determinization (sampled hidden state)
- Statistics are stored at the information-set level, not per-state
- Effectively averages strategy over possible hidden states

For Rabuka, since hidden info is limited to deck order, ISMCTS is a natural fit.

---

## 3. Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│                  Bot Orchestrator                        │
│  ┌──────────┐  ┌──────────┐  ┌───────────────────────┐  │
│  │  Phase   │  │  Choice  │  │   Action Generator    │  │
│  │ Handler  │  │ Handler  │  │  (legal moves per      │  │
│  │          │  │          │  │   phase + game state)  │  │
│  └────┬─────┘  └────┬─────┘  └───────────┬───────────┘  │
│       │              │                    │              │
│       └──────────────┴────────────────────┘              │
│                          │                                │
│                    ┌─────▼──────┐                        │
│                    │   MCTS /   │                        │
│                    │  ISMCTS    │                        │
│                    │   Engine   │                        │
│                    └─────┬──────┘                        │
│                          │                                │
│              ┌───────────┴───────────┐                    │
│              │                       │                    │
│     ┌────────▼──────┐     ┌─────────▼─────────┐         │
│     │  Game State   │     │  Determinization   │         │
│     │  Copy + Step  │     │  Sampler (deck     │         │
│     │  (simulator)  │     │   permutations)    │         │
│     └───────────────┘     └───────────────────┘         │
└─────────────────────────────────────────────────────────┘
```

### 3.1 Components

| Component | Responsibility |
|---|---|
| **Phase Handler** | Determines what phase we're in and what actions are legal |
| **Choice Handler** | Resolves multi-step abilities that require sub-choices |
| **Action Generator** | Enumerates all legal actions for the current information set |
| **ISMCTS Engine** | Core tree search with determinizations |
| **Game State Simulator** | Deep-copies GameState, applies actions deterministically |
| **Determinization Sampler** | Samples plausible deck orders consistent with observations |

---

## 4. ISMCTS Implementation Details

### 4.1 Tree Structure

```python
class ISMCTSNode:
    info_set_key: str        # hash of all public information
    children: dict[Action, ISMCTSNode]
    visit_count: int
    total_reward: float
    untried_actions: list[Action]   # actions not yet expanded from this node
    parent: ISMCTSNode | None
    player: int             # whose turn it is at this node
```

**Key design choice:** The `info_set_key` must hash only publicly visible state. In Rabuka, this includes:
- All cards in hand (opponent can see count; for bot, hand is fully known)
- All cards on stage (positions, orientation)
- All energy zone cards
- All waitroom cards
- All live card zones
- All success live card zones
- Current phase and turn number
- All modifiers and ability tracking flags
- Turn-limited ability usage counts (public once used)

### 4.2 Determinization Sampling

Since the only hidden information is deck order, a determinization is a **full assignment of the remaining deck sequence**.

```python
def sample_determinization(public_state):
    """
    Produce one plausible complete state.
    
    The bot knows:
    - Its own hand contents
    - Opponent's hand SIZE (from cards played)
    - Contents of all public zones
    - The complete card database
    
    Unknown:
    - Exact ordering of own remaining deck cards
    - Exact ordering of opponent's remaining deck cards
    - Opponent's hand contents
    
    Strategy: sample uniformly from remaining unknown cards
    for each deck and opponent's hand.
    """
    own_remaining = compute_own_deck_remaining(public_state)
    opp_remaining = compute_opp_deck_remaining(public_state)
    
    # Shuffle remaining cards into plausible deck orders
    own_deck = shuffle(own_remaining)
    opp_deck = shuffle(opp_remaining)
    
    return Determinization(own_deck, opp_deck)
```

**Important:** The bot should track which cards it has seen (drawn, revealed, discarded, played) and maintain a **belief distribution** over the opponent's hand and deck. Cards that haven't been seen by either player should be uniformly distributed.

### 4.3 ISMCTS Algorithm (Single Iteration)

```python
def ismcts_iteration(root_node, game_rules):
    # 1. SAMPLE a determinization (plausible full state)
    determinization = sample_determinization(root_node.public_state)
    
    # 2. SELECTION: traverse tree using UCB1
    node = root_node
    state = determinization.clone()
    path = [node]
    
    while not node.is_terminal() and node.fully_expanded():
        action = select_ucb1(node)
        state.apply_action(action)
        node = node.children[action]
        path.append(node)
    
    # 3. EXPANSION: add one new child
    if not node.is_terminal():
        action = node.pop_untried_action()
        state.apply_action(action)
        child = ISMCTSNode(state, parent=node)
        node.children[action] = child
        path.append(child)
    
    # 4. SIMULATION (rollout): play randomly to terminal
    reward = simulate(state, game_rules)
    
    # 5. BACKPROPAGATION
    for node in reversed(path):
        node.visit_count += 1
        node.total_reward += reward
```

### 4.4 UCB1 Selection

```python
def select_ucb1(node):
    C = 1.4  # exploration constant — sqrt(2) theoretical, tune empirically
    best_score = -inf
    best_action = None
    
    for action, child in node.children.items():
        if child.visit_count == 0:
            return action  # always explore unvisited children first
        
        exploitation = child.total_reward / child.visit_count
        exploration = C * sqrt(ln(node.visit_count) / child.visit_count)
        score = exploitation + exploration
        
        if score > best_score:
            best_score = score
            best_action = action
    
    return best_action
```

**Note on reward:** Use asymmetric rewards:
- Win = +1.0
- Loss = -1.0
- Draw = 0.0
- Scale intermediate rewards during rollout by heuristic (optional)

---

## 5. Handling the Large Action Space

The biggest challenge: at any given phase, there can be **50–200+ legal actions** (play any card from hand, activate any of N abilities, pass, etc.). Raw MCTS will struggle to explore all of them.

### 5.1 Phase-Aware Action Pruning

Different phases have different action categories. Use this structure:

```rust
enum ActionCategory {
    PlayMember(usize),      // hand index
    UseAbility(usize),      // which ability on which card
    BatonTouch(usize, usize), // which member -> which position
    Pass,
    SetLiveCard(usize),     // play a live card from hand
    ActivateEnergy(usize),  // activate energy zone position
    SelectTarget(Vec<i16>), // ability target selection
    ChoiceResponse(usize),  // respond to ability choice prompt
    YellResponse(Vec<usize>), // heart allocation choices
    Concede,
}
```

**Prune by phase:**
- **Main phase:** PlayMember, UseAbility, Pass
- **Live card set phase:** SetLiveCard only
- **Choice/ability resolution:** SelectTarget, ChoiceResponse only
- **Energy phase:** auto-advance (no bot choice needed)
- **Active phase:** auto-advance (no bot choice needed)

This naturally reduces branching factor by 3-5x.

### 5.2 Action Abstraction (Grouping)

Many cards are strategically similar. Group actions by high-level intent:

```python
# Instead of 40 individual "Play member X to position Y" actions:
action_groups = {
    "play_high_cost_beater": [all members with cost >= 3 and high blade],
    "play_cheap_enabler":   [all members with cost <= 2],
    "play_energy_fixer":    [all energy cards],
    "play_specific_card":   [individual important cards],  # keep top-K individually
    "activate_ability_X":   [group by ability trigger type],
}
```

**Progressive widening** then creates children lazily within groups:
- At visit count N_parent, allow up to `k * sqrt(N_parent)` children per group
- Most promising action within group is expanded first

### 5.3 Heuristic Move Ordering

Order untried actions by heuristic score so UCB1 finds good moves faster:

```python
def heuristic_action_score(action, state):
    match action.category:
        case PlayMember:
            # Prefer higher blade, lower cost (efficiency)
            return card.blade / (card.cost + 1)
        case UseAbility:
            # Prefer abilities with no cost or obvious upside
            return score_ability(card.ability, state)
        case SetLiveCard:
            # Prefer live cards with lower need_heart
            return 1.0 / (card.need_heart or 1)
        case _:
            return 0.0
```

### 5.4 Rollout Policy

Rollouts should not be fully random — use a **light heuristic policy**:

```python
def rollout_policy(state):
    """Pick a reasonable action during rollout (not uniform random)."""
    actions = legal_actions(state)
    
    # Weight actions by heuristic
    weights = []
    for a in actions:
        w = 1.0
        match a.category:
            case ActionCategory.Pass:
                w = 0.3  # don't pass too eagerly
            case ActionCategory.PlayMember:
                w = 2.0  # prefer playing members
            case ActionCategory.UseAbility:
                w = 1.5  # prefer using abilities
            case ActionCategory.SetLiveCard:
                w = 1.8  # prefer setting live cards
        weights.append(w)
    
    return random.choices(actions, weights=weights)[0]
```

For better rollouts, add domain-specific rules:
- Don't play a member if you can't afford it (cost > available energy)
- Prefer activating abilities that draw cards or generate resources
- During live: allocate hearts greedily (the engine already does smart allocation)

---

## 6. Evaluation Function (for Truncated Rollouts)

Full rollouts to terminal state are expensive (15–30 turns). Use a **truncated rollout** with an evaluation function after D plies (e.g., D=5).

```python
def evaluate_state(state, player):
    """
    Heuristic evaluation of a non-terminal state.
    Returns score in [-1, 1] from player's perspective.
    """
    score = 0.0
    
    # Primary: success zone progress
    my_success = len(state.players[player].success_live_card_zone)
    opp_success = len(state.players[1-player].success_live_card_zone)
    score += 0.4 * (my_success - opp_success) / 3.0
    
    # Secondary: board presence
    my_blade = sum(state.players[player].stage.active_blades())
    opp_blade = sum(state.players[1-player].stage.active_blades())
    score += 0.2 * tanh((my_blade - opp_blade) / 5.0)
    
    # Energy advantage
    my_energy = state.players[player].energy_zone.active_count()
    opp_energy = state.players[1-player].energy_zone.active_count()
    score += 0.15 * tanh((my_energy - opp_energy) / 3.0)
    
    # Hand size
    my_hand = len(state.players[player].hand)
    opp_hand = len(state.players[1-player].hand)
    score += 0.1 * tanh((my_hand - opp_hand) / 3.0)
    
    # Card advantage (total resources)
    my_total = len(state.players[player].hand) + \
               state.players[player].stage.total_cards() + \
               state.players[player].energy_zone.total()
    opp_total = len(state.players[1-player].hand) + \
                state.players[1-player].stage.total_cards() + \
                state.players[1-player].energy_zone.total()
    score += 0.15 * tanh((my_total - opp_total) / 5.0)
    
    return clip(score, -1.0, 1.0)
```

**When to truncate:**
- If current phase is Live (close to scoring), roll out to victory determination
- Otherwise, roll out D plies (e.g., D=5) then evaluate
- Weight: `reward = (1 - w) * rollout_result + w * eval`, increasing w with depth

---

## 7. Progressive Widening Strategy

Given the large action space, use **double progressive widening**:

```python
def max_children(node):
    """Limit children based on parent visit count."""
    return int(K * sqrt(node.visit_count))
    # K = 5-10, tuned empirically

def select_action_for_expansion(node):
    """
    Among untried actions, pick the one with highest heuristic score.
    This ensures the most promising actions are expanded first.
    """
    best_action = None
    best_score = -inf
    
    for action in node.untried_actions:
        score = heuristic_action_score(action, node.state)
        # Add small noise to break ties
        score += random() * 0.001
        if score > best_score:
            best_score = score
            best_action = action
    
    return best_action
```

---

## 8. Time Management

### 8.1 Iteration Budget

Estimate iteration cost:
- Determinization sampling: ~0.1ms
- Selection: ~0.05ms per node depth
- Expansion: ~0.5ms (action generation + state copy)
- Rollout (D=5): ~5ms (simple heuristic)
- Backpropagation: ~0.05ms

**Target: ~2000 iterations in 2 seconds** (aggressive but feasible with optimized Rust).

### 8.2 Time-per-Phase Allocation

```python
def allocate_time(phase):
    match phase:
        case Phase.Main:
            return 2000ms   # most decisions happen here
        case Phase.LiveCardSet:
            return 1000ms
        case Phase.AbilityChoice:
            return 500ms    # simpler sub-problem
        case _:
            return 200ms    # trivial decisions
```

### 8.3 Early Termination

Stop search early if:
- A move is clearly dominant (visit_count > 90% of root visits)
- Time budget exhausted
- Convergence detected (top-K moves stable for N iterations)

---

## 9. Integration with Existing Engine

### 9.1 Simulator Interface

The existing Rust engine already has all components needed:
- `GameState` — deep-copyable via `Clone`
- `generate_possible_actions()` — lists all legal actions
- `execute_action()` — applies action and advances phase
- Already deterministic given a seeded RNG

The bot needs to:

```rust
/// Bot entry point — called by web_server when it's the AI's turn
fn get_bot_action(
    state: &GameState,
    player: PlayerId,
    time_budget: Duration,
    rng: &mut impl Rng,
) -> Action {
    // 1. Build information set from public state + own hand
    // 2. Run ISMCTS for time_budget
    // 3. Return best action
}
```

### 9.2 Web Server Integration

The existing `web_server.rs` already has `is_ai_game` flag and `AiDriver.js` on the frontend. Replace the random-action endpoint with the ISMCTS engine.

Architectural options:
1. **In-process Rust bot** — runs ISMCTS in same process (fastest)
2. **Separate bot process** — communicates via IPC (cleaner, but slower)
3. **WASM in browser** — runs on client side (offloads server, but limited by JS performance)

**Recommendation:** Option 1 — embed ISMCTS as a library inside the web server. The engine already has the game simulator; ISMCTS just wraps it.

### 9.3 Bot vs. Human Differences

The bot should use the exact same `GameState` and `execute_action` code as the human path. No special privileges. This ensures:
- The bot cannot see hidden information (it only uses what's in `GameState`)
- The bot must pay costs, respect phase restrictions, etc.
- If the engine has a bug, the bot experiences it too

---

## 10. Advanced Techniques (Future Work)

### 10.1 Neural Network Guidance

Replace heuristic rollouts with a neural network:
- **Policy head:** `P(a | s)` — reduces branching factor by prioritizing actions
- **Value head:** `V(s)` — replaces rollout entirely (AlphaZero style)

```python
def neural_selection_bias(node, action):
    """Add neural policy prior to UCB1."""
    P = policy_network.predict(node.state, action)
    return Q(node, action) + C * P * sqrt(ln(N_parent)) / (1 + n_child)
```

**Training:** Self-play with ISMCTS as the teacher. Store (state, policy_target, value_target) from search. Train a small transformer/ResNet.

### 10.2 Re-determinizing ISMCTS

If the bot leaks hidden info through its action choices (opponent can deduce deck contents from bot's play), add re-determinization:
- Track belief distribution over opponent's hand
- Sample determinizations from this belief (not uniform)
- Update belief after each opponent action (Bayesian update)

### 10.3 Opening Book

Precompute strong first-turn actions via offline self-play. Store as:
```json
{
  "opening_hand_hash": "abc123",
  "recommended_actions": [
    {"action": "PlayMember(2)", "weight": 0.7},
    {"action": "Pass", "weight": 0.3}
  ]
}
```

### 10.4 Parallel Search

Three levels of parallelism:

| Level | Complexity | Speedup | Notes |
|---|---|---|---|
| Root parallel | Trivial | ~Nx for N threads | Each thread runs independent ISMCTS, vote on best move |
| Tree parallel | Moderate | ~Nx | Shared tree with locking, higher quality per iteration |
| Leaf parallel | Easy | ~Nx | Rollouts only, synchronize at backprop |

**Recommendation:** Start with root parallel (simplest, effective).

---

## 11. Testing & Evaluation

### 11.1 Benchmark Suite

```python
benchmarks = {
    "random_vs_random":      (RandomBot, RandomBot,      10_000 games),
    "random_vs_heuristic":   (RandomBot, HeuristicBot,    5_000 games),
    "mcts_vs_random":        (MCTS(100), RandomBot,       2_000 games),
    "mcts_vs_mcts_100":      (MCTS(100), MCTS(100),       1_000 games),
    "mcts_vs_mcts_1000":     (MCTS(1000), MCTS(100),       1_000 games),
    "mcts_vs_mcts_5000":     (MCTS(5000), MCTS(1000),       500 games),
}
```

The existing `profile_target.rs` runs 5000 random-vs-random games. Extend this.

### 11.2 Metrics

| Metric | What It Measures |
|---|---|
| Win rate vs. random | Baseline competence |
| Win rate as first/second | Turn-order fairness |
| Average game length | Aggression level |
| Action diversity | Not playing the same line every game |
| Time per move | Practical deployability |
| Iterations per second | Search efficiency |

### 11.3 Self-Play Curve

Plot Elo rating vs. iterations/search budget. This tells you the marginal value of more search:
- Flat curve → diminishing returns → stop increasing budget
- Steep curve → more search helps → increase budget

---

## 12. Implementation Roadmap

### Phase 1: Foundation (Week 1-2)
- [ ] Implement action generator wrapper (Rust trait for bot action enumeration)
- [ ] Implement determinization sampler (deck permutation given observed cards)
- [ ] Implement core ISMCTS with UCB1
- [ ] Implement simple rollout policy (uniform random)
- [ ] Integration test: bot can complete a game vs. random without errors

### Phase 2: Heuristics (Week 3-4)
- [ ] Add heuristic action ordering (play high-value cards first)
- [ ] Add heuristic rollout policy (weighted, not uniform)
- [ ] Add progressive widening for large action spaces
- [ ] Add phase-aware time allocation
- [ ] Benchmark: measure win rate vs. random

### Phase 3: Evaluation (Week 5-6)
- [ ] Implement truncated rollouts with evaluation function
- [ ] Tune evaluation weights via self-play
- [ ] Add root parallelization
- [ ] Benchmark: measure win rate vs. previous versions

### Phase 4: Production (Week 7-8)
- [ ] Integrate into web server (replace `AiDriver.js` random)
- [ ] Add logging and diagnostics
- [ ] Performance optimization (reduce state clone overhead)
- [ ] Add configuration API (difficulty levels: iterations budget)

### Phase 5: Advanced (Future)
- [ ] Neural network policy + value guidance
- [ ] Opening book generation
- [ ] Bayesian opponent modeling (re-determinizing ISMCTS)
- [ ] Cluster self-play for continuous improvement

---

## 13. Key Design Decisions Summary

| Decision | Choice | Rationale |
|---|---|---|
| Search algorithm | ISMCTS | Hidden deck order requires information-set handling |
| Action representation | Phase-aware categories | Reduces branching factor 3-5x vs. flat action list |
| Rollout policy | Weighted heuristic | Better than uniform, cheaper than neural |
| Progressive widening | `K * sqrt(N_parent)` | Essential for 50-200+ action branching |
| Evaluation | Truncated rollout + heuristic | Avoids 30-turn full rollouts |
| Time budget | 200ms–2000ms per action | Real-time web UI requirement |
| Integration | In-process Rust library | Fastest path; engine already has all needed primitives |
| Parallelism | Root parallel (start) | Simplest effective parallelization |

---

## 14. Learned Representations — Zero Manual Feature Engineering

This section explains how the neural network sees the game without anyone telling it what `blade`, `base_heart`, `need_heart`, phases, or any individual game variable means.

### 14.1 Core Principle: Raw Observation → Latent Understanding

The network does not receive feature vectors like `[blade, hand_size, energy_count, ...]`. Instead, it receives the raw game state structure and learns its own compressed representation of what matters.

Think of it like AlphaGo: the network was never told "capturing stones is good" or "corner territory is valuable." It learned those concepts from gradient descent on self-play data. Same approach here.

### 14.2 State Encoding as a Set of Entities

The game state is naturally a **set of entities** (cards in various zones) plus **scalar globals** (phase, turn number). Encode each entity as a feature vector. Order doesn't matter — use permutation-invariant architectures.

```
GAME STATE TENSOR ─────────────────────────────────────────────
│                                                              │
├─ GLOBAL SCALARS (1D vector, length ~20)                      │
│  phase (one-hot)      turn_number (normalized)               │
│  is_first_attacker    active_player                          │
│  turn_phase           game_result                            │
│  (All auto-encoded — no manual categorization needed)        │
│                                                              │
├─ ENTITY SETS (each entity → fixed-size embedding)            │
│                                                              │
│  ├─ MY STAGE (3 slots)                                       │
│  │   card_id → learned embedding                             │
│  │   orientation (active/wait)                               │
│  │   position (left/center/right)                            │
│  │   underlay_count                                          │
│  │                                                           │
│  ├─ OPPONENT STAGE (3 slots)                                 │
│  │   card_id → learned embedding                             │
│  │   orientation                                             │
│  │   position                                                │
│  │   underlay_count                                          │
│  │                                                           │
│  ├─ MY HAND (up to ~10 cards)                                │
│  │   card_id → learned embedding                             │
│  │   (order-agnostic — set encoder)                          │
│  │                                                           │
│  ├─ OPPONENT HAND (0 cards — hidden)                         │
│  │   But we DO encode: hand_size (1 scalar)                  │
│  │   The network learns to infer from hand_size +            │
│  │   public information what opponent might hold             │
│  │                                                           │
│  ├─ MY ENERGY ZONE (up to ~15 cards)                         │
│  │   card_id → learned embedding                             │
│  │   is_active (tapped/ready)                                │
│  │                                                           │
│  ├─ OPPONENT ENERGY ZONE                                     │
│  │   (same as above, fully visible)                          │
│  │                                                           │
│  ├─ MY WAITROOM / OPPONENT WAITROOM                          │
│  │   card_id → learned embedding per card                    │
│  │   (entire discard pile, no order)                         │
│  │                                                           │
│  ├─ MY SUCCESS ZONE / OPPONENT SUCCESS ZONE                  │
│  │   card_id → learned embedding per card                    │
│  │                                                           │
│  ├─ MY LIVE CARD ZONE / OPPONENT LIVE CARD ZONE              │
│  │   card_id → learned embedding per card                    │
│  │   heart_allocation (during live phase)                    │
│  │                                                           │
│  ├─ MY RESOLUTION ZONE (yell reveals)                        │
│  │   card_id → learned embedding per card                    │
│  │                                                           │
│  └─ MY DECK / OPPONENT DECK                                  │
│       deck_size (scalar)                                     │
│       top_card (only if publicly revealed via yell)          │
│                                                              │
└──────────────────────────────────────────────────────────────
```

**Key design rule:** Every card in every zone gets the same treatment — `card_id` → learned embedding. The network figures out which cards matter and in which contexts. A vanilla card in hand is just an embedding; a vanilla card on stage with active orientation is the same embedding but in a different slot — the network learns the difference from positional/contextual information.

### 14.3 Card Embedding

Every card is represented by its `card_id` (integer index into the 2280-card database) mapped to a learned embedding vector:

```python
# NOT 64. 64 is too small for 2280 cards with complex abilities.
card_embed = nn.Embedding(num_cards=2280, embedding_dim=128)
```

**Is 128 enough?** Rough rule of thumb: embedding_dim ≈ `(num_cards)^0.25 × 8` → 2280^0.25 ≈ 6.9, × 8 ≈ 55. That suggests ~64 minimum. But cards have complex ability text, not just scalar stats. 128 gives enough capacity to encode ability clusters, cost tiers, synergy tags, and still have room for the model to learn interactions. 256 is safer if compute budget allows. Going above 256 is wasteful — the bottleneck becomes the transformer layers, not the embedding.

**Many cards are reskins:** Correct. Cards like "高坂穂乃果 #001" and "高坂穂乃果 #002" often have identical or near-identical game function with different art. The embedding naturally collapses these — gradient descent pushes functionally identical cards toward the same region of embedding space because they produce the same game outcomes. The effective number of distinct card "types" is much smaller than 2280. The embedding table has capacity for 2280 but will only use what it needs.

**Energy cards are interchangeable:** Most energy cards are pure energy with no abilities and identical function. The embedding will converge them to the same vector. You could also deduplicate at input time: map all vanilla energy cards to a single "energy" token. But the network will figure this out on its own within a few thousand training games — it's a waste of parameters but not harmful.

**What if the embedding hasn't converged yet during early training?** That's fine. The ISMCTS search provides the training signal. Poor embeddings just mean slower convergence. The network initially relies more on the structural features (zone type, orientation, phase) and gradually refines card distinctions as it sees more data.

**You never tell it about:**
- `base_heart` — it learns some cards provide more hearts
- `blade` — it learns some cards draw more yell cards
- `cost` — it learns some cards are expensive
- `ability` — it learns some cards have powerful effects
- `need_heart` — it learns some live cards are harder to succeed
- `series`, `group`, `unit` — it may learn μ's vs. Aqours matter for synergy

### 14.4 Architecture: Set Transformer / Perceiver

Since the state is a set of entities with no natural ordering, use a **permutation-invariant architecture**:

```
┌──────────────────────────────────────────────────────────┐
│                    INPUT ENCODING                         │
│  Each entity: [card_embedding || position || orient ||   │
│                zone_type || extra_scalars]                │
│  → Linear projection to d_model                          │
└──────────────────────┬───────────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────────┐
│              SET TRANSFORMER ENCODER                      │
│  (Permutation-invariant self-attention over all entities) │
│                                                           │
│  Layer 1: Multi-head Self-Attention (entities × entities) │
│           "Which cards on stage matter most right now?"   │
│  Layer 2: Feed-Forward                                   │
│  Layer 3: Multi-head Self-Attention                      │
│           "How does my hand relate to opponent's stage?"  │
│  Layer 4: Feed-Forward                                   │
│  ... (4-8 layers total)                                  │
│                                                           │
│  Pool: Learned CLS token or average pooling               │
│  → produces a single latent state vector                  │
└──────────────────────┬───────────────────────────────────┘
                       │
        ┌──────────────┴──────────────┐
        │                             │
┌───────▼───────┐           ┌─────────▼──────────┐
│  VALUE HEAD   │           │   POLICY HEAD       │
│  (MLP)        │           │   (MLP → action     │
│  → ∈ [-1, 1]  │           │    logits)          │
│  "who wins?"  │           │  "which action?"    │
└───────────────┘           └────────────────────┘
```

**Why Set Transformer over CNN/GNN:**
- **CNN** assumes grid topology — doesn't fit card games (cards don't have spatial neighbors)
- **GNN** works but needs explicit edge definitions — too many assumptions to hand-code
- **Set Transformer** treats slots as unordered; the self-attention figures out relationships (combo between card in hand and card on stage) without being told

**Position encoding** is by zone membership, not spatial position:
```python
entity_features = concat([
    card_embedding,        # [128]
    one_hot(zone_type),    # [10] — stage/my, stage/opp, hand, energy, etc.
    position_in_zone,      # [3] — left/center/right for stage, or 0 for hand
    orientation,           # [2] — active/wait
    extra_count,           # [1] — underlay cards, etc.
])
```

### 14.5 Action Encoding

Actions must be encoded for the policy head to output. Each action is an **action type + target entity**. The network needs to understand not just which card to play, but *what kind of action* to take — and these are radically different.

```python
class ActionEncoder:
    """
    Every action becomes: [action_type_embed || target_1 || target_2 || ...]
    
    Instead of one-hotting all 200+ possible actions, decompose:
    - Action type: ~15 types — these are NOT just one-hot IDs, 
      they have structure the network learns
    - Target card indices: pointers into the entity set 
      (which card in hand/stage/etc.)
    - Extra parameters: position slot, etc.
    """
    def encode(self, action, entities):
        return concat([
            # Action type embedding (learned, ~16-dim)
            # The network learns that PlayMember and PlayEnergy 
            # are similar (both put cards on board), while 
            # PlayMember and Pass are very different
            action_type_embed(action.type),
            
            # Target: pointer to entity set index
            # "Play that card in hand position 3" → attends to 
            # the embedding of entity[hand_pos_3]
            pointer_to_entity(action.target, entities),
            
            # Extra: position on stage (which slot to play into)
            position_embed(action.position),
        ])
```

**How the network learns that PlayMember ≠ SetLiveCard:**

The action type embedding starts random. During training:

| Training Signal | What the Network Learns |
|---|---|
| "PlayMember puts a card on stage → next turn you have more blade → higher win rate" | The PlayMember embedding shifts toward "board development" region |
| "SetLiveCard puts a card in the live zone → during live phase it scores → if score > opponent you win" | The SetLiveCard embedding shifts toward "scoring" region |
| "Pass does nothing → your board stays the same → opponent develops → lower win rate" | The Pass embedding stays near the origin (no benefit detected) |

After training, dot products between action-type embeddings reflect functional similarity:
- `PlayMember · PlayEnergy` = high (both develop the board)
- `PlayMember · SetLiveCard` = medium (both put cards from hand to a zone, but different zones)
- `PlayMember · Pass` = low (opposite intent)
- `UseAbility · PlayMember` = medium (both spend resources for advantage)

**Action type is never a raw one-hot.** One-hot is uninformative — it says these 15 categories are equally different (all pairwise distance = sqrt(2)). The learned embedding lets the network discover structure: "activation abilities are similar to auto abilities" or "playing a member to center stage is different from playing to left wing."

**Pointer mechanism:** Instead of one-hotting card IDs, point to the entity slot. This makes the policy head **interact with the entity representation** directly. When considering "play card X," the policy head computes attention between the action type embedding and the card's entity embedding in the hand set. The network learns to favor playing cards whose embeddings have high blade (after it discovers blade matters) into stage slots that are empty.

**Concrete example of what the policy head computes internally:**

```
For each possible action a:
    score(a) = W · [ action_type_embed(a) 
                   || entity_embed(target_card) 
                   || position_embed(slot) 
                   || global_state_vector ]
    
    # global_state_vector comes from the Set Transformer encoder
    # of the full board — it represents "the overall situation"
    
softmax over all action scores → probability distribution
```

The network can distinguish "Play member A to center" from "Play member B to center" because the entity pointer feeds the correct card's embedding. It can distinguish "Play member A to center" from "UseAbility of member A" because the action type embedding is different. It can distinguish both from "SetLiveCard" because the action type + target zone combination produces a different score — and it learns this purely from which actions lead to wins.

### 14.6 Training Without Annotations

The neural network is trained entirely from self-play data:

```
Step 1: ISMCTS plays a game, logging every (state, search_policy, result)
Step 2: Collect training examples:
  - Input: raw state encoding
  - Target policy: visit counts from ISMCTS root node
  - Target value: +1 if MCTS player won, -1 if lost
Step 3: Train with:
  - Loss = cross_entropy(policy_pred, policy_target) 
        + MSE(value_pred, value_target)
        + L2 regularization
Step 4: After training, replace rollout policy with neural network
Step 5: Repeat — the network improves, ISMCTS gets better, data quality improves
```

**After enough iterations:**
- The value head predicts game outcomes at 60-70%+ accuracy
- The policy head assigns high probability to strong moves
- ISMCTS uses the policy head as UCB prior (AlphaZero style): `score = Q + c * P(s,a) * sqrt(N) / (1 + n)`, reducing branching factor by 5-10x
- Rollouts are replaced by the value head — no simulation needed at all (just search + evaluate)

### 14.7 What the Network Actually Learns (Emergent Concepts)

The network never sees variable names. But internally, individual neurons will learn to fire for:

| Emergent Concept | Neural Evidence |
|---|---|
| "I have more total blade" | Weighted sum of stage card embeddings consistently correlates with win rate |
| "My energy is low this turn" | Energy zone embedding cluster shifts |
| "This live card needs specific hearts" | Attention between live card slot and heart vectors |
| "Opponent has card advantage" | Sum of opponent hand + deck size embeddings |
| "This ability is game-winning" | Specific card embedding + activated flag → high value delta |
| "I should pass vs. play something" | Policy head output skews from the latent bottleneck |

You can probe the network post-training and find these concepts in hidden layer activations, but you never needed to program them. The network discovered them because they're predictive of winning.

### 14.8 Concrete Example: The Network Discovers "Blade"

Without anyone ever telling the network what `blade` is:

1. Initial random games: ISMCTS tries random actions. Sometimes it wins, sometimes it loses.
2. Training signal: "When you have card X on stage, you won that game more often."
3. The network's card embedding for high-blade cards starts to cluster in embedding space because they correlate with positive outcomes in similar board states.
4. The attention mechanism learns: "When I see card embeddings from this cluster on my stage positions AND the phase is approaching Live, I should predict a higher win probability."
5. Multiple cards with the same blade value end up with similar embeddings (by function, not by programmer decree).

If you were to visualize the embedding space with PCA, you'd see cards naturally cluster by role — even without labels:

```
        PCA2
         ↑
         │   (high-blade members)
         │      ● ● ●
         │   ● ● ● ● ●
         │      ● ●
         │
         │      ● ● ●  (low-cost utility cards)
         │   ● ●   ●
         │      ●
         │
         │  ● ● ●   (energy cards)
         │ ●   ● ●
         │
         │   ● ●    (live cards)
         │  ●   ● ●
         │
         └──────────────────────────→ PCA1
```

No manual feature engineering. No annotations. Just card_id → embedding → learn.

### Key Takeaway

> **You don't annotate the game variables. You encode the raw structure (card in this zone, card in that zone, phase, turn count) and let gradient descent discover what matters.**

The network will learn blade, heart, cost, ability strength, synergy, tempo, card advantage, and everything else needed to play well — all from watching ISMCTS self-play and predicting outcomes.

---

## 15. References

1. Coulom, R. (2006). "Efficient Selectivity and Backup Operators in Monte-Carlo Tree Search." *CG 2006*.
2. Kocsis, L. & Szepesvari, C. (2006). "Bandit based Monte-Carlo Planning." *ECML 2006*.
3. Browne, C. et al. (2012). "A Survey of Monte Carlo Tree Search Methods." *IEEE TCIAIG*.
4. Cowling, P. et al. (2012). "Information Set Monte Carlo Tree Search." *IEEE TCIAIG*.
5. Silver, D. et al. (2016). "Mastering the game of Go with deep neural networks and tree search." *Nature*.
6. Silver, D. et al. (2018). "A general reinforcement learning algorithm that masters chess, shogi, and Go through self-play." *Science*.
7. Goodman, J. (2019). "Re-determinizing Information Set Monte Carlo Tree Search in Hanabi." *IEEE CoG 2019*.
8. Świechowski, M. et al. (2021). "Monte Carlo Tree Search: A Review of Recent Modifications and Applications." *arXiv:2103.04931*.
9. Vaswani, A. et al. (2017). "Attention Is All You Need." *NeurIPS 2017*.
10. Lee, J. et al. (2019). "Set Transformer: A Framework for Attention-based Permutation-Invariant Neural Networks." *ICML 2019*.
11. DeepMind OpenSpiel: https://github.com/deepmind/open_spiel
12. Rabuka Reloaded engine docs: `engine/rules/rules_1_06.txt`
