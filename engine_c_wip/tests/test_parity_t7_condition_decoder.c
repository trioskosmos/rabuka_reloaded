/* tests/test_parity_t7_condition_decoder.c
 *
 * Cluster T7 parity test for the GENERATED condition decoder.
 *
 * WHAT RUST ASSERTS, AND WHERE
 * ----------------------------
 * `condition_decoder_gen.rs::decode_condition_field` names 90 distinct keys.
 * `core/card.rs::ConditionCommon` (card.rs:3189-3248) carries five of them that
 * the C twin did not, so each used to fall through the C catch-all, get
 * rb_note_decode_fallback()'d, and be DROPPED:
 *
 *   any_location          condition_decoder_gen.rs:137  card.rs:3198
 *   activated_this_turn   condition_decoder_gen.rs:25   card.rs:3242
 *   paid_cost_this_turn   condition_decoder_gen.rs:84   card.rs:3248
 *   text                  condition_decoder_gen.rs:234  card.rs:3218
 *   trigger_event         condition_decoder_gen.rs:25   card.rs:3223
 *
 * `text` and `trigger_event` are `#[cfg(feature = "debug_conditions")]` in Rust,
 * and debug_conditions IS listed under `default` in engine/Cargo.toml, so an
 * ordinary build has them.
 *
 * Corpus counts over the 985 shipped abilities (tools/audit_t7_wire.c, `make
 * t7-wire`): any_location 18 nodes / 18 abilities, activated_this_turn 3 / 1,
 * paid_cost_this_turn 1 / 1, text 680 / 504, trigger_event 131 / 112. None of
 * that is visible today because `decode_condition_direct` (vm.c) has no callers:
 * every condition decodes through the lossless mirror `read_condition_fields`
 * (vm.c:404). This test is what makes the trap observable.
 *
 * WHAT IS ASSERTED
 * ----------------
 * For every condition variant 0..=20 (the 21 build_* functions) and every value
 * shape each key can take on the wire:
 *
 *   1. decode_condition_direct returns non-NULL;
 *   2. it consumes EXACTLY the node's bytes  Ea decoder that mis-reads a value
 *      desyncs every later field, so length equality is the load-bearing check;
 *   3. rb_decode_fallback_count() does not move  Ethe catch-all did not fire;
 *   4. the key is present on the resulting Condition with the value and type
 *      Rust assigns (bool / bool / bool / string / nested object);
 *   5. a CONTROL key that the C genuinely does not name DOES move the counter,
 *      which proves assertion 3 is not vacuous.
 */
#include "rabuka.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* vm.c's private reader shape (vm.c:239). Identical layout, so the test can
 * address a node inside a hand-built buffer. */
typedef struct { const unsigned char *p; const unsigned char *end; int ability; } T7Rdr;
typedef struct Condition T7Condition;
T7Condition *decode_condition_direct(T7Rdr *r, uint8_t variant);

static int fails;
#define FAILF(...) do { fprintf(stderr, "FAIL: " __VA_ARGS__); fails++; } while (0)

/* ── interned key lookup, so the test never hardcodes a table index ── */
static int key_idx(const char *name) {
    for (int i = 0; i < 8000; i++) {
        const char *s = rb_get_string((uint32_t)i);
        if (!s) return -1;
        if (!strcmp(s, name)) return i;
    }
    return -1;
}

/* ── byte buffer ── */
static unsigned char buf[512];
static int blen;
static void put(uint8_t b) { buf[blen++] = b; }
static void put_len(uint32_t n) {
    if (n < 0xFE) put((uint8_t)n);
    else { put(0xFE); put((uint8_t)(n & 0xFF)); put((uint8_t)(n >> 8)); }
}
static void put_idx(uint32_t i) {
    if (i < 0xFE) put((uint8_t)i);
    else { put(0xFE); put((uint8_t)(i & 0xFF)); put((uint8_t)(i >> 8)); }
}

/* ── value encoders (tags per include/rabuka.h:17-25) ── */
static void put_bool(int v) { put(v ? 0x02 : 0x01); }
static void put_null(void) { put(0x00); }
static void put_str(int idx) { put(0x06); put_idx((uint32_t)idx); }
/* a TriggerEvent-shaped nested dict: TAG_OBJECT, 2 fields */
static void put_obj2(int k1, int v1kind, int k2, int v2kind) {
    put(0x08); put_len(2);
    put_idx((uint32_t)k1); put(v1kind);
    if (v1kind == 0x06) put_idx(7);
    put_idx((uint32_t)k2); put(v2kind);
    if (v2kind == 0x06) put_idx(7);
}
/* a nested condition: TAG_OBJVAR, variant 5 (state), 0 fields */
static void put_objvar_empty(uint8_t variant) { put(0x09); put(variant); put_len(0); }

/* ── run one synthetic node ── */
static const CondValue *find_field(const T7Condition *c, const char *key) {
    for (uint32_t i = 0; i < c->n_fields; i++)
        if (c->fields[i].key && !strcmp(c->fields[i].key, key)) return &c->fields[i].v;
    return NULL;
}

/* One case: variant + one (key, value) pair.
 * shape 0 = RB_TAG_FALSE, 1 = RB_TAG_TRUE, 2 = RB_TAG_NULL. */
#define SH_FALSE 0
#define SH_TRUE  1
#define SH_NULL  2

static void case_ok(uint8_t variant, const char *key, const char *label, int shape) {
    int ki = key_idx(key);
    if (ki < 0) { FAILF("key \"%s\" not in the string table\n", key); return; }

    blen = 0;
    put(variant);
    put_len(1);
    put_idx((uint32_t)ki);
    if (shape == SH_TRUE)       put_bool(1);
    else if (shape == SH_FALSE) put_bool(0);
    else                        put_null();

    long node_len = blen - 1;
    uint32_t before = rb_decode_fallback_count();
    T7Rdr r; r.p = buf + 1; r.end = buf + blen; r.ability = 0;
    uint8_t rv; memcpy(&rv, buf, 1);
    T7Condition *c = decode_condition_direct(&r, rv);
    uint32_t after = rb_decode_fallback_count();
    long consumed = (r.p - buf - 1);

    if (!c) { FAILF("variant %u key=%s (%s): decode returned NULL\n", variant, key, label); return; }
    if (consumed != node_len)
        FAILF("variant %u key=%s (%s): consumed %ld of %ld bytes\n",
              variant, key, label, consumed, node_len);
    if (after != before)
        FAILF("variant %u key=%s (%s): hit the catch-all (%u -> %u)\n",
              variant, key, label, before, after);
    const CondValue *v = find_field(c, key);
    if (shape == SH_NULL) {
        /* Rust: read_bool_value on TAG_NULL yields None, so the field must be
         * absent  Einventing a FALSE here would be a new silent wrong value. */
        if (v) FAILF("variant %u: key \"%s\" INVENTED from TAG_NULL (%s)\n", variant, key, label);
    } else {
        uint8_t want = shape == SH_TRUE ? RB_TAG_TRUE : RB_TAG_FALSE;
        if (!v) FAILF("variant %u: key \"%s\" DROPPED (%s)\n", variant, key, label);
        else if (v->tag != want)
            FAILF("variant %u: key \"%s\" tag 0x%02X, wire says 0x%02X\n", variant, key, v->tag, want);
        else if (v->b != (shape == SH_TRUE))
            FAILF("variant %u: key \"%s\" b=%d, wire says %d\n", variant, key, v->b, shape == SH_TRUE);
    }
    rb_free_condition(c);
}

/* Control: a key the C decoder genuinely does not name. Proves the
 * rb_decode_fallback_count() assertions above are not vacuous. */
static void case_control(void) {
    int ki = key_idx("full_text");
    if (ki < 0) { FAILF("control key full_text not interned\n"); return; }
    blen = 0;
    put(0); put_len(1); put_idx((uint32_t)ki); put_str(ki);
    long node_len = blen - 1;
    uint32_t before = rb_decode_fallback_count();
    T7Rdr r; r.p = buf + 1; r.end = buf + blen; r.ability = 0;
    uint8_t rv = 0;
    T7Condition *c = decode_condition_direct(&r, rv);
    uint32_t after = rb_decode_fallback_count();
    if (!c) { FAILF("control: decode returned NULL\n"); return; }
    if ((r.p - buf - 1) != node_len)
        FAILF("control: consumed %ld of %ld bytes\n", (r.p - buf - 1), node_len);
    if (after == before)
        FAILF("control: an unnamed key did NOT move rb_decode_fallback_count(), so the "
              "no-catch-all assertions are vacuous\n");
    rb_free_condition(c);
}

/* `text` and `trigger_event` have non-bool shapes; separate cases. */
static void case_text_str(uint8_t variant) {
    int ki = key_idx("text"), si = key_idx("full_text");
    if (ki < 0 || si < 0) { FAILF("text/full_text not interned\n"); return; }
    blen = 0; put(variant); put_len(1); put_idx((uint32_t)ki); put_str(si);
    long node_len = blen - 1;
    uint32_t before = rb_decode_fallback_count();
    T7Rdr r; r.p = buf + 1; r.end = buf + blen; r.ability = 0;
    uint8_t rv; memcpy(&rv, buf, 1);
    T7Condition *c = decode_condition_direct(&r, rv);
    uint32_t after = rb_decode_fallback_count();
    if (!c) { FAILF("variant %u: text/STR decode NULL\n", variant); return; }
    if ((r.p - buf - 1) != node_len) FAILF("variant %u: text/STR consumed %ld of %ld\n", variant, (r.p - buf - 1), node_len);
    if (after != before) FAILF("variant %u: text/STR hit the catch-all\n", variant);
    const CondValue *v = find_field(c, "text");
    if (!v) FAILF("variant %u: text DROPPED (TAG_STR)\n", variant);
    else if (v->tag != RB_TAG_STR) FAILF("variant %u: text tag 0x%02X, want RB_TAG_STR\n", variant, v->tag);
    else if (strcmp(v->s ? v->s : "", rb_get_string((uint32_t)si)))
        FAILF("variant %u: text value mismatch\n", variant);
    rb_free_condition(c);
}

static void case_text_null(uint8_t variant) {
    int ki = key_idx("text");
    if (ki < 0) { FAILF("text not interned\n"); return; }
    blen = 0; put(variant); put_len(1); put_idx((uint32_t)ki); put_null();
    uint32_t before = rb_decode_fallback_count();
    T7Rdr r; r.p = buf + 1; r.end = buf + blen; r.ability = 0;
    uint8_t rv; memcpy(&rv, buf, 1);
    T7Condition *c = decode_condition_direct(&r, rv);
    uint32_t after = rb_decode_fallback_count();
    if (!c) { FAILF("variant %u: text/NULL decode NULL\n", variant); return; }
    if ((r.p - buf - 1) != blen - 1) FAILF("variant %u: text/NULL consumed %ld of %ld\n", variant, (r.p - buf - 1), (long)(blen - 1));
    if (after != before) FAILF("variant %u: text/NULL hit the catch-all\n", variant);
    if (find_field(c, "text")) FAILF("variant %u: text INVENTED from TAG_NULL\n", variant);
    rb_free_condition(c);
}

static void case_trigger_event(uint8_t variant, uint8_t value_tag) {
    int ki = key_idx("trigger_event");
    int t1 = key_idx("type"), t2 = key_idx("location");
    if (ki < 0 || t1 < 0 || t2 < 0) { FAILF("trigger_event keys not interned\n"); return; }
    blen = 0;
    put(variant); put_len(1); put_idx((uint32_t)ki);
    if (value_tag == 0x08)      put_obj2(t1, 0x06, t2, 0x02);
    else if (value_tag == 0x09) put_objvar_empty(5);
    else                        put_null();
    long node_len = blen - 1;
    uint32_t before = rb_decode_fallback_count();
    T7Rdr r; r.p = buf + 1; r.end = buf + blen; r.ability = 0;
    uint8_t rv; memcpy(&rv, buf, 1);
    T7Condition *c = decode_condition_direct(&r, rv);
    uint32_t after = rb_decode_fallback_count();
    if (!c) { FAILF("variant %u: trigger_event/0x%02X decode NULL\n", variant, value_tag); return; }
    if ((r.p - buf - 1) != node_len)
        FAILF("variant %u: trigger_event/0x%02X consumed %ld of %ld\n", variant, value_tag, (r.p - buf - 1), node_len);
    if (after != before) FAILF("variant %u: trigger_event/0x%02X hit the catch-all\n", variant, value_tag);
    const CondValue *v = find_field(c, "trigger_event");
    if (value_tag == 0x00) {
        if (v) FAILF("variant %u: trigger_event INVENTED from TAG_NULL\n", variant);
    } else {
        if (!v) FAILF("variant %u: trigger_event DROPPED (tag 0x%02X)\n", variant, value_tag);
        /* cond_add_cond (vm.c:1491) is the C carrier for every nested
         * condition and always re-tags the field RB_TAG_OBJVAR, so a
         * TAG_OBJECT TriggerEvent and a TAG_OBJECT_VARIANT one both land as
         * OBJVAR. The byte-exact length check above is what proves the value
         * was consumed whole. */
        else if (v->tag != RB_TAG_OBJVAR)
            FAILF("variant %u: trigger_event tag 0x%02X, cond_add_cond carries OBJVAR\n", variant, v->tag);
        else if (!v->cond)
            FAILF("variant %u: trigger_event carried no nested condition\n", variant);
    }
    rb_free_condition(c);
}

/* all five keys in ONE node  Ethe shape the corpus actually uses (T7's wire
 * census shows `text` and `card_type`/`target`/`location` co-occurring on the
 * same condition 505 times). */
static void case_all_five(uint8_t variant) {
    int kal = key_idx("any_location"), katt = key_idx("activated_this_turn");
    int kpc = key_idx("paid_cost_this_turn"), ktx = key_idx("text");
    int kte = key_idx("trigger_event"), kct = key_idx("card_type");
    int ktt = key_idx("target"), kloc = key_idx("location"), si = key_idx("full_text");
    int kneg = key_idx("negation");
    if (kal < 0 || katt < 0 || kpc < 0 || ktx < 0 || kte < 0 || kct < 0 || ktt < 0 || kloc < 0
        || si < 0 || kneg < 0) {
        FAILF("variant %u: a needed key is not interned\n", variant); return;
    }
    blen = 0;
    put(variant);
    put_len(9);
    put_idx((uint32_t)ktt);  put_str(si);
    put_idx((uint32_t)ktx);  put_str(si);
    put_idx((uint32_t)kloc); put_str(si);
    put_idx((uint32_t)kct);  put_str(si);
    put_idx((uint32_t)kal);  put_bool(1);
    put_idx((uint32_t)katt); put_bool(0);
    put_idx((uint32_t)kpc);  put_bool(1);
    put_idx((uint32_t)kte);  put_obj2(kal, 0x06, ktt, 0x02);
    put_idx((uint32_t)kneg); put_bool(1);
    long node_len = blen - 1;
    uint32_t before = rb_decode_fallback_count();
    T7Rdr r; r.p = buf + 1; r.end = buf + blen; r.ability = 0;
    uint8_t rv; memcpy(&rv, buf, 1);
    T7Condition *c = decode_condition_direct(&r, rv);
    uint32_t after = rb_decode_fallback_count();
    if (!c) { FAILF("variant %u: all-five decode NULL\n", variant); return; }
    if ((r.p - buf - 1) != node_len)
        FAILF("variant %u: all-five consumed %ld of %ld\n", variant, (r.p - buf - 1), node_len);
    if (after != before) FAILF("variant %u: all-five hit the catch-all %u time(s)\n", variant, after - before);
    const CondValue *al = find_field(c, "any_location");
    const CondValue *at = find_field(c, "activated_this_turn");
    const CondValue *pc = find_field(c, "paid_cost_this_turn");
    const CondValue *tx = find_field(c, "text");
    const CondValue *te = find_field(c, "trigger_event");
    if (!al) FAILF("variant %u: all-five any_location DROPPED\n", variant);
    else if (al->tag != RB_TAG_TRUE || al->b != 1) FAILF("variant %u: all-five any_location != TRUE\n", variant);
    if (!at) FAILF("variant %u: all-five activated_this_turn DROPPED\n", variant);
    else if (at->tag != RB_TAG_FALSE || at->b != 0) FAILF("variant %u: all-five activated_this_turn != FALSE\n", variant);
    if (!pc) FAILF("variant %u: all-five paid_cost_this_turn DROPPED\n", variant);
    else if (pc->tag != RB_TAG_TRUE || pc->b != 1) FAILF("variant %u: all-five paid_cost_this_turn != TRUE\n", variant);
    if (!tx) FAILF("variant %u: all-five text DROPPED\n", variant);
    else if (tx->tag != RB_TAG_STR) FAILF("variant %u: all-five text tag 0x%02X\n", variant, tx->tag);
    if (!te) FAILF("variant %u: all-five trigger_event DROPPED\n", variant);
    else if (te->tag != RB_TAG_OBJVAR) FAILF("variant %u: all-five trigger_event tag 0x%02X\n", variant, te->tag);
    if (c->n_fields != 9)
        FAILF("variant %u: all-five kept %u of 9 fields\n", variant, c->n_fields);
    rb_free_condition(c);
}

int main(int argc, char **argv) {
    const char *dir = argc > 1 ? argv[1] : "src";
    if (rb_load(dir) < 0) { fprintf(stderr, "FAIL: rb_load\n"); return 2; }

    case_control();

    for (uint8_t v = 0; v <= 20; v++) {
        case_ok(v, "any_location", "TRUE", SH_TRUE);
        case_ok(v, "any_location", "FALSE", SH_FALSE);
        case_ok(v, "any_location", "NULL", SH_NULL);
        case_ok(v, "activated_this_turn", "TRUE", SH_TRUE);
        case_ok(v, "activated_this_turn", "FALSE", SH_FALSE);
        case_ok(v, "activated_this_turn", "NULL", SH_NULL);
        case_ok(v, "paid_cost_this_turn", "TRUE", SH_TRUE);
        case_ok(v, "paid_cost_this_turn", "FALSE", SH_FALSE);
        case_ok(v, "paid_cost_this_turn", "NULL", SH_NULL);
        case_text_str(v);
        case_text_null(v);
        case_trigger_event(v, 0x08);
        case_trigger_event(v, 0x09);
        case_trigger_event(v, 0x00);
        case_all_five(v);
    }

    printf("variants tested: 21 (0..20)\n");
    printf("keys per variant: any_location TRUE/FALSE/NULL, activated_this_turn TRUE/FALSE/NULL, "
           "paid_cost_this_turn TRUE/FALSE/NULL, text STR/NULL, trigger_event OBJECT/OBJVAR/NULL, "
           "and one all-five node\n");
    rb_unload();
    if (fails) { fprintf(stderr, "test_parity_t7_condition_decoder: %d FAIL\n", fails); return 1; }
    printf("ok: parity_t7_condition_decoder\n");
    return 0;
}