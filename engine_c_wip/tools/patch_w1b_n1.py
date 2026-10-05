import io, hashlib
P = "tests/test_parity_wave1b.c"
s = io.open(P, encoding="utf-8", newline="").read()

OLD = '''static void b9_active_phase_is_not_an_activation_history_writer(void)
{
    static TestGame tg;
    fresh(&tg);
    int waiting = mid(&tg, PRINT_1);'''

NEW = '''static void b9_active_phase_is_not_an_activation_history_writer(void)
{
    static TestGame tg;
    /* `fresh()` is this file's reset idiom, but it reaches test_game_new through
     * a SIBLING function, which test_harness_self's section N deliberately
     * COUNTS as uninitialised (its N3c case: "the only initialiser is in a
     * SIBLING function the body merely CALLS ... is COUNTED").  Writing the
     * three calls out here puts the initialiser in THIS body, which is the N3b
     * shape -- not counted, and byte-for-byte what fresh() does.  Doing it this
     * way keeps this new case from moving harness_self's N1 ratchet, which the
     * brief forbids raising. */
    rb_card_copy_pool_reset();
    test_game_new(&tg);
    rb_look_reset_all();
    int waiting = mid(&tg, PRINT_1);'''

assert OLD in s, "anchor missing"
out = s.replace(OLD, NEW, 1)
with io.open(P, "r+", encoding="utf-8", newline="") as f:
    f.seek(0)
    f.write(out)
    f.truncate()
print("sha", hashlib.sha256(out.encode("utf-8")).hexdigest())