/* audit_actions.c — enumerate the real effect vocabulary present in the
 * compiled bytecode: action verbs, condition usage, and structural/compound
 * nesting keys. Drives which handlers engine.c must implement (PROGRESS.md B).
 *
 * Build: make audit   (or cc -std=c11 -O2 -Iinclude -o audit src/OBJ tools/audit_actions.c)
 */
#include "rabuka.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static char *xstrdup(const char *s) {
    if (!s) return NULL;
    size_t n = strlen(s) + 1; char *p = malloc(n);
    if (p) memcpy(p, s, n); return p;
}

/* hash map of string -> count */
typedef struct { char *k; int c; } Ent;
static Ent *g_act = NULL; static size_t g_act_n = 0, g_act_cap = 0;
static Ent *g_extra = NULL; static size_t g_extra_n = 0, g_extra_cap = 0;
static long g_effects = 0, g_has_condition = 0;

static unsigned long hash(const char *s) {
    unsigned long h = 1469598103934665603UL;
    for (; *s; s++) { h ^= (unsigned char)*s; h *= 1099511628211UL; }
    return h;
}
static void bump(Ent **m, size_t *n, size_t *cap, const char *k) {
    if (!k) return;
    size_t cap0 = *cap;
    if (cap0 == 0) { cap0 = 256; *m = malloc(cap0 * sizeof(Ent)); *cap = cap0; }
    unsigned long h = hash(k) & (cap0 - 1);
    for (;;) {
        if ((*m)[h].k == NULL) {
            (*m)[h].k = xstrdup(k); (*m)[h].c = 1; (*n)++; return;
        }
        if (!strcmp((*m)[h].k, k)) { (*m)[h].c++; return; }
        h = (h + 1) & (cap0 - 1);
    }
}

/* recurse the decoded effect tree */
static void walk(AbilityEffect *e) {
    if (!e) return;
    g_effects++;
    if (e->action) bump(&g_act, &g_act_n, &g_act_cap, e->action);
    if (e->has_condition) g_has_condition++;
    for (int i = 0; i < e->n_extra; i++) bump(&g_extra, &g_extra_n, &g_extra_cap, e->extra_k[i]);
    for (int i = 0; i < e->n_child; i++) walk(e->child[i]);
}

static int cmp_ent(const void *a, const void *b) {
    return ((const Ent*)b)->c - ((const Ent*)a)->c;
}

int main(void) {
    if (rb_load("src") != 0) { fprintf(stderr, "rb_load failed\n"); return 1; }
    uint32_t na = rb_num_abilities();
    for (uint32_t i = 0; i < na; i++) {
        Ability a;
        if (!rb_decode_ability(i, &a)) continue;
        if (a.cost) walk(a.cost);
        if (a.effect) walk(a.effect);
        rb_free_ability(&a);
    }

    printf("=== ACTION VERBS (%zu distinct, %ld effect nodes) ===\n", g_act_n, g_effects);
    qsort(g_act, g_act_n, sizeof(Ent), cmp_ent);
    for (size_t i = 0; i < g_act_n; i++)
        printf("%6d  %s\n", g_act[i].c, g_act[i].k ? g_act[i].k : "(null)");

    printf("\n=== EXTRA FIELDS (%zu distinct) ===\n", g_extra_n);
    qsort(g_extra, g_extra_n, sizeof(Ent), cmp_ent);
    for (size_t i = 0; i < g_extra_n; i++)
        printf("%6d  %s\n", g_extra[i].c, g_extra[i].k ? g_extra[i].k : "(null)");

    printf("\ncondition-flagged effect nodes: %ld\n", g_has_condition);
    printf("abilities with use_limit>=0: ");
    int ul = 0;
    for (uint32_t i = 0; i < na; i++) {
        Ability a; if (!rb_decode_ability(i, &a)) continue;
        if (a.use_limit > 0) ul++;
        rb_free_ability(&a);
    }
    printf("%d\n", ul);

    rb_unload();
    return 0;
}
