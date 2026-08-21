# -*- coding: utf-8 -*-
path = 'src/bot/strategy_v3.rs'
lines = open(path, encoding='utf-8').read().split('\n')

helpers = '''/// Allocate one life's requirements from a remaining heart pool.
/// Mirrors 8.3.15: specific colors first, wildcards (All/BAll) cover
/// deficits, grey bucket takes colorless + leftovers. Returns false if the
/// life can't be satisfied from what remains.
fn allocate_life(pool: &mut HeartAcc, need: &HeartAcc) -> bool {
    let mut wildcard = pool[10] + pool[7];
    let mut specific_surplus = 0i32;
    for c in 1..=6 {
        let deficit = need[c] - pool[c];
        if deficit > 0 {
            wildcard -= deficit;
            if wildcard < 0 {
                return false;
            }
        } else {
            specific_surplus += -deficit;
        }
    }
    let leftover = specific_surplus + wildcard.max(0) + pool[0];
    if leftover < need[0] {
        return false;
    }
    true
}

fn confirm_live_set(actions: &[Action]) -> Action {
    actions
        .iter()
        .find(|a| a.action_type == game_setup::ActionType::ConfirmLiveCardSet)
        .or_else(|| actions.first())
        .cloned()
        .expect("live set actions non-empty")
}'''

# find the doc comment line index
idx = None
for i, l in enumerate(lines):
    if 'Classification of every hand card' in l:
        idx = i
        break
assert idx is not None
# walk back to include preceding doc-comment lines (/// ...)
while idx > 0 and lines[idx - 1].strip().startswith('///'):
    idx -= 1
lines.insert(idx, helpers)
open(path, 'w', encoding='utf-8').write('\n'.join(lines))
print('inserted at line', idx)
