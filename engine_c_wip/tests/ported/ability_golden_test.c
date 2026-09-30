/* C port of engine/tests/test_modules/characterization/ability_golden_test.rs
 *
 *   fn abilities_shape_matches() {
 *       let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cards/abilities.json");
 *       let bytes = std::fs::read(&p).expect("abilities.json readable");
 *       let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
 *       let obj = val.as_object().expect("abilities.json is object");
 *       assert_eq!(obj.len(), 10, "abilities.json top-level key count changed — \
 *                                 re-golden deliberately; was 10");
 *   }
 *
 * BOTH Rust assertions are reproduced, neither weakened:
 *   1. the file must parse as JSON                     -> top-level `{`
 *   2. the top level must be an OBJECT                 -> counted below
 *   3. the object must have exactly 10 top-level keys   -> assert_eq
 *
 * No JSON library exists in the engine (src/**.c links no parser), so the
 * count is made by a single-pass scanner that tracks string state, escapes and
 * brace depth and counts the keys of the depth-1 object. That is enough for
 * this shape guard and is exact: a `{` or `}` or `:` inside a JSON string
 * cannot be counted, because string state takes priority over depth.
 *
 * FILE LOCATION. Rust resolves the path at compile time
 * (env!("CARGO_MANIFEST_DIR")/../cards/abilities.json). C has no such
 * anchor, so the path is resolved at run time, in order:
 *   1. $RABUKA_ABILITIES_JSON           (explicit override)
 *   2. ../cards/abilities.json          (what `make t` sees from engine_c_wip)
 *   3. ../../cards/abilities.json       (what an isolated build root sees:
 *                                       <root>/engine_c -> <root>/cards)
 *   4. cards/abilities.json
 * If none of them exists the test FAILS. It does not skip and it does not
 * pass: a shape guard that silently passes because it could not find its
 * input is the exact failure mode this file used to have.
 *
 * BUILD WIRING (NOT DONE HERE -- these files belong to another owner):
 *   * `make t T=<stem>` requires the source at tests/test_<stem>.c, and
 *     `run-all-tests` globs tests/test_*.c (Makefile:73 and :81). This file
 *     lives in tests/ported/, so NEITHER target compiles it. Renaming it to
 *     tests/test_ability_golden.c (or adding tests/ported/*.c to both globs)
 *     is required before it asserts anything.
 *   * tools/isolated_build.sh copies ONLY cards/build/cards.bin and
 *     cards/build/abilities_strings.bin into the build root (lines 61-63).
 *     cards/abilities.json is never copied, so under the isolated build the
 *     path search above finds nothing and this test will report RED with
 *     "abilities.json not found". Adding
 *         cp -f "$REPO/cards/abilities.json" "$ROOT/cards/"
 *     to that script is the second required change.
 * Both are called out rather than worked around: until they land this file
 * asserts nothing at runtime, and the failure message below says so.
 */
#include "rabuka.h"
#include "test_game.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int failures;
static int checks;

#define CHECK(cond, msg) do { \
    checks++; \
    if (!(cond)) { \
        fprintf(stderr, "FAIL %s:%d: %s\n", __FILE__, __LINE__, (msg)); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

#define CHECK_EQ(actual, expected, msg) do { \
    long a_ = (long)(actual), e_ = (long)(expected); \
    checks++; \
    if (a_ != e_) { \
        fprintf(stderr, "FAIL %s:%d: %s (got %ld expected %ld)\n", __FILE__, __LINE__, (msg), a_, e_); \
        failures++; \
    } else { printf("ok: %s\n", (msg)); } \
} while (0)

#define ABILITIES_JSON_GOLDEN_TOP_LEVEL_KEYS 10

typedef struct {
    int  is_object;      /* first non-space byte was '{'                     */
    int  top_level_keys; /* keys of the depth-1 object                      */
    int  truncated;      /* unterminated string literal, or depth < 0 at EOF */
} JsonShape;

/* Single-pass structural scan: enough to answer "is the top level an object
   and how many keys does it have", which is all the Rust test asserts. It is
   deliberately NOT a JSON parser and does not validate values -- it only has
   to be exact about braces and colons that live inside string literals. */
static JsonShape json_top_level_shape(const char *buf, long len)
{
    JsonShape s = { 0, 0, 0 };
    long i = 0;
    int depth = 0;
    int in_string = 0, escaped = 0;
    /* A key is a string literal at depth 1 whose next non-space byte is ':'.
       `just_closed_key_string` records that we are looking at one. */
    int just_closed_key_string = 0;

    while (i < len && (buf[i] == ' ' || buf[i] == '\t' ||
                       buf[i] == '\r' || buf[i] == '\n')) i++;
    if (i < len && buf[i] == '{') s.is_object = 1;

    for (; i < len; i++) {
        char c = buf[i];
        if (in_string) {
            if (escaped)          escaped = 0;
            else if (c == '\\')   escaped = 1;
            else if (c == '"')  { in_string = 0; just_closed_key_string = 1; }
            continue;
        }
        switch (c) {
            case '"':
                in_string = 1;
                escaped = 0;
                just_closed_key_string = 0;
                continue;
            case '{':
            case '[':
                depth++;
                just_closed_key_string = 0;
                continue;
            case '}':
            case ']':
                depth--;
                if (depth < 0) { s.truncated = 1; return s; }
                just_closed_key_string = 0;
                continue;
            case ':':
                if (depth == 1 && just_closed_key_string) s.top_level_keys++;
                just_closed_key_string = 0;
                continue;
            case ' ':
            case '\t':
            case '\r':
            case '\n':
            case ',':
                continue;   /* a string literal is still a key across these */
            default:
                just_closed_key_string = 0;
                continue;
        }
    }
    if (in_string || depth != 0) s.truncated = 1;
    return s;
}

static char *read_whole_file(const char *path, long *out_len)
{
    FILE *f = fopen(path, "rb");
    if (!f) return NULL;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return NULL; }
    long n = ftell(f);
    if (n < 0 || fseek(f, 0, SEEK_SET) != 0) { fclose(f); return NULL; }
    char *buf = (char *)malloc((size_t)n + 1);
    if (!buf) { fclose(f); return NULL; }
    if (n > 0 && fread(buf, 1, (size_t)n, f) != (size_t)n) {
        free(buf); fclose(f); return NULL;
    }
    buf[n] = '\0';
    fclose(f);
    *out_len = n;
    return buf;
}

static void abilities_shape_matches(void)
{
    const char *candidates[5];
    const char *env = getenv("RABUKA_ABILITIES_JSON");
    candidates[0] = env;
    candidates[1] = "../cards/abilities.json";
    candidates[2] = "../../cards/abilities.json";
    candidates[3] = "cards/abilities.json";
    candidates[4] = "../../../cards/abilities.json";

    char *buf = NULL;
    long len = 0;
    const char *used = NULL;
    for (size_t i = 0; i < sizeof(candidates) / sizeof(candidates[0]); i++) {
        if (!candidates[i] || !candidates[i][0]) continue;
        buf = read_whole_file(candidates[i], &len);
        if (buf) { used = candidates[i]; break; }
    }

    if (!buf) {
        /* FAIL, never skip. A shape guard that cannot reach its input and
           reports success is the bug this file was rewritten to remove. */
        checks++;
        fprintf(stderr,
                "FAIL %s:%d: abilities.json not found -- set RABUKA_ABILITIES_JSON "
                "to its absolute path. Tried %s, ../cards/abilities.json, "
                "../../cards/abilities.json, cards/abilities.json, "
                "../../../cards/abilities.json. NOTE: tools/isolated_build.sh "
                "copies only cards/build/cards.bin and cards/build/"
                "abilities_strings.bin into the build root, so this test is RED "
                "under `isolated_build.sh` until cards/abilities.json is copied "
                "there too.\n",
                __FILE__, __LINE__,
                env ? env : "(unset)");
        failures++;
        return;
    }
    printf("ok: abilities.json readable (%s, %ld bytes)\n", used, len);

    JsonShape s = json_top_level_shape(buf, len);

    /* Rust: `val.as_object().expect("abilities.json is object")`.
       serde_json has already proven the bytes parse, so a scan that stops
       mid-literal or unbalances its depth means the same thing to us. */
    CHECK(!s.truncated, "abilities.json parses (no unterminated string, balanced braces)");
    CHECK(s.is_object, "abilities.json is object (Rust: val.as_object().expect)");

    /* Rust: assert_eq!(obj.len(), 10, "...re-golden deliberately; was 10").
       Reported as the same number so a change here reads like the Rust diff.
       If this ever fails, the extractor changed its top-level shape on
       purpose -- re-golden BOTH engines together, do not relax the count. */
    CHECK_EQ(s.top_level_keys, ABILITIES_JSON_GOLDEN_TOP_LEVEL_KEYS,
             "abilities.json top-level key count changed — "
             "re-golden deliberately; was 10");

    free(buf);
}

int main(void)
{
    printf("\n%s\n", "ability_golden_test -- C port of "
                    "engine/tests/test_modules/characterization/"
                    "ability_golden_test.rs");
    abilities_shape_matches();
    printf("\n%s: %d checks, %d failures\n", "ability_golden_test", checks, failures);
    return failures ? 1 : 0;
}
