#include "web_server.h"
#include "deck_parser.h"
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
    uint32_t deck[2][RB_MAX_DECK];
    int deck_count[2];
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
static int find_card_for_deck(const char *raw) {
    char normalized[128];
    const char *card_no = raw;
    int direct;
    if (rb_normalize_card_no(raw, normalized, sizeof(normalized)) == 0) card_no = normalized;
    direct = rb_find_card_by_no(card_no);
    if (direct >= 0) return direct;
    for (uint32_t i = 0; i < rb_num_cards(); i++) {
        Card c;
        memset(&c, 0, sizeof(c));
        if (!rb_decode_card_by_index(i, &c)) continue;
        const char *candidate = rb_card_string(c.card_no_idx);
        const unsigned char *a = (const unsigned char *)card_no;
        const unsigned char *b = (const unsigned char *)candidate;
        size_t remaining = strlen(card_no);
        int match = 1;
        while (remaining > 0) {
            if (*a == '+' && b[0] == 0xef && b[1] == 0xbc && b[2] == 0x9b) { a++; b += 3; remaining--; continue; }
            if (*b == '+' && a[0] == 0xef && a[1] == 0xbc && a[2] == 0x9b) { b++; a += 3; remaining -= 3; continue; }
            if (!*b || tolower(*a) != tolower(*b)) { match = 0; break; }
            a++; b++; remaining--;
        }
        rb_free_card(&c);
        if (match) return (int)i;
    }
    return -1;
}

static const char *phase_name(const GameState *g) { return rb_phase_name((int)g->phase); }

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
static void append_player(Buffer *b, const RbPlayer *p) {
    buffer_put(b, "{\"hand\":"); append_bag(b, &p->hand, NULL);
    buffer_put(b, ",\"main_deck_count\":"); buffer_putf(b, "%d", p->deck.n);
    buffer_put(b, ",\"energy_deck_count\":"); buffer_putf(b, "%d", p->energy_deck.n);
    buffer_put(b, ",\"energy\":"); append_bag(b, &p->energy, "Active");
    buffer_put(b, ",\"live_zone\":"); append_bag(b, &p->live, NULL);
    buffer_put(b, ",\"success_live_card_zone\":"); append_bag(b, &p->success, NULL);
    buffer_put(b, ",\"discard\":"); append_bag(b, &p->discard, NULL);
    buffer_put(b, ",\"stage\":{\"left_side\":"); if (p->stage[0] >= 0) append_card(b, p->stage[0], p->stage_wait[0] ? "Wait" : "Active"); else buffer_put(b, "null");
    buffer_put(b, ",\"center\":"); if (p->stage[1] >= 0) append_card(b, p->stage[1], p->stage_wait[1] ? "Wait" : "Active"); else buffer_put(b, "null");
    buffer_put(b, ",\"right_side\":"); if (p->stage[2] >= 0) append_card(b, p->stage[2], p->stage_wait[2] ? "Wait" : "Active"); else buffer_put(b, "null");
    buffer_put(b, ",\"left_under\":"); append_bag(b, &p->under_cards[0], NULL); buffer_put(b, ",\"center_under\":"); append_bag(b, &p->under_cards[1], NULL); buffer_put(b, ",\"right_under\":"); append_bag(b, &p->under_cards[2], NULL); buffer_put(b, "},\"score\":");
    buffer_putf(b, "%d,\"life\":%d}", p->score, p->life);
}
static void append_action(Buffer *b, const RbGeneratedAction *a, int index) {
    const char *type = a->action_type == 1 ? "pass" : a->action_type == 2 ? "play_member_to_stage" : "use_ability";
    buffer_putf(b, "{\"index\":%d,\"action_type\":", index); json_string(b, type);
    buffer_put(b, ",\"description\":"); json_string(b, a->action_type == 1 ? "Pass" : "Action"); buffer_put(b, ",\"parameters\":{");
    if (a->has_parameters) { buffer_put(b, "\"card_id\":"); buffer_putf(b, "%d", a->parameters.card_id); buffer_put(b, ",\"available_areas\":[");
        for (int i = 0; i < a->parameters.n_available_areas; i++) buffer_putf(b, "%s{\"area\":%d,\"available\":%s,\"use_baton_touch\":%s}", i ? "," : "", i, a->parameters.available_areas[i].available ? "true" : "false", a->parameters.available_areas[i].is_baton_touch ? "true" : "false");
        buffer_put(b, "]"); }
    buffer_put(b, "}}");
}
static int serialize_state(Sandbox *s, Buffer *b) {
    RbGeneratedActionList actions = rb_generate_action_candidates(&s->state);
    buffer_put(b, "{\"state_id\":\"c-"); buffer_putf(b, "%d", s->version); buffer_put(b, "\",\"frame_counter\":"); buffer_putf(b, "%d", s->version);
    buffer_put(b, ",\"phase\":"); json_string(b, phase_name(&s->state)); buffer_put(b, ",\"active_player\":"); buffer_putf(b, "%d", s->state.active + 1);
    buffer_put(b, ",\"winner\":"); if (s->state.winner < 0) buffer_put(b, "null"); else buffer_putf(b, "%d", s->state.winner);
    buffer_put(b, ",\"player1\":"); append_player(b, &s->state.p[0]); buffer_put(b, ",\"player2\":"); append_player(b, &s->state.p[1]);
    buffer_put(b, ",\"legal_actions\":["); for (int i = 0; i < actions.count; i++) { if (i) buffer_put(b, ","); append_action(b, &actions.actions[i], i); } buffer_put(b, "]");
    buffer_put(b, ",\"ui_config\":{\"perspective_player\":0,\"current_lang\":\"jp\"}}");
    rb_free(actions.actions); return 1;
}
static void send_http(int fd, const char *status, const char *type, const char *body, size_t len) {
    char head[512]; int n = snprintf(head, sizeof(head), "HTTP/1.1 %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\nAccess-Control-Allow-Origin: *\r\n\r\n", status, type, len);
    send(fd, head, (size_t)n, 0); if (len) send(fd, body, len, 0);
}
static void send_json(int fd, int code, const char *json) { char st[32]; snprintf(st, sizeof(st), code == 200 ? "200 OK" : code == 404 ? "404 Not Found" : "400 Bad Request"); send_http(fd, st, "application/json; charset=utf-8", json, strlen(json)); }
static const char *json_value(const char *body, const char *key) { char needle[80]; snprintf(needle, sizeof(needle), "\"%s\"", key); const char *p = strstr(body, needle); if (!p) return NULL; p = strchr(p, ':'); return p ? p + 1 : NULL; }
static int json_int(const char *body, const char *key, int def) { const char *p = json_value(body, key); return p ? atoi(p) : def; }
static void make_test_deck(Sandbox *s) {
    memset(s->deck, 0, sizeof(s->deck)); s->deck_count[0] = s->deck_count[1] = 0;
    for (uint32_t i = 0; i < rb_num_cards() && s->deck_count[0] < 40; i++) if (rb_card_ability_idx(i) != 0xFFFF) s->deck[0][s->deck_count[0]++] = i;
    for (uint32_t i = 0; i < rb_num_cards() && s->deck_count[1] < 40; i++) if (rb_card_ability_idx(i) != 0xFFFF) s->deck[1][s->deck_count[1]++] = i;
    rb_seed(0xCAFE); memset(&s->state, 0, sizeof(s->state)); rb_game_init(&s->state, s->deck[0], s->deck_count[0], s->deck[1], s->deck_count[1]); s->initialized = 1; s->deck_ready[0] = s->deck_ready[1] = 1; s->version++;
}
static void set_deck(Sandbox *s, const char *body, int player) {
    if (player < 0 || player > 1) return;
    s->deck_count[player] = 0;
    const char *p = json_value(body, "deck"); if (!p) return; p = strchr(p, '['); if (!p) return;
    fprintf(stderr, "[SET_DECK] player=%d body=%s\n", player, body);
    p++; char card[128];
    while (s->deck_count[player] < RB_MAX_DECK && *p && *p != ']') {
        while (*p == ' ' || *p == ',' || *p == '\n' || *p == '\r' || *p == '\t') p++;
        if (*p == '"') { p++; size_t n = 0; while (*p && *p != '"' && n + 1 < sizeof(card)) card[n++] = *p++; card[n] = 0; if (*p == '"') p++; int idx = find_card_for_deck(card); fprintf(stderr, "[SET_DECK] card=%s idx=%d\n", card, idx); if (idx >= 0) s->deck[player][s->deck_count[player]++] = (uint32_t)idx; }
        while (*p && *p != ',' && *p != ']') p++;
    }
    s->deck_ready[player] = s->deck_count[player] > 0;
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
            if (!strstr(card_no, "-PE") && !strstr(card_no, "-E")) continue;
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
    if (!strcmp(path, "api/status")) { char status[192]; snprintf(status, sizeof(status), "{\"status\":\"c_server\",\"cards\":%u,\"niji\":%d,\"instance_id\":\"c-web-single-room\"}", rb_num_cards(), rb_find_card_by_no("PL!N-bp1-026-L")); send_json(fd, 200, status); return; }
    if (!strcmp(path, "api/rooms/create")) { s->deck_count[0] = s->deck_count[1] = 0; s->deck_ready[0] = s->deck_ready[1] = 0; send_json(fd, 200, "{\"success\":true,\"room_id\":\"SANDBX\",\"session\":{\"token\":\"sandbox\",\"player_id\":0}}"); return; }
    if (!strcmp(path, "api/rooms/list")) { send_json(fd, 200, "{\"rooms\":[{\"room_id\":\"SANDBX\",\"mode\":\"sandbox\",\"players\":1}]}"); return; }
    if (!strcmp(path, "api/rooms/join") || !strcmp(path, "api/rooms/leave")) { send_json(fd, 200, "{\"success\":true,\"room_id\":\"SANDBX\"}"); return; }
    if (!strcmp(path, "api/get_decks")) { Buffer b = {0}; buffer_put(&b, "{\"success\":true,"); append_deck_presets(&b, web_root); buffer_put(&b, "}"); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/get_test_deck")) { Buffer b = {0}; buffer_put(&b, "{\"success\":true,\"content\":["); append_deck_file_content(&b, web_root, "nijigaku_cup.txt"); buffer_put(&b, "]}"); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/set_deck") && !strcmp(method, "POST")) { int player = json_int(r->body, "player", 0); set_deck(s, r->body, player); int room_init = s->deck_ready[0] && s->deck_ready[1]; if (room_init) { rb_seed(0xCAFE); memset(&s->state, 0, sizeof(s->state)); rb_game_init(&s->state, s->deck[0], s->deck_count[0], s->deck[1], s->deck_count[1]); s->initialized = 1; s->version++; } char response[160]; snprintf(response, sizeof(response), "{\"success\":true,\"room_init\":%s,\"p0_count\":%d,\"p1_count\":%d,\"body_len\":%d}", room_init ? "true" : "false", s->deck_count[0], s->deck_count[1], (int)r->body_len); send_json(fd, 200, response); return; }
    if (!strcmp(path, "api/init") && !strcmp(method, "POST")) { if (!s->initialized) make_test_deck(s); else { rb_seed(0xCAFE); memset(&s->state, 0, sizeof(s->state)); rb_game_init(&s->state, s->deck[0], s->deck_count[0], s->deck[1], s->deck_count[1]); s->version++; } Buffer b = {0}; serialize_state(s, &b); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/game-state/version")) { char b[64]; snprintf(b, sizeof(b), "{\"version\":%d}", s->version); send_json(fd, 200, b); return; }
    if (!strcmp(path, "api/game-state")) { Buffer b = {0}; if (!s->initialized) make_test_deck(s); serialize_state(s, &b); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return; }
    if (!strcmp(path, "api/execute-action") && !strcmp(method, "POST")) {
        if (!s->initialized) make_test_deck(s);
        RbGeneratedActionList a = rb_generate_action_candidates(&s->state); int idx = json_int(r->body, "action_index", 0);
        if (idx < 0 || idx >= a.count) { rb_free(a.actions); send_json(fd, 400, "{\"error\":\"invalid action index\"}"); return; }
        RbGeneratedAction action = a.actions[idx]; int area = json_int(r->body, "stage_area", 0); if (area < 0) area = 0;
        if (action.action_type == 1) rb_execute_main_phase_action(&s->state, 1, -1, 0, 0, 0);
        else if (action.action_type == 2) { int hi = json_int(r->body, "card_index", -1); if (hi < 0) for (int i = 0; i < s->state.p[s->state.active].hand.n; i++) if (s->state.p[s->state.active].hand.cards[i] == action.parameters.card_id) { hi = i; break; } rb_play_member(&s->state, s->state.active, hi, area); }
        else rb_execute_main_phase_action(&s->state, 0, action.parameters.card_id, area, 0, 0);
        rb_free(a.actions); s->version++; Buffer b = {0}; serialize_state(s, &b); send_json(fd, 200, b.data ? b.data : "{}"); buffer_free(&b); return;
    }
    if (!strcmp(path, "api/ui/config") && !strcmp(method, "POST")) { send_json(fd, 200, "{\"success\":true,\"ui_config\":{}}"); return; }
    if (strstr(path, "..") || !web_root) { send_json(fd, 404, "{\"error\":\"not found\"}"); return; }
    char file[1024]; snprintf(file, sizeof(file), "%s/%s", web_root, path); if (!strcmp(path, "")) snprintf(file, sizeof(file), "%s/index.html", web_root);
    FILE *fp = fopen(file, "rb"); if (!fp) { send_json(fd, 404, "{\"error\":\"not found\"}"); return; } fseek(fp, 0, SEEK_END); long n = ftell(fp); fseek(fp, 0, SEEK_SET); char *data = (char *)malloc((size_t)n); fread(data, 1, (size_t)n, fp); fclose(fp);
    const char *type = strstr(file, ".js") ? "text/javascript" : strstr(file, ".css") ? "text/css" : strstr(file, ".wasm") ? "application/wasm" : "text/html"; send_http(fd, "200 OK", type, data, (size_t)n); free(data);
}

int rb_web_server_run(const char *host, int port, const char *web_root, const char *data_dir) {
#ifdef _WIN32
    WSADATA wsa; if (WSAStartup(MAKEWORD(2, 2), &wsa)) return 1;
#endif
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
