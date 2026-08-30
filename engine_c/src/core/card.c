#include "rabuka.h"
#include "gen_data.h"
#include <stdlib.h>
#include <string.h>
extern uint16_t *g_card_ability_pairs;

static uint16_t le16p(const unsigned char *p) {
    return (uint16_t)p[0] | ((uint16_t)p[1] << 8);
}

int rb_decode_card_by_index(uint32_t i, Card *out) {
    const unsigned char *r = rb_card_record(i);
    if (!r || !out) return 0;
    memset(out, 0, sizeof(*out));

    out->card_no_idx = le16p(r + 0);
    out->name_idx    = le16p(r + 2);
    out->series_idx  = le16p(r + 4);
    out->group_idx   = le16p(r + 6);
    out->unit_idx    = le16p(r + 8);
    out->img_idx     = le16p(r + 10);
    out->product_idx = le16p(r + 12);
    out->rare_idx    = le16p(r + 14);
    out->ability_idx = le16p(r + 16);
    out->type_flags  = r[18];
    out->cost        = r[19];
    out->blade       = r[20];
    out->score       = r[21];
    out->num_base    = r[22];
    out->num_blade   = r[23];
    out->num_need    = r[24];

    out->has_special = (out->type_flags & 0x04) ? 1 : 0;
    out->name = (char *)rb_card_string(out->name_idx);

    const unsigned char *h = r + 25;
    uint32_t total = (uint32_t)out->num_base + out->num_blade + out->num_need;
    out->n_hearts = 0;
    for (uint32_t k = 0; k < total && out->n_hearts < RB_MAX_HEARTS; k++) {
        out->heart_color[out->n_hearts] = *h++;
        out->heart_count[out->n_hearts] = *h++;
        out->n_hearts++;
    }
    if (out->has_special && out->n_hearts < RB_MAX_HEARTS) {
        out->special_color = *h++;
        out->special_count = *h++;
    }

    if (out->ability_idx != 0xFFFF) {
        out->ability = malloc(sizeof(Ability));
        if (out->ability) {
            if (!rb_decode_ability(out->ability_idx, out->ability)) {
                free(out->ability); out->ability = NULL;
            }
        }
    }
    return 1;
}

void rb_free_card(Card *c) {
    if (!c) return;
    if (c->ability) { rb_free_ability(c->ability); free(c->ability); c->ability = NULL; }
}

/* Multi-ability support — uses RBKA_CARD_ABILITY_PAIRS (card_no string idx → ability idx).
   The pairs table's card_no idx is into abilities_strings (RBKA_STRINGS_OFFSETS),
   while the card's card_no_idx is into g_card_strings (cards.bin). They are
   different string tables, so we compare the actual string content, not the
   numeric index. Mirrors Rust CardLoader::build_abilities_map_shared. */
int rb_card_num_abilities(uint32_t card_idx){
    const unsigned char *r = rb_card_record(card_idx);
    if(!r) return 0;
    uint16_t card_no_idx = le16p(r+0);
    const char *card_no = rb_card_string(card_no_idx);
    if(!card_no) return 0;
    int cnt=0;
    for(uint32_t i=0;i<RBKA_NUM_CARD_ABILITY_PAIRS*2; i+=2){
        const char *pair_no = rb_get_string(g_card_ability_pairs[i]);
        if(pair_no && !strcmp(pair_no, card_no)) cnt++;
    }
    return cnt;
}
int rb_card_get_ability_idx(uint32_t card_idx, int n, uint32_t *out){
    const unsigned char *r = rb_card_record(card_idx);
    if(!r || !out) return 0;
    uint16_t card_no_idx = le16p(r+0);
    const char *card_no = rb_card_string(card_no_idx);
    if(!card_no) return 0;
    int cur=0;
    for(uint32_t i=0;i<RBKA_NUM_CARD_ABILITY_PAIRS*2; i+=2){
        const char *pair_no = rb_get_string(g_card_ability_pairs[i]);
        if(pair_no && !strcmp(pair_no, card_no)){
            if(cur==n){ *out = g_card_ability_pairs[i+1]; return 1; }
            cur++;
        }
    }
    return 0;
}
int rb_decode_card_ability(uint32_t card_idx, int n, Ability *out){
    uint32_t ab_idx;
    if(!rb_card_get_ability_idx(card_idx, n, &ab_idx)) return 0;
    return rb_decode_ability(ab_idx, out);
}

/* Card classification — mirrors Rust Card::is_live / is_energy.
   A card is "live" (song) when it has no member hearts and no play cost and
   no blade; it is "energy" when it has a cost but no hearts and no blade. The
   database record carries these directly (num_base+num_blade+num_need = heart
   count, cost, blade), so we read them without a full decode. */
int rb_card_is_live(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    if (!r) return 0;
    /* compile_cards.py: type_flags = ctype | has_special<<2 | has_cost<<3 |
       has_score<<4; ctype low 2 bits: 0=Member, 1=Live, 2=Energy. */
    return (r[18] & 0x03) == 1;
}
int rb_card_is_energy(int card_id) {
    const unsigned char *r = rb_card_record(card_id);
    if (!r) return 0;
    return (r[18] & 0x03) == 2;
}
