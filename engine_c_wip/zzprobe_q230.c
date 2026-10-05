#include "rabuka.h"
#include <stdio.h>

static void dump_cond(const Condition *c, int depth) {
    if (!c) return;
    for (int i = 0; i < depth; i++) printf("  ");
    printf("Condition %p\n", (void *)c);
    for (int i = 0; i < depth; i++) printf("  ");
    printf("  type=%s", c->type ? c->type : "(null)");
    if (c->location)     printf(" location=%s", c->location);
    if (c->target)       printf(" target=%s", c->target);
    if (c->card_type)    printf(" card_type=%s", c->card_type);
    if (c->operator)     printf(" operator=%s", c->operator);
    if (c->comparison_type) printf(" comparison_type=%s", c->comparison_type);
    if (c->count >= 0)   printf(" count=%d", c->count);
    if (c->value >= 0)   printf(" value=%d", c->value);
    printf("\n");
    for (int i = 0; i < depth; i++) printf("  ");
    printf("  extras:");
    for (int i = 0; i < c->n_extra; i++) printf(" [%s=%s]", c->extra_k[i], c->extra_v[i]);
    printf("\n");
    for (int i = 0; i < c->n_and; i++) { printf("  AND:\n"); dump_cond(c->and_[i], depth + 2); }
    for (int i = 0; i < c->n_or; i++)  { printf("  OR:\n");  dump_cond(c->or_[i],  depth + 2); }
    if (c->child) { printf("  CHILD:\n"); dump_cond(c->child, depth + 2); }
}

int main(void) {
    if (rb_load("src") != 0) return 1;
    int id = rb_find_card_by_no("PL!N-bp5-007-R＋");
    printf("card id=%d\n", id);
    int nab = rb_card_num_abilities((uint32_t)id);
    for (int a = 0; a < nab; a++) {
        Ability ab;
        if (!rb_decode_card_ability((uint32_t)id, a, &ab)) continue;
        printf("\n=== ability[%d] triggers=%s\n", a, ab.triggers ? ab.triggers : "?");
        if (ab.effect) {
            printf("  effect action=%s text=[%s]\n",
                   ab.effect->action ? ab.effect->action : "(null)",
                   ab.effect->text ? ab.effect->text : "");
            printf("  has_condition=%d\n", ab.effect->has_condition);
            dump_cond(ab.effect->condition, 2);
        }
        rb_free_ability(&ab);
    }
    return 0;
}
