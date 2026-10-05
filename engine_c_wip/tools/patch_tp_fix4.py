import io, hashlib
P = "tests/test_tp_activation_offer.c"
s = io.open(P, encoding="utf-8", newline="").read()

old = '''    printf("info: offered activation count = %d (Rust asserts <= 5)\\n", activations);
    CHECK(activations <= 5,
          "RUST (:75-79): activations <= 5 -- bounded by affordable energy "
          "(9/2 = 4); the count on the preceding info line is what is bounded");'''

new = '''    printf("info: offered activation count = %d (Rust asserts <= 5)\\n", activations);
    /* NOT written as a bare `CHECK(activations <= 5, ...)`.  With the real
     * generator a zero offer count satisfies that bound VACUOUSLY -- 0 <= 5 --
     * and a vacuously green bound is exactly how this file's premise went
     * stale in the first place.  The bound is therefore only evaluated once
     * its premise holds, and a missing premise is a FAILURE rather than a
     * silent pass.  The retired press proxy could not go vacuous this way,
     * which is the only thing it had going for it. */
    if (activations > 0) {
        CHECK(activations <= 5,
              "RUST (:75-79): activations <= 5 -- bounded by affordable energy "
              "(9/2 = 4); the count on the preceding info line is what is bounded");
    } else {
        CHECK(0,
              "RUST (:75-79) bound is UNEVALUABLE: rb_generate_action_candidates "
              "emitted no RB_SETUP_ACT_USE_ABILITY row for PL!HS-bp1-002-R at all, "
              "with the card on p1's centre and 9 active energy, so 'activations "
              "<= 5' would hold at 0 for free. Rust's count_sayaka_offers "
              "(game_setup.rs:2078 activation scan) is non-zero here. This is a C "
              "GENERATOR gap, not a bound that was met.");
    }'''

assert old in s, "anchor missing"
out = s.replace(old, new, 1)
with io.open(P, "r+", encoding="utf-8", newline="") as f:
    f.seek(0)
    f.write(out)
    f.truncate()
print("sha", hashlib.sha256(out.encode("utf-8")).hexdigest())