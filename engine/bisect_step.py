# -*- coding: utf-8 -*-
path = 'src/bot/strategy_v3.rs'
src = open(path, encoding='utf-8').read()

start = src.find('pub fn choose_live_set_action_v3(')
nxt = src.find('\npub fn ', start + 10)
new_ls = '''pub fn choose_live_set_action_v3(
    gs: &GameState,
    actions: &[Action],
    db: &CardDatabase,
    policy: &V2Policy,
    plan: &V3Plan,
) -> Action {
    // BISECTED back to pure v2 delegation (rush-window relaxation kept).
    // The wrapper layers (ammo hold, concede, member dumps) each measured
    // NEGATIVE in 30s arena ablations and are removed until re-proven.
    if plan.in_rush_window(gs.turn_number) {
        let relaxed = V2Policy {
            mc_trials: policy.mc_trials,
            gamble_floor: (policy.gamble_floor * 0.5).max(0.04),
            urgent_gamble_floor: policy.urgent_gamble_floor * 0.5,
        };
        return strategy_v2::choose_live_set_action_v2(gs, actions, db, &relaxed);
    }
    strategy_v2::choose_live_set_action_v2(gs, actions, db, policy)
}
'''
src = src[:start] + new_ls + src[nxt + 1:]

open(path, 'w', encoding='utf-8').write(src)
print('live-set replaced')
