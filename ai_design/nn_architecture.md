# NN Architecture Design

## Current State: What's Wrong

### Problem 1: Blind Card Embeddings (91% of params doing nothing)

```
Current: card_embed[2400 × 128] = 307,200 floats
```

Each card gets a free-floating 128-dim vector that must learn everything from scratch:
- That "blade = 1" is weak
- That "heart06" is a purple heart
- That this card belongs to μ's, not Aqours

The model has no shared structure between similar cards. A card with blade=5 and a card with blade=1 are equally distant in embedding space — the model must discover blade is meaningful through 1000s of games.

**With explicit features**, these are just numbers the model can use immediately:

```
card = [type_onehot(3), blade/15, cost/10, heart_counts(7), blade_heart_counts(7),
        has_ability, ability_count/5, group_onehot(6)]
     → ~26 floats, 0 learned params
```

The group embedding (6 × 8 = 48) is learned — but tiny. Everything else is ground truth.

### Problem 2: Abilities Invisible

For `UseAbility(card_id)`, we encode card_id but NOT:
- Which ability slot (index)
- What the ability costs
- What it targets
- Which effect type (draw, search, buff, etc.)

A card with "draw 2 cards" ability and a card with "deal 3 damage" ability look identical.

Fix: encode `ability_index`, effect type one-hot, activation cost.

### Problem 3: No Card Position Structure

Stage positions are individual zone slots (good), but there's no structure relating them. The left/center/right positions have no positional encoding relative to each other.

Fix: already partially done with position features. Could add relative position encoding.

### Problem 4: Action Space is O(N) Forward Passes

Each legal action requires a separate forward pass through the action MLP. With 50-200 legal actions, this is the bottleneck (~119 steps/s).

Fix: batch action evaluation (already partially done — state trunk computed once). For training in PyTorch, this is fully batched. For Rust inference, we could use SIMD or just accept the cost.

---

## Proposed Architecture

### Card Feature Vector (26 floats, computed from DB)

```
[0:2]   card_type one-hot: Member, Live, Energy
[3]     blade / 15.0
[4]     cost / 10.0 (0 for non-members)
[5:12]  base_heart counts (heart01..heart06 + wildcard), each / 3.0
[12:19] blade_heart counts (b_heart01..b_heart06 + wildcard), each / 3.0
[19]    has_ability (0 or 1)
[20]    num_abilities / 3.0
[21]    score / 10.0 (live cards)
[22:25] group one-hot (μ's, Aqours, 虹ヶ咲, Liella!, 蓮ノ空, other)
```

Total: 25 explicit features + group projection → learned embedding not needed for card stats.

### State Encoding (revised)

The `EncodedState` remains zone-aware, but each card entity is [card_feats(26) + zone_embed(16) + pos_features(4)] = 46 floats instead of the old 148.

| Zone | Entities | Dim per entity | Total |
|------|----------|----------------|-------|
| My hand | sum-pool | 26 | 26 |
| My stage ×3 | 3 × (26+4) | 30 | 90 |
| My energy | sum-pool | 26 | 26 |
| My waitroom | sum-pool | 26 | 26 |
| My live | sum-pool | 26 | 26 |
| My success | sum-pool | 26 | 26 |
| Opp stage ×3 | 3 × (26+4) | 30 | 90 |
| Opp energy | sum-pool | 26 | 26 |
| Opp waitroom | sum-pool | 26 | 26 |
| Opp live | sum-pool | 26 | 26 |
| Opp success | sum-pool | 26 | 26 |
| **Globals** | | | **28** |
| **Total** | | | **520** |

From ~1835 to ~520. The state is smaller AND more informative.

### Action Encoding (revised)

```
action_enc = [
    action_type_embed(16),
    card_features(26),              # target card's actual stats
    target_zone_embed(16),
    position_features(4),
    ability_index / 3.0,            # which ability slot (0 if N/A)
    effect_type_onehot(20),         # what the ability does
    activation_cost / 10.0,         # ability's additional cost
]
```

Total: ~85 dim (was 163). Smaller + informative about abilities.

### Network Architecture

```
Input: state_vector [520]

h_state = relu(W1_state · state_vector + b1)  [256]

For each action:
  score = policy_head(relu(h_state + W1_action · action_enc(a)))  [1]
  → softmax over all legal actions

Value: V(s) = tanh(W_value · h_state + b_value)  [1]
```

### Parameter Count

| Component | Params |
|-----------|--------|
| group_embed (6 groups × 8) | 48 |
| zone_embed (15 × 16) | 240 |
| action_type_embed (25 × 16) | 400 |
| card_feat_proj (64 × 26) | 1,664 |
| W1_state (256 × 520) | 133,120 |
| b1 | 256 |
| W1_action (256 × 85) | 21,760 |
| policy_head (256 × 1) | 257 |
| value_head (256 × 1) | 257 |
| **Total** | **~158K** |

Down from 822K. And 0 of those are blind card embeddings.

### Training (PPO)

Same PPO pipeline as designed:
1. Collect trajectories (Rust)
2. Compute GAE (Python)
3. PPO update with clipped surrogate + value loss + entropy bonus
4. Save weights
5. Rust loads weights, does inference for next data collection

### Inference (Rust)

The forward pass uses no card lookup table — just feature vectors computed from the CardDatabase. The only learned weights are:
- Small embedding tables (zone, action_type, group)
- Projection + MLP weights (~158K)

Weights file size: ~630 KB (vs 1.3 MB for card embeddings alone).

---

## Implementation Plan

1. Build `card_features()` in Rust — reads `CardDatabase`, outputs [26] per card
2. Build `card_features()` in Python — reads `cards.json`, identical output
3. Update `encoding.rs` — smaller entity size, ability info in action encoding
4. Update `neural.rs` — replace `card_embed` LUT with feature computation
5. Update `train_ppo.py` — load card DB, use feature-based model
6. Run full cycle
