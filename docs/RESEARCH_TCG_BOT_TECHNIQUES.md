# TCG Bot Techniques Research — Candidates for v9 Strength

Survey of published/competitor TCG bot work, mapped against what `rabuka_reloaded`
already does. All arena claims in this repo require the paired `bot_arena` A/B
protocol (docs/V8_IMPROVEMENT_LOG.md §3 of V8_NEXT_METHODS).

Sources: Hearthstone AI Competition (CoG/IA-IAAI, SabberStone), arXiv:2303.04096
(ByteRL/LOCM), MTG-Causal-RL (arXiv:2605.06066), MageZero, STS-RL, Tales of
Tribute AI Competition (arXiv:2305.08234), STS-RL variants.

## A. Search / lookahead architecture

1. **Dynamic lookahead, EV of a leaf heuristic** (HL/CoG winners). v7 already is
   ISMCTS-flavored; but v9's Main is v7's and its live set is v8's flat model.
   Candidate: apply v7-style rollout at the *live-set* choice too is what
   `V9_LIVE_ROLLOUT=v7` tested (-9.4pp, rejected). Do NOT invert that finding
   without new data.
2. **Pruned BFS / breadth over fixed-width beams** (Tales of Tribute winner).
   Candidate: fixed-depth BFS over live-set×heart-commitment joint decisions,
   evaluating with the calibrated leaf, instead of sampling `V8_LIVE`.
3. **MetaStone pattern: search to end of current move, no opponent model.**
   Cheaper than full rollout; candidate for the Main phase where T3/T4 races
   are where OppModel under-rates leapfrogs (V8_NEXT_METHODS §5.5).

## B. Opponent modeling

4. **Bigram next-card prediction** (IPMU 2018, up to 95% on held-out moves).
   Not implemented anywhere here; OppModel is a static prior. Candidate:
   maintain an observed-hand bigram table per turn from the replay/outcomes and
   update OppModel's decklist prior at each draw. Plausibly the largest untested
   B-category item.
5. **Deck identification from observed cards** (co-occurrence / anchor decks).
   Loveca has known lists (ミウミ guide decks). OppModel could switch templates
   after N observed cards. Needs a deck-metadata sidecar; check cards.json tags.
6. **Opponent intent/category conditioning**: when their last-set live failed,
   their distribution of next-set lives is biased; conditional p_pass table.
   V8's `V8_OPP_SCORE_BIAS` probes were null/small — explains why: bias shifts
   a scalar, intent is categorical.

## C. Evaluation function

7. **Evolutionary tuning of the weighted eval** (CoEA, 2nd place 2018
   Hearthstone; also Tales of Tribute). v7's four-term score is the project's
   "SabberStoneBasicAI". Candidate: CMA-ES over (V7_ABILITY_WEIGHT,
   V7_AMMO_WEIGHT, V7_BLADE_WEIGHT, coverage, placement) on a 3-seed
   paired arena, 400 games/side, instead of one-knob-at-a-time arms.
8. **Terminal short-circuit before weighted terms** (SabberStoneBasicAI.Rate).
   Check does v9's leaf evaluate a won game correctly at non-leaf positions;
   `TERMINAL_WIN` exists as a knob — confirm it is on by default.
9. **Board presence as discrete term** (same source). v7 has `V7_PRESENCE`
   knob measured flat (-0.4pp). Document as tried.
10. **Causal factor-credit advantage (CGFA, MTG-Causal-RL)**: per-loss-bucket
    credit trace. This repo already decomposes losses (§28.31) and maps buckets
    to rules — the CGFA "SCM over strategic variables" is §28.31's table. The
    untried half: *train the value net on per-bucket targets*, not win/loss.
11. **MuZero TD-blend value target** (MageZero: `y = λ·y_{t+1} + (1-λ)·h_t`).
    The repo's value_v2/vsv9 bins were trained how? Check `train_ppo.py`; if the
    target is pure terminal outcome, blending with MCTS root value is a real
    candidate (training change, cheap to try, big potential).

## D. Behavioral cloning / RL training loop

12. **BC → PPO warm start** (STS-RL). The repo HAS bc_1500/bc_3000/bc_smoke and
    policy_rl1 — check whether shipping policy (value_v2/vsv9) used it. If not,
    that is a standard, literature-backed strength gain for policy nets.
13. **Curriculum over opponent strength** (STS-RL Defect > Watcher pattern):
    train vs v6, then v7, then v9. Check train_ppo self-play opponent schedule.
14. **MaskablePPO-style action masking**: verify the PPO loop masks illegal
    actions (a known bug source: probability mass on illegal moves). Audit
    `training/train_ppo.py`.
15. **MTG-Causal-RL causal world-model PPO**: expensive; reference only.

## E. Determinization / hidden info

16. **40 determinizations × 250 sims** (Cowling MTG; standard in CIG work).
    v7's determinization/ismcts counts: compare against 40×250 and the
    1×N vs N×1 tradeoff. Quick config-level A/B — cheap to run, literature
    says the ratio matters at low budgets.
17. **Bucketing chance nodes in determinized MCTS** (COC-2019 paper, "Enhancing
    MCTS for Hearthstone"): chance node = blade flip; bucket by outcome
    category rather than sampling per-node. v8's flip sampling at 192 samples
    is the same shape; test bucketing vs i.i.d. sampling (both null so far —
    §28.20 — so LOW priority).

## F. Stochastic decision heuristics

18. **Heuristic EV over playouts instead of UCT** (SabberStoneBasicAI): tried
    in spirit (`V7_LEAN`, `v7-forecast`) — mostly null. Document as tried.
19. **Dynamic playstyle / preference vector** (per-preset preferences): run the
    same code as aggro and control profiles. v7 has no playstyle term; §28.15
    says "doctrine does not transfer to this decklist" — a *deck-matched*
    playstyle knob is a real candidate: add an aggro/control tempo scalar to
    the prior decklist template instead of the universal doctrine.
20. **Archetype prediction early** (predict opponent's deck from first 3-5
    cards; switch argmax policy table per predicted archetype). Related to #4,
    but cheaper: discrete template switch, not model update.

## G. Game-specific micro-techniques from Loveca guides (not yet implemented)

21. **Yell selection (8.3.14)** — V8_NEXT_METHODS §6.8 lists as untested. The
    choice of which members are active is real. Candidates: send blades-with-
    bad-density to wait; keep heart-dense members for wait pool. Implement as a
    live-set post-pass over `summoned` categories, gate with env, arena it.
22. **Initialize Aim (8.4.14) / Swap to Commander (8.4.15)** — member order in
    the live area is a choice nobody models. Audit which cards in cards.json
    reference them; if common in the current deck, this is an implement target.
23. **Baton touch cost accounting (9.6.2.3.2)** — v7's Main models cost; verify
    it models the reduction from the *previous turn's* on-stage member, else it
    over-estimates the cost of every baton.
24. **Position (センター/左サイド) scoring** — 65/18 cards care which column a
    member is in; v8 scores stage areas as interchangeable (§6.12). Candidate:
    tiny scalar on center vs side.
25. **Baton タッチ直後の Energy 残量をリフレッシュ前提で評価** — decklist-side,
    skip.
26. **Mulligan policy asymmetry** (`V9_MULLIGAN=v8` was -4.4pp; rejected).
27. **先攻コワレヤスキ + 余剰ハート消し (ミウミ X post 2026-05)** — yell overflow
    interaction is order-sensitive; confirm engine tests model it.

## H. Meta / infrastructure

28. **Hold-out seed for selection** (§22): selection is fitted to seeds
    11/23/31; add seed 41+ as selection lockout. Process change, zero code.
29. **Second baseline** (§23): every claim is v9-vs-v7. Run the same arm
    against v6 and v7rollout before shipping.
30. **Per-card play-rate telemetry for Main** (§25): the 21.4% live-agreement
    insight came from counting. Add the same counter to Main phase.
31. **A/B against `queen` (net tournament winrate?)** — ask: is there a
    tournament/matchmaking harness? If not, the metric is only self-play.

## Minimum-cost highest-value queue (my read after the sweep)

1. #7 evolutionary tuning of v7's term vector (CMA-ES outer loop; uses existing
   `V7_*` knobs).
2. #11 MuZero-style TD-blend value target (one training-script change; check
   current target first).
3. #12 verify BC→PPO pipeline was used for the shipping policy; if not, run it.
4. #4 bigram opponent-next-card model (needs a small table build from replays;
   then an OppModel swap knob).
5. #16 determinization-ratio config sweep (one-line knob).
6. #21 yell selection pass (engine already offers the choice; v9 never uses it).
7. #2 pruned-BFS over live×heart at the decision point (v7 live replaced by
   search; measure against `V9_LIVE_ROLLOUT=v7`'s -9.4pp reference).

Tried-and-flat (do not retry; see log): scaling p_place (§28.20), tie-mass
correction (§28.20), `p-floor`, `V8_OPP_SCORE_BIAS`, `V8_MAX_LIVES` width,
`V7_PRESENCE`, coverage-gradient on Main, `v7-*` scalar probes (mostly null),
`allow 2 lives` (354 vs 364, worse).

## Loveca guide debt I noticed re-reading sources

- BOT_STRATEGY §1 assumes ~1 placement/live-phase from T2; shipped v9 at 0.42
  still under-converts. Guide-consistent target is a per-card passable-icon
  audit: if v9's Main is paying for cards whose lives it cannot pass, that's
  the exact §6.1 claim. Verify with `V6_TRACE`/arena trace before touching code.
