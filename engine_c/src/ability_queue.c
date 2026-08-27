#include "rabuka.h"
#include <string.h>

int rb_queue_push(RbAbilityQueue *q, int card_id, int ability_idx) {
    if (!q || q->n_entries >= RB_QUEUE_DEPTH) return 0;
    q->entries[q->n_entries].card_id = card_id;
    q->entries[q->n_entries].ability_idx = ability_idx;
    q->entries[q->n_entries].cost_paid = 0;
    q->entries[q->n_entries].effect_started = 0;
    q->n_entries++;
    return 1;
}
void rb_queue_clear(RbAbilityQueue *q) { if (q) { memset(q, 0, sizeof(*q)); } }
int rb_queue_has_pending(const RbAbilityQueue *q) { return q && q->n_entries > 0; }

int rb_use_limit_reached(RbAbilityQueue *q, int card_id, int ability_idx, int limit, int cur_turn) {
    if (!q || limit <= 0) return 0;
    if (q->use_turn != cur_turn) return 0;
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++) if (q->use_keys[i] == key) return q->use_counts[i] >= limit;
    return 0;
}
void rb_record_use(RbAbilityQueue *q, int card_id, int ability_idx, int cur_turn) {
    if (!q) return;
    if (q->use_turn != cur_turn) { q->n_uses = 0; q->use_turn = cur_turn; }
    int key = (card_id << 4) | (ability_idx & 0xF);
    for (int i = 0; i < q->n_uses; i++) if (q->use_keys[i] == key) { q->use_counts[i]++; return; }
    if (q->n_uses < RB_USE_TRACK) { q->use_keys[q->n_uses] = key; q->use_counts[q->n_uses] = 1; q->n_uses++; }
}
