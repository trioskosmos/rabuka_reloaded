#include "web_server.h"
#include "deck_parser.h"
#include "deck_builder.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdarg.h>
#include <ctype.h>
#include <errno.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>
#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#define SOCKET int
#define INVALID_SOCKET (-1)
#define closesocket closesocket
#define SOCKET_ERROR 1
typedef int socklen_t;
#else
typedef int SOCKET;
#define INVALID_SOCKET (-1)
#define SOCKET_ERROR 1
#define closesocket close
typedef int socklen_t;
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>
#endif

#define RB_WEB_MAX_BODY (1024 * 1024)
#define ROOM_ID "SANDBX"

typedef struct {
    char *data;
    size_t len;
    size_t cap;
} Buffer;

typedef struct {
    int socket;
    char request[8192];
    char *body;
    size_t body_len;
} HttpRequest;

typedef struct {
    GameState state;
    RbBuiltDeck built_deck[2];
    int version;
    int initialized;
    int deck_ready[2];
} Sandbox;

static void buffer_free(Buffer *b) { free(b->data); b->data = NULL; b->len = b->cap = 0; }
static int buffer_reserve(Buffer *b, size_t extra) {
    size_t need = b->len + extra + 1;
    if (need <= b->cap) return 1;
    size_t cap = b->cap ? b->cap : 4096;
    while (cap < need) cap *= 2;
    char *p = (char *)realloc(b->data, cap);
    if (!p) return 0;
    b->data = p; b->cap = cap; return 1;
}
static int buffer_put(Buffer *b, const char *s) {
    size_t n = strlen(s); return buffer_reserve(b, n) && (memcpy(b->data + b->len, s, n), b->len += n, b->data[b->len] = 0, 1);
}
static int buffer_putf(Buffer *b, const char *fmt, ...) {
    va_list ap; va_start(ap, fmt); int n = vsnprintf(NULL, 0, fmt, ap); va_end(ap);
    if (n < 0 || !buffer_reserve(b, (size_t)n)) return 0;
    va_start(ap, fmt); vsnprintf(b->data + b->len, (size_t)n + 1, fmt, ap); va_end(ap); b->len += (size_t)n; return 1;
}
static void json_string(Buffer *b, const char *s) {
    buffer_put(b, "\""); if (!s) s = "";
    for (; *s; s++) {
        unsigned char c = (unsigned char)*s;
        if (c == '\\' || c == '"') buffer_putf(b, "\\%c", c);
        else if (c == '\n') buffer_put(b, "\\n");
        else if (c == '\r') buffer_put(b, "\\r");
        else if (c == '\t') buffer_put(b, "\\t");
        else if (c < 32) buffer_putf(b, "\\u%04x", c);
        else buffer_putf(b, "%c", c);
    }
    buffer_put(b, "\"");
}
static const char *phase_wire_name(const GameState *g)
{
    if (!g) return "Active";
    switch (g->phase) {
        case RB_PHASE_RPS: return "RockPaperScissors";
        case RB_PHASE_OPENING: return "ChooseFirstAttacker";
        case RB_PHASE_MULLIGAN_FIRST: return "MulliganFirstAttacker";
        case RB_PHASE_MULLIGAN_SECOND: return "MulliganSecondAttacker";
        case RB_PHASE_ACTIVE: return "Active";
        case RB_PHASE_ENERGY: return "Energy";
        case RB_PHASE_DRAW: return "Draw";
        case RB_PHASE_MAIN: return "Main";
        case RB_PHASE_LIVE_SET:
        case RB_PHASE_LIVE_SET_SECOND:
            return g->active == g->first_attacker ? "LiveCardSetFirstAttacker" : "LiveCardSetSecondAttacker";
        case RB_PHASE_PERFORMANCE:
        case RB_PHASE_PERFORMANCE_SECOND:
            return g->active == g->first_attacker ? "FirstAttackerPerformance" : "SecondAttackerPerformance";
        case RB_PHASE_VICTORY: return "LiveVictoryDetermination";
        case RB_PHASE_DONE: return "Response";
        default: return "Active";
    }
}

static void append_card(Buffer *b, int cid, const char *orientation) {
    Card c; memset(&c, 0, sizeof(c));
    if (!rb_decode_card_by_index((uint32_t)cid, &c)) { buffer_put(b, "null"); return; }
    buffer_put(b, "{\"id\":"); buffer_putf(b, "%d", cid); buffer_put(b, ",\"card_no\":");
    json_string(b, rb_card_string(c.card_no_idx)); buffer_put(b, ",\"name\":"); json_string(b, c.name);
    buffer_putf(b, ",\"type\":%d,\"cost\":%d,\"blade\":%d,\"score\":%d,\"num_need\":%d",
        (int)(c.type_flags & 3), c.cost, c.blade, c.score, c.num_need);
    if (orientation) { buffer_put(b, ",\"orientation\":"); json_string(b, orientation); }
    buffer_put(b, "}"); rb_free_card(&c);
}
static void append_bag(Buffer *b, const RbBag *bag, const char *orientation) {
    buffer_put(b, "{\"count\":"); buffer_putf(b, "%d", bag->n); buffer_put(b, ",\"cards\":[");
    for (int i = 0; i < bag->n; i++) { if (i) buffer_put(b, ","); append_card(b, bag->cards[i], orientation); }
    buffer_put(b, "]}");
}
static void append_energy_bag(Buffer *b, const RbPlayer *p)
{
    buffer_put(b, "{\"count\":");
    buffer_putf(b, "%d", p->energy.n);
    buffer_put(b, ",\"cards\":[");
    for (int i = 0; i < p->energy.n; i++) {
        if (i) buffer_put(b, ",");
        append_card(b, p->energy.cards[i], i < p->energy_active ? "Active" : "Wait");
    }
    buffer_put(b, "]}");
}

static void append_player(Buffer *b, const RbPlayer *p) {
    buffer_put(b, "{\"hand\":"); append_bag(b, &p->hand, NULL);
    buffer_put(b, ",\"main_deck_count\":"); buffer_putf(b, "%d", p->deck.n);
    buffer_put(b, ",\"energy_deck_count\":"); buffer_putf(b, "%d", p->energy_deck.n);
    buffer_put(b, ",\"energy_active_count\":"); buffer_putf(b, "%d", p->energy_active);
    buffer_put(b, ",\"energy\":"); append_energy_bag(b, p);
    buffer_put(b, ",\"live_zone\":"); append_bag(b, &p->live, NULL);
    buffer_put(b, ",\"success_live_card_zone\":"); append_bag(b, &p->success, NULL);
    buffer_put(b, ",\"discard\":"); append_bag(b, &p->discard, NULL);
    buffer_put(b, ",\"stage\":{\"left_side\":"); if (p->stage[0] >= 0) append_card(b, p->stage[0], p->stage_wait[0] ? "Wait" : "Active"); else buffer_put(b, "null");
    buffer_put(b, ",\"center\":"); if (p->stage[1] >= 0) append_card(b, p->stage[1], p->stage_wait[1] ? "Wait" : "Active"); else buffer_put(b, "null");
    buffer_put(b, ",\"right_side\":"); if (p->stage[2] >= 0) append_card(b, p->stage[2], p->stage_wait[2] ? "Wait" : "Active"); else buffer_put(b, "null");
    buffer_put(b, ",\"left_under\":"); append_bag(b, &p->under_cards[0], NULL); buffer_put(b, ",\"center_under\":"); append_bag(b, &p->under_cards[1], NULL); buffer_put(b, ",\"right_under\":"); append_bag(b, &p->under_cards[2], NULL); buffer_put(b, "},\"score\":");
    buffer_putf(b, "%d,\"current_score\":%d,\"life\":%d}", p->score, p->score, p->life);
}
static void append_action(Buffer *b, const RbGeneratedAction *a, int index) {
    const char *type = a->action_type == 1 ? "pass" :
                      a->action_type == 14 ? "play_member_to_stage" : "use_ability";
    buffer_putf(b, "{\"index\":%d,\"action_type\":", index); json_string(b, type);
    buffer_put(b, ",\"description\":"); json_string(b, a->action_type == 1 ? "Pass" : "Action"); buffer_put(b, ",\"parameters\":{");
    if (a->has_parameters) { buffer_put(b, "\"card_id\":"); buffer_putf(b, "%d", a->parameters.card_id); buffer_put(b, ",\"available_areas\":[");
        for (int i = 0; i < a->parameters.n_available_areas; i++) buffer_putf(b, "%s{\"area\":%d,\"available\":%s,\"use_baton_touch\":%s}", i ? "," : "", i, a->parameters.available_areas[i].available ? "true" : "false", a->parameters.available_areas[i].is_baton_touch ? "true" : "false");
        buffer_put(b, "]"); }
    buffer_put(b, "}}");
}
static const char *choice_kind_wire(RbChoiceKind kind)
{
    switch (kind) {
        case RB_CHOICE_SELECT_CARD: return "select_card";
        case RB_CHOICE_SELECT_TARGET: return "select_target";
        case RB_CHOICE_SELECT_HEART_COLOR: return "select_heart_color";
        case RB_CHOICE_SELECT_NUMBER: return "select_number";
        case RB_CHOICE_SELECT_POSITION: return "select_position";
        case RB_CHOICE_SELECT_AUTO_ABILITY: return "select_auto_ability";
        default: return "choice";
    }
}

/* Rust core/game_state/abilities.rs:2559-2568, the `choice_player_id` key --
       let raw_pid = entry.choice_player_id.as_ref().unwrap_or(&entry.player_id);
       let normalized = match raw_pid.as_str() {
           "player1" => "p1", "player2" => "p2", _ => raw_pid.as_str() };

   Defined in src/core/game_state_abilities.c, NOT here, on purpose: the Makefile
   links src/web_server.o only into $(WEB_TARGET) (Makefile:100-102) and not into
   OBJ_LIB, so a symbol defined here cannot be reached from any tests/test_*.c
   binary — the routing this key carries would be unobservable from a suite.
   game_state_abilities.c IS in OBJ_LIB. Declared locally for the same reason and
   with the same precedent as rb_check_effect_position in src/turn/triggers.c:6-8.

   THE C EMITTED NO SUCH KEY AT ALL before this: `pending_choice` carried
   `kind`, `zone`, `target`, `card_type`, `count`, `allow_skip` and an INTEGER
   `actor`, and nothing named the routed seat. Rust's web client depends on the
   key twice over -- web_server.rs:1240-1249 drops the ENTIRE pending_choice
   when `choice_player_id` is missing or names anybody but the requester
   ("defensive: never leak opponent choices"), and pvp_player_can_act
   (web_server.rs:1151) gates the answer endpoint on the same field. The C has
   neither, so the routed seat reached the client only as an int. */
const char *rb_wire_pending_choice_player_id(const GameState *g);

/* ORPHANED LOGIC O2 — Rust's choice-ability CONTEXT.
 *
 * Rust builds this object in `inject_choice_ability_context`
 * (core/game_state/abilities.rs:2483-...) and `get_pending_choice_json`
 * (abilities.rs:2475-2480) MERGES it into the pending_choice object before the
 * object reaches the client, so the UI can show WHICH card and WHICH ability is
 * asking. That is `card_no` / `ability_text` / `card_name` / `card_id`, and it is
 * built for every choice in the game, not gated on any ability.
 *
 * Until now the C's actual wire -- `append_pending_choice` below -- never called
 * it: it emitted only count / allow_skip / actor, plus the choice_player_id key a
 * peer added. So `rb_inject_choice_ability_context` had ZERO production callers
 * and the C web UI could not tell the player what it was being asked about. MEASURED
 * with nm over all 49 OBJ_LIB objects: defined in game_state_abilities.o,
 * referenced by no object.
 *
 * Declared here, not in include/rabuka.h, on purpose: the Makefile links
 * src/web_server.o ONLY into $(WEB_TARGET) (Makefile:101-102) and not into
 * OBJ_LIB, so a prototype in the shared header would be the second declaration of
 * a symbol no suite binary can link against. Same file-scope declaration the
 * choice_player_id reader above already uses, and the same precedent as
 * rb_check_effect_position at src/turn/triggers.c:6-8.
 *
 * ⚠ THE ESCAPING CAVEAT, STATED BECAUSE IT IS REAL: the producer builds its
 * object with snprintf("%s") on raw card text, so a value containing '"' or a
 * newline is not escaped in the buffer this file is handed. Rather than splice
 * that buffer into the response verbatim -- which would emit invalid JSON for
 * such a card and break the WHOLE /state payload, not just the choice -- the two
 * helpers below pull each member out and re-emit it through json_string(), which
 * escapes properly. The producer is in src/core/game_state_abilities.c, which is
 * not this file's to edit; escaping at the producer is the proper fix and is
 * reported as a cross-file request. */
void rb_inject_choice_ability_context(GameState *g, char *json_buf, size_t buf_sz);

/* Read one `"key":"<value>"` member out of the object
 * rb_inject_choice_ability_context built. The producer does not escape, so the
 * value runs to the next '"' that is immediately followed by ',' or '}' -- the
 * two bytes that can only be the member separator or the object's own close. A
 * truncated buffer has neither and yields the empty string, which is the safe
 * direction: a missing display string, never a malformed response. */
static void json_injected_string(Buffer *b, const char *obj, const char *key)
{
    char needle[64];
    char scratch[1024];
    snprintf(needle, sizeof(needle), "\"%s\":\"", key);
    const char *p = obj && *obj ? strstr(obj, needle) : NULL;
    if (!p) { json_string(b, ""); return; }
    p += strlen(needle);
    const char *end = p;
    while (end[0] && !(end[0] == '"' && (end[1] == ',' || end[1] == '}'))) end++;
    size_t n = (size_t)(end - p);
    if (n >= sizeof scratch) n = sizeof scratch - 1;
    memcpy(scratch, p, n);
    scratch[n] = '\0';
    json_string(b, scratch);
}

/* The integer member, read straight out of the same buffer. */
static int json_injected_int(const char *obj, const char *key)
{
    char needle[64];
    snprintf(needle, sizeof(needle), "\"%s\":", key);
    const char *p = obj && *obj ? strstr(obj, needle) : NULL;
    return p ? atoi(p + strlen(needle)) : 0;
}

/* Rust engine/src/game/web_server.rs:1240-1249, the seat-leak filter, applied to
 * the C's pending_choice before it is written:
 *
 *     if let Some(ref pending) = display.pending_choice {
 *         let cpid = pending.get("choice_player_id").and_then(|v| v.as_str());
 *         let requester_str = if requester_player_id == 0 { "p1" } else { "p2" };
 *         match cpid {
 *             Some(cpid) if cpid == requester_str => {}
 *             _ => { display.pending_choice = None; }
 *         }
 *     }
 *
 * Two cases, both dropping the WHOLE object: the key is missing (Rust's comment
 * there is "defensive: never leak opponent choices") and the key names anybody
 * but the requester. The C had no such filter, so a prompt routed to the
 * opponent -- queue.rs:346-352, effects/choice.rs choice_maker, live.rs:1577 --
 * was serialised to the other seat's client in full. */
static int pending_choice_is_for_requester(const GameState *state, int requester)
{
    const char *cpid = rb_wire_pending_choice_player_id(state);
    const char *want = (requester == 0) ? "p1" : "p2";
    if (!cpid || !*cpid) return 0;
    return strcmp(cpid, want) == 0;
}

static void append_pending_choice(Buffer *b, const GameState *state, int requester)
{
    const RbChoice *choice = rb_get_pending_choice(state);
    if (!state || !rb_has_pending_choice(state) || !choice) {
        buffer_put(b, "null");
        return;
    }
    /* web_server.rs:1240-1249 -- drop the whole object when the routed seat is
       missing or is not the requester. Placed BEFORE anything is written, so a
       dropped choice leaks nothing at all: not the zone, not the card type, not
       the ability text. */
    if (!pending_choice_is_for_requester(state, requester)) {
        buffer_put(b, "null");
        return;
    }
    char ctx[4096];
    ctx[0] = '\0';
    rb_inject_choice_ability_context((GameState *)state, ctx, sizeof ctx);
    buffer_put(b, "{\"kind\":");
    json_string(b, choice_kind_wire(choice->kind));
    buffer_put(b, ",\"zone\":");
    json_string(b, choice->zone);
    buffer_put(b, ",\"target\":");
    json_string(b, choice->target);
    buffer_put(b, ",\"card_type\":");
    json_string(b, choice->card_type);
    buffer_putf(b, ",\"count\":%d,\"allow_skip\":%s,\"actor\":%d",
                choice->count, choice->allow_skip ? "true" : "false", choice->actor);
    /* Rust injects the key through inject_choice_ability_context, which
       get_pending_choice_json (abilities.rs:2475-2480) merges into the choice
       object BEFORE it reaches the client. Emitted last here so the pre-existing
       field order is byte-identical and only one key is added. */
    buffer_put(b, ",\"choice_player_id\":");
    json_string(b, rb_wire_pending_choice_player_id(state));
    /* ── O2: the rest of Rust's injected object, merged flat ──────────────────
       abilities.rs:2483-... puts card_no / ability_text / card_name / card_id in
       the SAME object, and get_pending_choice_json merges the whole thing, so
       these four are siblings of choice_player_id, not a nested sub-object.
       They are what lets the UI answer "which card, which ability". */
    buffer_put(b, ",\"card_no\":");
    json_injected_string(b, ctx, "card_no");
    buffer_put(b, ",\"ability_text\":");
    json_injected_string(b, ctx, "ability_text");
    buffer_put(b, ",\"card_name\":");
    json_injected_string(b, ctx, "card_name");
    buffer_putf(b, ",\"card_id\":%d", json_injected_int(ctx, "card_id"));
    buffer_put(b, "}");
}

static int serialize_state(Sandbox *s, Buffer *b) {
    RbGeneratedActionList actions = rb_generate_action_candidates(&s->state);
    /* The seat this payload is rendered FOR. It was a literal 0 in the
       ui_config below; it is now a named value used by BOTH the ui_config and
       the pending_choice seat-leak filter (web_server.rs:1240-1249), so the
       perspective the client is told about and the perspective the choice is
       filtered against cannot drift apart. */
    const int requester_player_id = 0;
    buffer_put(b, "{\"state_id\":\"c-"); buffer_putf(b, "%d", s->version); buffer_put(b, "\",\"frame_counter\":"); buffer_putf(b, "%d", s->version);
    buffer_put(b, ",\"turn\":"); buffer_putf(b, "%d", s->state.turn);
    buffer_put(b, ",\"phase\":"); json_string(b, phase_wire_name(&s->state));
    buffer_put(b, ",\"mode\":\"sandbox\",\"game_over\":"); buffer_put(b, s->state.winner >= 0 ? "true" : "false");
    buffer_put(b, ",\"active_player\":"); buffer_putf(b, "%d", s->state.active + 1);
    buffer_put(b, ",\"winner\":"); if (s->state.winner < 0) buffer_put(b, "null"); else buffer_putf(b, "%d", s->state.winner);
    buffer_put(b, ",\"pending_choice\":"); append_pending_choice(b, &s->state, requester_player_id);
    buffer_put(b, ",\"player1\":"); append_player(b, &s->state.p[0]); buffer_put(b, ",\"player2\":"); append_player(b, &s->state.p[1]);
    buffer_put(b, ",\"legal_actions\":["); for (int i = 0; i < actions.count; i++) { if (i) buffer_put(b, ","); append_action(b, &actions.actions[i], i); } buffer_put(b, "]");
    buffer_put(b, ",\"ui_config\":{\"perspective_player\":"); buffer_putf(b, "%d", requester_player_id); buffer_put(b, ",\"current_lang\":\"jp\"}}");
    rb_free(actions.actions); return 1;
}
static void send_http(int fd, const char *status, const char *type, const char *body, size_t len) {
    char head[512]; int n = snprintf(head, sizeof(head), "HTTP/1.1 %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n", status, type, len);
    send(fd, head, (size_t)n, 0); if (len) send(fd, body, len, 0);
}
static void send_json(int fd, int code, const char *json) { char st[32]; snprintf(st, sizeof(st), code == 200 ? "200 OK" : code == 404 ? "404 Not Found" : "400 Bad Request"); send_http(fd, st, "application/json; charset=utf-8", json, strlen(json)); }
static const char *json_value(const char *body, const char *key) { char needle[80]; if (!body || !key) return NULL; snprintf(needle, sizeof(needle), "\"%s\"", key); const char *p = strstr(body, needle); if (!p) return NULL; p = strchr(p, ':'); return p ? p + 1 : NULL; }
static int json_int(const char *body, const char *key, int def) { const char *p = json_value(body, key); return p ? atoi(p) : def; }
static int parse_json_card_array(const char *body, const char *key,
                                 int *ids, int max_ids, int *count)
{
    if (!body || !key || !ids || !count) return 0;
    *count = 0;
    const char *p = json_value(body, key);
    if (!p) return 0;
    while (*p && isspace((unsigned char)*p)) p++;
    if (*p != '[') return 0;
    p++;
    while (*p && *p != ']') {
        while (*p && (isspace((unsigned char)*p) || *p == ',')) p++;
        if (*p == ']') break;
        int card_id = -1;
        if (*p == '"') {
            char card[128];
            size_t n = 0;
            p++;
            while (*p && *p != '"' && n + 1 < sizeof(card)) {
                if (*p == '\\' && p[1]) p++;
                card[n++] = *p++;
            }
            card[n] = '\0';
            if (*p != '"') return 0;
            p++;
            /* Rust: web_server.rs:2355-2362 maps EVERY array element through
             * `DeckParser::normalize_card_no` (deck_parser.rs:200-208) before
             * the string is stored, and only `get_card_id` ever sees the result
             * (deck_builder.rs:42 / :103). That normalisation is NOT the same
             * fold `get_card_id` applies internally: CardDatabase::normalize_
             * card_no (card.rs:608) uppercases and folds fullwidth to halfwidth,
             * whereas DeckParser's replaces ASCII '+'/'!' with fullwidth and then
             * STRIPS every trailing fullwidth ＋ / ！.
             *
             * The peer claim that "the fold is symmetric, so it does not matter"
             * is FALSE, and the fold is not even symmetric — they are different
             * functions. The two differ exactly when the submitted number ends in
             * ASCII '+' or '!':
             *   Rust  F(N(s))  strips them, then resolves the rarity WITHOUT them
             *   C     F(s)      keeps them, so step 2 (exact normalized key) and
             *                    step 3a (base + requested rarity) both look for a
             *                    "...-R+" print that Rust never asks for, and the
             *                    answer falls through to step 3c "any rarity of this
             *                    base, lowest key wins" — a DIFFERENT CARD.
             * Step 1 (exact) differs too: N(s) is never the stored key when s
             * carries a trailing rarity mark, so Rust can miss step 1 where C
             * hits it.
             *
             * rb_normalize_card_no (src/game/deck_parser.c:27) is the ported
             * DeckParser::normalize_card_no, and this file already includes
             * deck_parser.h and calls rb_parse_deck_line from it. It is applied
             * here so the C resolves what Rust resolves.
             *
             * The parse_deck_content_ids fallback further down already gets this
             * right for free: rb_parse_deck_content runs every line through
             * rb_parse_deck_line, which normalises (deck_parser.c:117 / :131). */
            char normalized[128];
            if (rb_normalize_card_no(card, normalized, sizeof(normalized)) != 0) return 0;
            card_id = rb_card_get_card_id(normalized);
        } else {
            char *end = NULL;
            long value = strtol(p, &end, 10);
            if (end == p) return 0;
            p = end;
            card_id = (int)value;
        }
        if (card_id < 0 || rb_card_record((uint32_t)card_id) == NULL) return 0;
        if (*count >= max_ids) return 0;
        ids[(*count)++] = card_id;
        while (*p && *p != ',' && *p != ']') p++;
    }
    return *p == ']';
}

static int parse_json_string_value(const char *body, const char *key,
                                   char *out, size_t out_size)
{
    if (!body || !key || !out || out_size == 0) return 0;
    const char *p = json_value(body, key);
    if (!p) return 0;
    while (*p && isspace((unsigned char)*p)) p++;
    if (*p != '"') return 0;
    p++;
    size_t n = 0;
    while (*p && *p != '"' && n + 1 < out_size) {
        if (*p == '\\' && p[1]) p++;
        out[n++] = *p++;
    }
    if (*p != '"') return 0;
    out[n] = '\0';
    return 1;
}

static int parse_deck_content_ids(const char *body, int *ids, int max_ids, int *count)
{
    if (!body || !ids || !count) return 0;
    char content[16384];
    if (!parse_json_string_value(body, "content", content, sizeof(content))) return 0;
    char **cards = NULL;
    size_t card_count = 0;
    if (rb_parse_deck_content(content, &cards, &card_count) != 0) return 0;
    if (card_count > (size_t)max_ids) {
        rb_deck_card_numbers_free(cards, card_count);
        return 0;
    }
    for (size_t i = 0; i < card_count; i++) {
        ids[i] = rb_card_get_card_id(cards[i]);
        if (ids[i] < 0) {
            rb_deck_card_numbers_free(cards, card_count);
            return 0;
        }
    }
    *count = (int)card_count;
    rb_deck_card_numbers_free(cards, card_count);
    return 1;
}

/* Rust web_server.rs never hand-rolls the settle loop: every one of its four
 * call sites (web_server.rs:913 / 957 / 1702 / 1771) goes through the local
 * wrapper `settle_single_player_state` (web_server.rs:1144-1147), which is a
 * bare delegation to `game_setup::settle_single_player_state`
 * (game_setup.rs:410-430).
 *
 * This file used to carry its own narrower copy of that loop, and it was
 * wrong in three ways at once:
 *   - it advanced only Active / Energy / Draw, where Rust's
 *     `is_automatic_phase` list (game_setup.rs:369-375) is all SIX automatic
 *     phases — FirstAttackerPerformance, SecondAttackerPerformance and
 *     LiveVictoryDetermination as well. A web game that reached a performance
 *     window or the live victory determination STOPPED there, still sitting in
 *     PERFORMANCE / PERFORMANCE_SECOND / VICTORY, because the sandbox's
 *     serialize_state then advertised a phase the client can submit no action
 *     against. That is the stall.
 *   - it gave up after 16 iterations where Rust's guard is 500.
 *   - it had no `game_result != Ongoing` check, so it kept advancing a finished
 *     game.
 *
 * rb_setup_settle_single_player_state (src/game/game_setup.c:454) is the ported
 * twin and reproduces all three, including the 500-iteration guard. It had ZERO
 * callers tree-wide, which is why the divergence survived: this file could not
 * see a correct settle to copy. The inline loop is deleted, not kept as a
 * fallback — two settle loops in one binary is exactly how the two drift again.
 *
 * DECLARED HERE, NOT IN include/rabuka.h: the prototype is not in any header
 * (game_setup.c's own scope table at :113 marks it "WIRE?"), and rabuka.h is not
 * this file's to edit. A file-local extern is the minimal correct wiring. */
extern void rb_setup_settle_single_player_state(GameState *g);

static int make_test_deck(Sandbox *s)
{
    int main_ids[2][40];
    int energy_ids[2][RB_MAX_ENERGY_CARDS];
    int main_counts[2] = {0, 0};
    int energy_counts[2] = {0, 0};
    for (int pl = 0; pl < 2; pl++) {
        for (uint32_t card_id = 0; card_id < rb_num_cards(); card_id++) {
            if (rb_card_is_energy((int)card_id)) {
                if (energy_counts[pl] < RB_MAX_ENERGY_CARDS)
                    energy_ids[pl][energy_counts[pl]++] = (int)card_id;
            } else if (rb_card_ability_idx(card_id) != 0xFFFF && main_counts[pl] < 40) {
                main_ids[pl][main_counts[pl]++] = (int)card_id;
            }
        }
    }
    RbBuiltDeck built0;
    RbBuiltDeck built1;
    if (rb_build_deck_from_card_ids(main_ids[0], (size_t)main_counts[0],
                                    energy_ids[0], (size_t)energy_counts[0], &built0) != 0 ||
        rb_build_deck_from_card_ids(main_ids[1], (size_t)main_counts[1],
                                    energy_ids[1], (size_t)energy_counts[1], &built1) != 0)
        return 0;
    s->built_deck[0] = built0;
    s->built_deck[1] = built1;
    rb_seed(0xCAFE);
    if (rb_init_game_from_built_decks(&s->state, &built0, &built1) != 0) return 0;
    rb_setup_settle_single_player_state(&s->state);
    s->initialized = 1;
    s->deck_ready[0] = s->deck_ready[1] = 1;
    s->version++;
    return 1;
}

static int set_deck(Sandbox *s, const char *body, int player)
{
    if (!s || !body || player < 0 || player > 1) return 0;
    int main_ids[RB_MAX_DECK + RB_MAX_ENERGY_CARDS];
    int energy_ids[RB_MAX_ENERGY_CARDS];
    int main_count = 0;
    int energy_count = 0;
    int parsed = parse_json_card_array(body, "deck", main_ids,
                                       RB_MAX_DECK + RB_MAX_ENERGY_CARDS, &main_count);
    if (!parsed) parsed = parse_deck_content_ids(body, main_ids,
                                                 RB_MAX_DECK + RB_MAX_ENERGY_CARDS,
                                                 &main_count);
    if (!parsed || main_count == 0) return 0;
    if (json_value(body, "energy_deck") &&
        !parse_json_card_array(body, "energy_deck", energy_ids,
                               RB_MAX_ENERGY_CARDS, &energy_count))
        return 0;
    RbBuiltDeck built;
    if (rb_build_deck_from_card_ids(main_ids, (size_t)main_count,
                                    energy_ids, (size_t)energy_count, &built) != 0)
        return 0;
    s->built_deck[player] = built;
    s->deck_ready[player] = 1;
    return 1;
}
static int read_text_file(const char *path, char **out, size_t *out_len) {
    FILE *f = fopen(path, "rb"); long size;
    if (!f) return 0;
    if (fseek(f, 0, SEEK_END) != 0) { fclose(f); return 0; }
    size = ftell(f);
    if (size < 0 || fseek(f, 0, SEEK_SET) != 0) { fclose(f); return 0; }
    *out = (char *)malloc((size_t)size + 1u);
    if (!*out) { fclose(f); return 0; }
    *out_len = fread(*out, 1, (size_t)size, f);
    (*out)[*out_len] = '\0';
    fclose(f);
    return 1;
}

static void append_preset_card(Buffer *b, int *first, const char *card_no) {
    if (!*first) buffer_put(b, ",");
    *first = 0;
    json_string(b, card_no);
}

static int preset_card(const char *line, char *card_no, int *qty) {
    uint8_t quantity;
    if (!line || !card_no || !qty || !rb_parse_deck_line(line, card_no, 128, &quantity)) return 0;
    *qty = quantity;
    return 1;
}

static void append_deck_file_content(Buffer *b, const char *web_root, const char *filename) {
    char path[1024];
    RbDeckList deck;
    int first = 1;
    snprintf(path, sizeof(path), "%s/decks/%s", web_root, filename);
    if (rb_parse_deck_file(path, &deck) != 0) return;
    for (size_t i = 0; i < deck.count; i++) {
        for (int copy = 0; copy < deck.entries[i].quantity; copy++) {
            append_preset_card(b, &first, deck.entries[i].card_no);
        }
    }
    rb_deck_list_free(&deck);
}

static void append_deck_presets(Buffer *b, const char *web_root) {
    static const char *files[] = {
        "nijigaku_cup.txt", "muse_cup.txt", "liella_cup.txt", "hasunosora_cup.txt",
        "fade deck.txt", "bp7_unique_abilities.txt", "bp7_abilities_PL!SP.txt",
        "bp7_abilities_PL!S.txt", "bp7_abilities_PL!N.txt", "aqours_cup.txt",
        "aiscream 37PMZ.txt", "5ZNN5 sakkakubibi.txt", "5CP3Z idou.txt"
    };
    char path[1024];
    int first_preset = 1;
    buffer_put(b, "\"decks\":[");
    for (size_t fi = 0; fi < sizeof(files) / sizeof(files[0]); fi++) {
        char *content;
        size_t content_len;
        char *line;
        char id[128];
        int first_main = 1;
        int first_energy = 1;
        int count = 0;
        snprintf(path, sizeof(path), "%s/decks/%s", web_root, files[fi]);
        if (!read_text_file(path, &content, &content_len)) continue;
        snprintf(id, sizeof(id), "%s", files[fi]);
        size_t id_len = strlen(id);
        if (id_len > 4 && strcmp(id + id_len - 4, ".txt") == 0) id[id_len - 4] = '\0';
        if (!first_preset) buffer_put(b, ",");
        first_preset = 0;
        buffer_put(b, "{\"id\":");
        json_string(b, id);
        buffer_put(b, ",\"name\":");
        json_string(b, id);
        buffer_put(b, ",\"main\":[");
        for (line = strtok(content, "\r\n"); line; line = strtok(NULL, "\r\n")) {
            char card_no[128];
            int qty;
            if (!preset_card(line, card_no, &qty)) continue;
            int card_id = rb_card_get_card_id(card_no);
            if (card_id < 0 || rb_card_is_energy(card_id)) continue;
            for (int i = 0; i < qty; i++) {
                append_preset_card(b, &first_main, card_no);
                count++;
            }
        }
        buffer_put(b, "],\"energy\":[");
        content[0] = '\0';
        {
            FILE *f = fopen(path, "rb");
            if (f) {
                size_t n = fread(content, 1, content_len, f);
                content[n] = '\0';
                fclose(f);
            }
        }
        first_main = first_energy = 1;
        for (line = strtok(content, "\r\n"); line; line = strtok(NULL, "\r\n")) {
            char card_no[128];
            int qty;
            if (!preset_card(line, card_no, &qty)) continue;
            int card_id = rb_card_get_card_id(card_no);
            if (card_id < 0 || !rb_card_is_energy(card_id)) continue;
            for (int i = 0; i < qty; i++) append_preset_card(b, &first_energy, card_no);
        }
        buffer_put(b, "],\"card_count\":");
        char count_text[32];
        snprintf(count_text, sizeof(count_text), "%d}", count);
        buffer_put(b, count_text);
        free(content);
    }
    buffer_put(b, "]");
}

static void handle_request(Sandbox *s, int fd, HttpRequest *r, const char *web_root) {
    char method[16] = {0}, path[256] = {0}; const char *line = strstr(r->request, "\r\n");
    if (line) { size_t n = (size_t)(line - r->request); char *sp = memchr(r->request, ' ', n); if (sp) { sscanf(r->request, "%15s %255s", method, path); } }
    while (path[0] == '/') memmove(path, path + 1, strlen(path));
    if (!strcmp(path, "api/status")) { char status[224]; snprintf(status, sizeof(status), "{\"status\":\"rust_server\",\"backend\":\"c_server\",\"cards\":%u,\"niji\":%d,\"instance_id\":\"c-web-single-room\"}", rb_num_cards(), rb_find_card_by_no("PL!N-bp1-026-L")); send_json(fd, 200, status); return; }
    if (!strcmp(path, "api/rooms/create")) {
        rb_built_deck_clear(&s->built_deck[0]);
        rb_built_deck_clear(&s->built_deck[1]);
        s->deck_ready[0] = s->deck_ready[1] = 0;
        s->initialized = 0;
        send_json(fd, 200, "{\"success\":true,\"room_id\":\"SANDBX\",\"session\":{\"token\":\"sandbox\",\"player_id\":0}}");
        return;
    }
    if (!strcmp(path, "api/rooms/list")) { send_json(fd, 200, "{\"rooms\":[{\"room_id\":\"SANDBX\",\"mode\":\"sandbox\",\"players\":1}]}"); return; }
    if (!strcmp(path, "api/rooms/join") || !strcmp(path, "api/rooms/leave")) { send_json(fd, 200, "{\"success\":true,\"room_id\":\"SANDBX\"}"); return; }
    if (!strcmp(path, "api/get_decks")) { Buffer b = {0}; buffer_put(&b, "{\"success\":true,"); append_deck_presets(&b, web_root); buffer_put(&b, "}"); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/get_test_deck")) { Buffer b = {0}; buffer_put(&b, "{\"success\":true,\"content\":["); append_deck_file_content(&b, web_root, "nijigaku_cup.txt"); buffer_put(&b, "]}"); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/set_deck") && !strcmp(method, "POST")) {
        int player = json_int(r->body, "player", 0);
        if (!set_deck(s, r->body, player)) {
            send_json(fd, 400, "{\"success\":false,\"error\":\"invalid deck\"}");
            return;
        }
        int room_init = s->deck_ready[0] && s->deck_ready[1];
        if (room_init) {
            rb_seed(0xCAFE);
            if (rb_init_game_from_built_decks(&s->state, &s->built_deck[0], &s->built_deck[1]) != 0) {
                send_json(fd, 400, "{\"success\":false,\"error\":\"game initialization failed\"}");
                return;
            }
            s->initialized = 1;
            rb_setup_settle_single_player_state(&s->state);
            s->version++;
        }
        char response[220];
        snprintf(response, sizeof(response),
                 "{\"success\":true,\"room_init\":%s,\"p0_count\":%d,\"p1_count\":%d,\"p0_energy\":%d,\"p1_energy\":%d}",
                 room_init ? "true" : "false",
                 s->built_deck[0].main_count, s->built_deck[1].main_count,
                 s->built_deck[0].energy_count, s->built_deck[1].energy_count);
        send_json(fd, 200, response);
        return;
    }
    if (!strcmp(path, "api/init") && !strcmp(method, "POST")) {
        if (!s->initialized) {
            if (!make_test_deck(s)) {
                send_json(fd, 400, "{\"error\":\"test deck initialization failed\"}");
                return;
            }
        } else {
            rb_seed(0xCAFE);
            if (rb_init_game_from_built_decks(&s->state, &s->built_deck[0], &s->built_deck[1]) != 0) {
                send_json(fd, 400, "{\"error\":\"game initialization failed\"}");
                return;
            }
            rb_setup_settle_single_player_state(&s->state);
            s->version++;
        }
        Buffer b = {0};
        serialize_state(s, &b);
        send_json(fd, 200, b.data ? b.data : "{}");
        buffer_free(&b);
        return;
    }
    if (!strcmp(path, "api/game-state/version")) { char b[64]; snprintf(b, sizeof(b), "{\"version\":%d}", s->version); send_json(fd, 200, b); return; }
    if (!strcmp(path, "api/game-state")) { Buffer b = {0}; if (!s->initialized) make_test_deck(s); serialize_state(s, &b); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/execute-action") && !strcmp(method, "POST")) {
        if (!s->initialized && !make_test_deck(s)) {
            send_json(fd, 400, "{\"error\":\"game initialization failed\"}");
            return;
        }
        RbGeneratedActionList actions = rb_generate_action_candidates(&s->state);
        int index = json_int(r->body, "action_index", -1);
        if (index < 0 || index >= actions.count) {
            rb_free(actions.actions);
            send_json(fd, 400, "{\"error\":\"invalid action index\"}");
            return;
        }
        RbGeneratedAction action = actions.actions[index];
        int requested_type = action.action_type;
        char type_text[32];
        if (parse_json_string_value(r->body, "action_type", type_text, sizeof(type_text))) {
            if (!strcmp(type_text, "pass")) requested_type = 1;
            else if (!strcmp(type_text, "play_member_to_stage")) requested_type = 14;
            else if (!strcmp(type_text, "use_ability")) requested_type = 0;
            else {
                rb_free(actions.actions);
                send_json(fd, 400, "{\"error\":\"unknown action_type\"}");
                return;
            }
            if (requested_type != action.action_type &&
                !(action.action_type == 2 && requested_type == 14)) {
                rb_free(actions.actions);
                send_json(fd, 400, "{\"error\":\"action_type does not match action index\"}");
                return;
            }
        }
        int actor = s->state.active;
        int card_id = action.parameters.card_id;
        int hand_index = -1;
        if (requested_type != 1) {
            hand_index = json_int(r->body, "card_index", -1);
            if (hand_index >= s->state.p[actor].hand.n || hand_index < 0) {
                hand_index = -1;
                for (int i = 0; i < s->state.p[actor].hand.n; i++) {
                    if (s->state.p[actor].hand.cards[i] == card_id) {
                        hand_index = i;
                        break;
                    }
                }
            }
            if (hand_index < 0 || s->state.p[actor].hand.cards[hand_index] != card_id) {
                rb_free(actions.actions);
                send_json(fd, 400, "{\"error\":\"card is not in actor hand\"}");
                return;
            }
        }
        int area = -1;
        char area_text[32];
        if (parse_json_string_value(r->body, "stage_area", area_text, sizeof(area_text)))
            area = rb_member_area_to_index(area_text);
        else if (json_value(r->body, "stage_area"))
            area = json_int(r->body, "stage_area", -1);
        if (requested_type == 14 && (area < 0 || area >= RB_STAGE_SIZE)) {
            rb_free(actions.actions);
            send_json(fd, 400, "{\"error\":\"invalid stage_area\"}");
            return;
        }
        int result;
        if (requested_type == 1) {
            result = rb_execute_main_phase_action(&s->state, 1, -1, 0, 0, 0);
        } else if (requested_type == 14) {
            result = rb_play_member(&s->state, actor, hand_index, area);
        } else if (requested_type == 0) {
            result = rb_execute_main_phase_action(&s->state, 0, card_id, 0, 0, 0);
        } else {
            result = -1;
        }
        rb_free(actions.actions);
        if (result != 0) {
            send_json(fd, 400, "{\"error\":\"action execution failed\"}");
            return;
        }
        s->version++;
        Buffer b = {0};
        serialize_state(s, &b);
        send_json(fd, 200, b.data ? b.data : "{}");
        buffer_free(&b);
        return;
    }
    if (!strcmp(path, "api/ui/config") && !strcmp(method, "POST")) { send_json(fd, 200, "{\"success\":true,\"ui_config\":{}}"); return; }
    if (!strcmp(path, "cards/cards.json")) {
        char card_path[1024];
        snprintf(card_path, sizeof(card_path), "%s/../cards/cards.json", web_root ? web_root : ".");
        char *card_data = NULL;
        size_t card_len = 0;
        if (read_text_file(card_path, &card_data, &card_len)) {
            send_http(fd, "200 OK", "application/json; charset=utf-8", card_data, card_len);
            free(card_data);
        } else {
            send_json(fd, 404, "{\"error\":\"card database not found\"}");
        }
        return;
    }
    if (strstr(path, "..") || !web_root) { send_json(fd, 404, "{\"error\":\"not found\"}"); return; }
    char file[1024]; snprintf(file, sizeof(file), "%s/%s", web_root, path); if (!strcmp(path, "")) snprintf(file, sizeof(file), "%s/index.html", web_root);
    FILE *fp = fopen(file, "rb"); if (!fp) { send_json(fd, 404, "{\"error\":\"not found\"}"); return; } fseek(fp, 0, SEEK_END); long n = ftell(fp); fseek(fp, 0, SEEK_SET); char *data = (char *)malloc((size_t)n); fread(data, 1, (size_t)n, fp); fclose(fp);
    const char *type = strstr(file, ".js") ? "text/javascript" : strstr(file, ".css") ? "text/css" : strstr(file, ".wasm") ? "application/wasm" : strstr(file, ".json") ? "application/json; charset=utf-8" : "text/html"; send_http(fd, "200 OK", type, data, (size_t)n); free(data);
}

int rb_web_server_run(const char *host, int port, const char *web_root, const char *data_dir) {
#ifdef _WIN32
    WSADATA wsa; if (WSAStartup(MAKEWORD(2, 2), &wsa)) return 1;
#endif
    /* ── ORPHANED LOGIC D3: rb_ability_debug_set's production caller ──────────
       Rust engine/src/game/web_server.rs:3652-3655, the first statement of
       `run_web_server_with_ngrok`:

           // Enable structured verdict items in the in-game rule log when RABUKA_RULE_LOG=1
           if std::env::var("RABUKA_RULE_LOG").as_deref() == Ok("1") {
               crate::ability::debug::set_debug(true);
           }

       MEASURED (nm -f posix over all 49 OBJ_LIB objects): `rb_ability_debug_set`
       (src/ability/debug.c:12) had NO caller anywhere in src/ -- its only 23
       callers are tests/, every one of them behind a getenv of its own name. So
       the C engine could not switch ability tracing on at all, and the whole of
       debug.c's buffered trace was dead outside the suite binaries.

       Rust's is the ONE production writer, so this is the faithful call site and
       not an added feature: the same env var, the same "1" comparison, the same
       one-shot-at-startup placement. Placed before rb_load so a rule log that
       fails to open is still visible, exactly as Rust has it before its card
       database load at :3659-3660.

       ⛔ NOT DONE, AND NOT AN OMISSION: nothing else in the C reads an env var
       for this. Rust's other ability-debug reader is
       ABILITY_DEBUG_BATON_TOUCH (engine/src/ability/condition/state.rs:617),
       a SEPARATE flag with its own accessor, so it is not this call's business. */
    {
        const char *rl = getenv("RABUKA_RULE_LOG");
        if (rl && strcmp(rl, "1") == 0) rb_ability_debug_set(1);
    }
    if (rb_load(data_dir) != 0) return 1;
    Sandbox *s = (Sandbox *)calloc(1, sizeof(Sandbox)); if (!s) return 1; make_test_deck(s);
    SOCKET server = socket(AF_INET, SOCK_STREAM, 0); int yes = 1; setsockopt(server, SOL_SOCKET, SO_REUSEADDR, (const char *)&yes, sizeof(yes));
    struct sockaddr_in addr; memset(&addr, 0, sizeof(addr)); addr.sin_family = AF_INET; addr.sin_port = htons((unsigned short)port); inet_pton(AF_INET, host, &addr.sin_addr);
    if (bind(server, (struct sockaddr *)&addr, sizeof(addr)) == SOCKET_ERROR || listen(server, 16) == SOCKET_ERROR) return 1;
    fprintf(stderr, "rb_web_server listening on http://%s:%d (room SANDBX)\n", host, port);
    for (;;) { SOCKET fd = accept(server, NULL, NULL); if (fd == INVALID_SOCKET) continue; HttpRequest r; memset(&r, 0, sizeof(r)); r.socket = (int)fd; size_t got = recv(fd, r.request, sizeof(r.request) - 1, 0); r.request[got] = 0; const char *cl = strstr(r.request, "Content-Length:"); if (cl) { size_t n = (size_t)strtoul(cl + 15, NULL, 10); if (n > RB_WEB_MAX_BODY) n = RB_WEB_MAX_BODY; const char *start = strstr(r.request, "\r\n\r\n"); size_t already = start ? (got - (size_t)(start + 4 - r.request)) : 0; r.body = (char *)malloc(n + 1); if (already >= n) { memcpy(r.body, start + 4, n); r.body_len = n; } else { if (already) memcpy(r.body, start + 4, already); r.body_len = already; while (r.body_len < n) { size_t more = recv(fd, r.body + r.body_len, n - r.body_len, 0); if (!more) break; r.body_len += more; } } r.body[r.body_len] = 0; } handle_request(s, fd, &r, web_root); free(r.body); closesocket(fd); }
}

#ifdef __CYGWIN__
int WinMain(void *instance, void *prev, void *cmdline, int show) {
    (void)instance; (void)prev; (void)show;
    return rb_web_server_run("127.0.0.1", 18080, "..\\web_ui", "src");
}
#endif
