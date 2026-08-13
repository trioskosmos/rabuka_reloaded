// Rabuka 3DS — C shim using citro2d for board rendering + text display.
//
// FONT SCALING REFERENCE:
// The BCFNT font (romfs:/font.bcfnt) has native cellHeight=42px.
// citro2d normalizes ALL fonts via: textScale = 30.0 / cellHeight.
// The user-supplied scale is then multiplied by textScale internally,
// so the final rendered glyph height is always: scale * 30.0 pixels.
//
//   Scale   Glyph Height   Use Case
//   -----   --------------   --------
//   0.50      15px          Too small for 3DS screens
//   0.60      18px          Barely readable minimum
//   0.65      20px          Zone labels, body text
//   0.70      21px          Deck list items, utility counts
//   0.75      23px          Menu items, ability queue header
//   0.80      24px          Card names in detail view
//   0.85      26px          Titles, CLI mode text
//   1.00      30px          Full system-font size
//
// Line advance ≈ ceil(scale * 0.714 * 31) px (depends on font lineFeed).
// Top screen: 400x240. Bottom screen: 320x240.

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <3ds.h>
#include <citro2d.h>
#include <errno.h>
#include <tremor/ivorbisfile.h>
#include "fbi_task.h"
#include "fbi_capturecam.h"

u32 __ctru_heap_size = 64 * 1024 * 1024;
u32 __stacksize__ = 2 * 1024 * 1024;

// ---- QR forward declarations (defined in QR section below) ----
typedef struct rabuka_qr_data_s rabuka_qr_data;
void _3ds_qr_free(void *p);
static rabuka_qr_data *g_rqr = NULL;

// ---- Text rendering (top screen: stats + card info) ----
#define TEXTLEN 8192
static char top_text[TEXTLEN];
static C3D_RenderTarget* top_target = NULL;
static C3D_RenderTarget* top_target_right = NULL;
static C3D_RenderTarget* bot_target = NULL;
static C2D_TextBuf top_buf = NULL;
static C2D_Text top_obj;
static bool top_parsed = false;
static bool text_dirty = true;
static C2D_Font custom_font = NULL;

// ---- Board rendering (bottom screen) ----
#define MAX_CACHED_ATLAS 64
#define MAX_SLOTS 32

typedef struct {
    char name[64];
    C2D_SpriteSheet sheet;
} CachedAtlas;

static CachedAtlas atlases[MAX_CACHED_ATLAS];
static int atlas_count = 0;

typedef struct {
    bool active;
    char atlas[64];
    int index;
    bool landscape;
    bool tapped;
    bool flipped;
    int score;           // live card total score (base + modifiers)
    char stat_text[128]; // icon markup for score + need hearts
} CardSlot;

typedef struct {
    CardSlot stage[3];
    CardSlot live[3];
    CardSlot energy[MAX_SLOTS];
    int energy_count;
    CardSlot hand[MAX_SLOTS];
    int hand_count;
    int deck, edeck, discard, success;
} PlayerBoard;

static bool board_mode = false;
static int board_view = 2;  // default to both players
static float section_y0 = 0, section_h = 240;
static int top_scroll_y = 0;          // 0=player, 1=opponent, 2=both
static int board_sel_slot = -1;
static int board_sel_type = 0;
static bool section_opponent = false;

static PlayerBoard p_board;  // player (self)
static PlayerBoard o_board;  // opponent

// Colors
static u32 COL_BG        = 0xFF0F141E; // navy
static u32 COL_ZONE_BG   = 0x221A2333; // zone background
static u32 COL_ZONE_BDR  = 0xFF2A3A5C; // zone border
static u32 COL_SEL       = 0xFFF59E0B; // gold selection
static u32 COL_TEXT      = 0xFFD1D5DB; // light grey text
static u32 COL_GOLD      = 0xFFF59E0B; // gold accent
static u32 COL_BLUE      = 0xFF4A9EFF; // blue accent
static u32 COL_GREEN     = 0xFF2ECC71; // green
static u32 COL_PINK      = 0xFFFF55AA; // pink
static u32 COL_PRPL      = 0xFF9B59B6; // purple
static u32 COL_TAPPED    = 0xAA000000; // tapped overlay

// ---- Game mode state (vs CLI debug mode) ----
static bool cli_mode = false;

// ---- Top screen draw-op queue (used when !cli_mode) ----
#define MAX_DRAW_OPS 256
typedef struct {
    float x, y, w, h;
    u32 color;
    float scale;
    const char *text;  // points into string_pool — no per-op buffer needed
    const char *atlas; // atlas name for card images
    int   atlas_idx;
} DrawOp;
#define OP_RECT 0
#define OP_TEXT 1
#define OP_CARD 2
static DrawOp draw_ops[MAX_DRAW_OPS];
static int   draw_op_count = 0;
static int   draw_op_types[MAX_DRAW_OPS];

// String pool: one 32KB slab reused every frame.  Draw ops borrow pointers
// into this pool.  _3ds_top_clear() resets the pool each frame.
#define STRING_POOL_SIZE (32 * 1024)
static char string_pool[STRING_POOL_SIZE];
static int   string_pool_pos = 0;
static const char* pool_strdup(const char* s) {
    if (!s) return "";
    int len = 0;
    while (s[len]) len++;
    len++;  /* include NUL */
    if (string_pool_pos + len > STRING_POOL_SIZE) return "";
    char* dest = &string_pool[string_pool_pos];
    memcpy(dest, s, len);
    string_pool_pos += len;
    return dest;
}
static u32   COL_TOP_BG = 0xFF0A0E1A; // very dark navy

// ---- Bottom screen draw-op queue (setup menus before board mode) ----
// Mirrors the top draw-op queue but renders to the bottom screen when the
// board is not active (board_mode == false). Used by setup-phase menus so
// they appear on the touch screen.
#define MAX_BOT_DRAW_OPS 256
#define BOT_STRING_POOL_SIZE (32 * 1024)
typedef struct {
    float x, y, w, h;
    u32 color;
    float scale;
    const char *text;  // points into bot_string_pool
} BotDrawOp;
static BotDrawOp bot_draw_ops[MAX_BOT_DRAW_OPS];
static int   bot_draw_op_count = 0;
static int   bot_draw_op_types[MAX_BOT_DRAW_OPS];
static char  bot_string_pool[BOT_STRING_POOL_SIZE];
static int   bot_string_pool_pos = 0;
static const char* bot_pool_strdup(const char* s) {
    if (!s) return "";
    int len = 0;
    while (s[len]) len++;
    len++;  /* include NUL */
    if (bot_string_pool_pos + len > BOT_STRING_POOL_SIZE) return "";
    char* dest = &bot_string_pool[bot_string_pool_pos];
    memcpy(dest, s, len);
    bot_string_pool_pos += len;
    return dest;
}

// ---- Action highlight on board slots (multiple) ----
#define MAX_HIGHLIGHTS 16
static int hl_count = 0;
static int hl_zones[MAX_HIGHLIGHTS];
static int hl_slots[MAX_HIGHLIGHTS];
static bool hl_opponent[MAX_HIGHLIGHTS];

// ---- Need hearts counts per player (drawn next to live zone) ----
static u32 need_hearts_counts[2][8] = {{0}};
void _3ds_set_need_hearts(int player, u32 h0, u32 h1, u32 h2, u32 h3, u32 h4, u32 h5, u32 h6, u32 h7) {
    if (player < 0 || player > 1) return;
    u32 vals[8] = {h0, h1, h2, h3, h4, h5, h6, h7};
    for (int i = 0; i < 8; i++) need_hearts_counts[player][i] = vals[i];
}

// ---- Action overlay (Phase 2: show actions on bottom screen) ----
#define MAX_OVERLAY_LINES 16
#define OVERLAY_LINE_LEN  48
static int   overlay_count = 0;
static int   overlay_selected = -1;
static char  overlay_lines[MAX_OVERLAY_LINES][OVERLAY_LINE_LEN];

// ---- Temp text buffer for top screen game-mode rendering ----
static C2D_TextBuf  tmp_text_buf = NULL;
static C2D_Text     tmp_text_obj;

// ---- Forward declarations ----
C2D_Image _3ds_get_card_image(const char* atlas_name, int index);
static void zone_heights(float h, float* live, float* stage, float* energy, float* hand);
void _3ds_qr_draw_preview(float x_off);

// ---- init / exit ----
void _3ds_init() {
    gfxInitDefault();
    gfxSet3D(true);
    task_init();
    C3D_Init(C3D_DEFAULT_CMDBUF_SIZE);
    C2D_Init(C2D_DEFAULT_MAX_OBJECTS);
    C2D_Prepare();

    top_target = C2D_CreateScreenTarget(GFX_TOP, GFX_LEFT);
    top_target_right = C2D_CreateScreenTarget(GFX_TOP, GFX_RIGHT);
    bot_target = C2D_CreateScreenTarget(GFX_BOTTOM, GFX_LEFT);

    top_buf = C2D_TextBufNew(4096);
    top_text[0] = '\0';

    romfsInit();
    custom_font = C2D_FontLoad("romfs:/font.bcfnt");

    // Initialize game-mode draw queue + overlay
    atlas_count = 0;
    board_mode = false;
    cli_mode = false;
    draw_op_count = 0;
    overlay_count = 0;
    bot_draw_op_count = 0;
    bot_string_pool_pos = 0;

    hl_count = 0;
    tmp_text_buf = C2D_TextBufNew(32768);
}

// Measure per-line height for the custom font at scale 0.85.
// Parses a two-line string, gets total height, divides by 2.
// At scale 0.85: ceil(0.85 * (30/42) * 31) = 19px per line.
// Fallback 30.0px if measurement fails (matches system font at scale 1.0).
float _3ds_bot_line_height() {
    C2D_Font f = custom_font ? custom_font : NULL;
    C2D_TextBufClear(tmp_text_buf);
    C2D_Text t;
    C2D_TextFontParse(&t, f, tmp_text_buf, "A\nA\0");
    C2D_TextOptimize(&t);
    float w, h;
    C2D_TextGetDimensions(&t, 0.85f, 0.85f, &w, &h);
    if (h <= 0) return 30.0f;
    return h / 2.0f;
}

void _3ds_exit() {
    if (g_rqr) { _3ds_qr_free(g_rqr); g_rqr = NULL; }
    for (int i = 0; i < atlas_count; i++) {
        C2D_SpriteSheetFree(atlases[i].sheet);
    }
    C2D_TextBufDelete(top_buf);
    if (tmp_text_buf) C2D_TextBufDelete(tmp_text_buf);
    if (custom_font) C2D_FontFree(custom_font);
    C2D_Fini();
    C3D_Fini();
    task_exit();
    romfsExit();
    gfxExit();
}

// ---- Text API (top screen) ----
void _3ds_text_add_top(const char* msg) {
    strncat(top_text, msg, TEXTLEN - strlen(top_text) - 1);
    text_dirty = true;
}

void _3ds_text_set_scroll_y(int y) { top_scroll_y = y; }
int  _3ds_text_get_scroll_y() { return top_scroll_y; }

void _3ds_text_add_bot(const char* msg) {
    // Board mode: redirect to top (debug info)
    strncat(top_text, msg, TEXTLEN - strlen(top_text) - 1);
    text_dirty = true;
}

void _3ds_clear_top() {
    top_text[0] = '\0';
    text_dirty = true;
}

void _3ds_clear_both() {
    top_text[0] = '\0';
    text_dirty = true;
}

// ---- Board API ----
static void set_slot(CardSlot* slot, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    slot->active = active;
    if (atlas) { strncpy(slot->atlas, atlas, 63); slot->atlas[63] = '\0'; }
    else { slot->atlas[0] = '\0'; }
    slot->index = index;
    slot->landscape = landscape;
    slot->tapped = tapped;
    slot->score = 0;
    slot->stat_text[0] = '\0';
}

static void set_player_slot(PlayerBoard* pb, CardSlot slots[], int i, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    if (i >= 0 && i < MAX_SLOTS) set_slot(&slots[i], active, atlas, index, landscape, tapped);
}

void _3ds_board_enable(bool on) { board_mode = on; }

void _3ds_board_cycle_view() {
    board_view = (board_view + 1) % 3;  // 0=player, 1=opponent, 2=both
}
int  _3ds_board_current_view() { return board_view; }

void _3ds_board_clear_cache() {
    for (int i = 0; i < atlas_count; i++) C2D_SpriteSheetFree(atlases[i].sheet);
    atlas_count = 0;
}

// Player slots
void _3ds_board_set_stage(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    if (i >= 0 && i < 3) set_slot(&p_board.stage[i], a, atlas, idx, l, t);
}
void _3ds_board_set_live(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    if (i >= 0 && i < 3) set_slot(&p_board.live[i], a, atlas, idx, l, t);
}
void _3ds_board_set_live_stats(int i, int score, const char* stat_text) {
    if (i < 0 || i >= 3) return;
    p_board.live[i].score = score;
    if (stat_text) { strncpy(p_board.live[i].stat_text, stat_text, 127); p_board.live[i].stat_text[127] = '\0'; }
    else { p_board.live[i].stat_text[0] = '\0'; }
}
void _3ds_board_set_opp_live_stats(int i, int score, const char* stat_text) {
    if (i < 0 || i >= 3) return;
    o_board.live[i].score = score;
    if (stat_text) { strncpy(o_board.live[i].stat_text, stat_text, 127); o_board.live[i].stat_text[127] = '\0'; }
    else { o_board.live[i].stat_text[0] = '\0'; }
}
void _3ds_board_set_energy(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    set_player_slot(&p_board, p_board.energy, i, a, atlas, idx, l, t);
}
void _3ds_board_set_energy_count(int c) { p_board.energy_count = c > MAX_SLOTS ? MAX_SLOTS : c; }
void _3ds_board_set_hand(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    set_player_slot(&p_board, p_board.hand, i, a, atlas, idx, l, t);
}
void _3ds_board_set_hand_count(int c) { p_board.hand_count = c > MAX_SLOTS ? MAX_SLOTS : c; }
void _3ds_board_set_utility(int deck, int edeck, int discard, int success) {
    p_board.deck = deck; p_board.edeck = edeck; p_board.discard = discard; p_board.success = success;
}

// Opponent slots
void _3ds_board_set_opp_stage(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    if (i >= 0 && i < 3) set_slot(&o_board.stage[i], a, atlas, idx, l, t);
}
void _3ds_board_set_opp_live(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    if (i >= 0 && i < 3) set_slot(&o_board.live[i], a, atlas, idx, l, t);
}
void _3ds_board_set_opp_energy(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    set_player_slot(&o_board, o_board.energy, i, a, atlas, idx, l, t);
}
void _3ds_board_set_opp_energy_count(int c) { o_board.energy_count = c > MAX_SLOTS ? MAX_SLOTS : c; }
void _3ds_board_set_opp_hand(int i, bool a, const char* atlas, int idx, bool l, bool t) {
    set_player_slot(&o_board, o_board.hand, i, a, atlas, idx, l, t);
}
void _3ds_board_set_opp_hand_count(int c) { o_board.hand_count = c > MAX_SLOTS ? MAX_SLOTS : c; }
void _3ds_board_set_opp_utility(int deck, int edeck, int discard, int success) {
    o_board.deck = deck; o_board.edeck = edeck; o_board.discard = discard; o_board.success = success;
}

void _3ds_board_set_selection(int slot_idx, int slot_type) {
    board_sel_slot = slot_idx;
    board_sel_type = slot_type;
}

// Hand scroll range + arrow tracking
static int hand_range_vis = 0;
static int hand_range_off = 0;
static int hand_range_total = 0;

void _3ds_board_set_hand_scroll_info(int visible, int offset, int total) {
    hand_range_vis = visible;
    hand_range_off = offset;
    hand_range_total = total;
}

float _3ds_board_get_slot_w(int zone_type) {
    const float PORTRAIT = 0.711f;
    const float LANDSCAPE = 1.41f;
    float live_h, stage_h, energy_h, hand_h;
    zone_heights(section_h, &live_h, &stage_h, &energy_h, &hand_h);
    switch (zone_type) {
        case 0: { float h = live_h - 4.0f; return h < 0 ? 0 : h * LANDSCAPE; } // live
        case 1: { float h = stage_h - 4.0f; return h < 0 ? 0 : h; }           // stage (square)
        case 2: { float h = energy_h - 4.0f; return h < 0 ? 0 : h * PORTRAIT; } // energy (portrait)
        case 3: { float h = hand_h - 4.0f; return h < 0 ? 0 : h * PORTRAIT; } // hand
        default: return 0;
    }
}

// ---- CLI mode toggle ----
void _3ds_set_cli_mode(bool cli) { cli_mode = cli; }
bool _3ds_is_cli_mode() { return cli_mode; }

// ---- Top screen draw-op queue (game mode) ----
// Queued draw ops are rendered in _3ds_swap_buffers. Text ops carry a scale
// value which citro2d interprets as: glyph_height = scale * 30.0 pixels.
// This queue is cleared and re-filled every frame by the Rust game loop.
void _3ds_top_clear() { draw_op_count = 0; string_pool_pos = 0; }

void _3ds_top_queue_rect(float x, float y, float w, float h, u32 color) {
    if (draw_op_count >= MAX_DRAW_OPS) return;
    int i = draw_op_count++;
    draw_op_types[i] = OP_RECT;
    draw_ops[i].x = x; draw_ops[i].y = y;
    draw_ops[i].w = w; draw_ops[i].h = h;
    draw_ops[i].color = color;
}

void _3ds_top_queue_text(float x, float y, u32 color, float scale, const char* text) {
    if (!text || draw_op_count >= MAX_DRAW_OPS) return;
    int i = draw_op_count++;
    draw_op_types[i] = OP_TEXT;
    draw_ops[i].x = x; draw_ops[i].y = y;
    draw_ops[i].color = color; draw_ops[i].scale = scale;
    draw_ops[i].text = pool_strdup(text);
}

void _3ds_top_queue_card(const char* atlas, int idx, float x, float y, float w, float h) {
    if (!atlas || draw_op_count >= MAX_DRAW_OPS) return;
    int i = draw_op_count++;
    draw_op_types[i] = OP_CARD;
    draw_ops[i].x = x; draw_ops[i].y = y; draw_ops[i].w = w; draw_ops[i].h = h;
    draw_ops[i].atlas = pool_strdup(atlas);
    draw_ops[i].atlas_idx = idx;
}

void _3ds_bot_clear() { bot_draw_op_count = 0; bot_string_pool_pos = 0; }

void _3ds_bot_queue_rect(float x, float y, float w, float h, u32 color) {
    if (bot_draw_op_count >= MAX_BOT_DRAW_OPS) return;
    int i = bot_draw_op_count++;
    bot_draw_op_types[i] = OP_RECT;
    bot_draw_ops[i].x = x; bot_draw_ops[i].y = y;
    bot_draw_ops[i].w = w; bot_draw_ops[i].h = h;
    bot_draw_ops[i].color = color;
}

void _3ds_bot_queue_text(float x, float y, u32 color, float scale, const char* text) {
    if (!text || bot_draw_op_count >= MAX_BOT_DRAW_OPS) return;
    int i = bot_draw_op_count++;
    bot_draw_op_types[i] = OP_TEXT;
    bot_draw_ops[i].x = x; bot_draw_ops[i].y = y;
    bot_draw_ops[i].color = color; bot_draw_ops[i].scale = scale;
    bot_draw_ops[i].text = bot_pool_strdup(text);
}

static C2D_Image _atlas_get_image(const char* atlas, int idx) {
    for (int i = 0; i < atlas_count; i++) {
        if (strcmp(atlases[i].name, atlas) == 0)
            return C2D_SpriteSheetGetImage(atlases[i].sheet, (size_t)idx);
    }
    C2D_Image img = {0};
    return img;
}



// ---- Action highlight ----
void _3ds_board_set_action_highlight(int zone, int slot, bool opponent) {
    if (hl_count < MAX_HIGHLIGHTS) {
        hl_zones[hl_count] = zone;
        hl_slots[hl_count] = slot;
        hl_opponent[hl_count] = opponent;
        hl_count++;
    }
}
void _3ds_board_clear_action_highlight() { hl_count = 0; }
static bool _is_highlighted(int zone, int slot, bool opponent) {
    for (int i = 0; i < hl_count; i++) {
        if (hl_zones[i] == zone && hl_slots[i] == slot && hl_opponent[i] == opponent) return true;
    }
    return false;
}

// ---- Action overlay (safe: copies strings into C buffer) ----
// action_idx_map[i] = index into the flat action list for display line i.
// This lets grouped/reordered display lines map back to correct action indices.
static int action_idx_map[MAX_OVERLAY_LINES];

void _3ds_board_set_action_overlay_state(int count, int selected) {
    overlay_count = count > MAX_OVERLAY_LINES ? MAX_OVERLAY_LINES : (count < 0 ? 0 : count);
    overlay_selected = selected;
}
void _3ds_board_set_action_overlay_text(int index, const char* text) {
    if (index >= 0 && index < MAX_OVERLAY_LINES && text) {
        strncpy(overlay_lines[index], text, OVERLAY_LINE_LEN - 1);
        overlay_lines[index][OVERLAY_LINE_LEN - 1] = '\0';
    }
}
void _3ds_board_set_overlay_action_idx(int display_line, int action_index) {
    if (display_line >= 0 && display_line < MAX_OVERLAY_LINES) {
        action_idx_map[display_line] = action_index;
    }
}
int _3ds_board_get_overlay_action_idx(int display_line) {
    if (display_line >= 0 && display_line < MAX_OVERLAY_LINES) {
        return action_idx_map[display_line];
    }
    return -1;
}
void _3ds_board_clear_action_overlay() {
    overlay_count = 0;
    overlay_selected = -1;
}
int _3ds_board_get_overlay_selected() {
    return overlay_selected;
}

void _3ds_board_set_section_rect(float y0, float h, bool opponent) {
    section_y0 = y0; section_h = h; section_opponent = opponent;
}

int _3ds_board_get_zone_y(int zone_type) {
    float live_h, stage_h, energy_h, hand_h;
    zone_heights(section_h, &live_h, &stage_h, &energy_h, &hand_h);
    float live_y, stage_y, energy_y, hand_y;
    if (section_opponent) {
        hand_y = section_y0;
        energy_y = hand_y + hand_h + 1;
        stage_y = energy_y + energy_h + 1;
        live_y = stage_y + stage_h + 1;
    } else {
        live_y = section_y0;
        stage_y = live_y + live_h + 1;
        energy_y = stage_y + stage_h + 1;
        hand_y = energy_y + energy_h + 1;
    }
    switch (zone_type) {
        case 0: return (int)live_y;
        case 1: return (int)stage_y;
        case 2: return (int)energy_y;
        case 3: return (int)hand_y;
        default: return 0;
    }
}

int _3ds_board_get_zone_h(int zone_type) {
    float live_h, stage_h, energy_h, hand_h;
    zone_heights(section_h, &live_h, &stage_h, &energy_h, &hand_h);
    switch (zone_type) {
        case 0: return (int)live_h;
        case 1: return (int)stage_h;
        case 2: return (int)energy_h;
        case 3: return (int)hand_h;
        default: return 0;
    }
}

// ---- Atlas cache ----
C2D_Image _3ds_get_card_image(const char* atlas_name, int index) {
    C2D_Image empty = {0};
    if (!atlas_name || atlas_name[0] == '\0') return empty;

    // Search cache
    for (int i = 0; i < atlas_count; i++) {
        if (strcmp(atlases[i].name, atlas_name) == 0) {
            C2D_Image img = C2D_SpriteSheetGetImage(atlases[i].sheet, (size_t)index);
            if (!img.subtex) return empty;
            return img;
        }
    }

    // Load new atlas
    char path[128];
    snprintf(path, sizeof(path), "romfs:/cards/%s", atlas_name);
    C2D_SpriteSheet sheet = C2D_SpriteSheetLoad(path);
    if (!sheet) {
        return empty;
    }

    if (atlas_count >= MAX_CACHED_ATLAS) {
        C2D_SpriteSheetFree(sheet);
        return empty;
    }

    strncpy(atlases[atlas_count].name, atlas_name, 63);
    atlases[atlas_count].name[63] = '\0';
    atlases[atlas_count].sheet = sheet;
    atlas_count++;

    C2D_Image img = C2D_SpriteSheetGetImage(sheet, (size_t)index);
    if (!img.subtex) return empty;
    return img;
}

// ---- Drawing helpers (call between C2D_SceneBegin / C3D_FrameEnd) ----
void _3ds_draw_rect(float x, float y, float w, float h, u32 color) {
    C2D_DrawRectSolid(x, y, 0.0f, w, h, color);
}

void _3ds_draw_border(float x, float y, float w, float h, u32 color, float thickness) {
    C2D_DrawRectSolid(x, y, 0.1f, w, thickness, color);
    C2D_DrawRectSolid(x, y + h - thickness, 0.1f, w, thickness, color);
    C2D_DrawRectSolid(x, y, 0.1f, thickness, h, color);
    C2D_DrawRectSolid(x + w - thickness, y, 0.1f, thickness, h, color);
}

void _3ds_draw_dotted_rect(float x, float y, float w, float h, u32 color) {
    // Draw a 4-dash dotted rectangle outline showing wait-state card boundary
    float seg = 4.0f, gap = 4.0f;
    float step = seg + gap;
    // Top edge
    for (float px = x; px < x + w - seg; px += step) {
        float s = (px + seg > x + w) ? (x + w - px) : seg;
        C2D_DrawRectSolid(px, y, 0.1f, s, 2.0f, color);
    }
    // Bottom edge
    for (float px = x; px < x + w - seg; px += step) {
        float s = (px + seg > x + w) ? (x + w - px) : seg;
        C2D_DrawRectSolid(px, y + h - 2.0f, 0.1f, s, 2.0f, color);
    }
    // Left edge
    for (float py = y; py < y + h - seg; py += step) {
        float s = (py + seg > y + h) ? (y + h - py) : seg;
        C2D_DrawRectSolid(x, py, 0.1f, 2.0f, s, color);
    }
    // Right edge
    for (float py = y; py < y + h - seg; py += step) {
        float s = (py + seg > y + h) ? (y + h - py) : seg;
        C2D_DrawRectSolid(x + w - 2.0f, py, 0.1f, 2.0f, s, color);
    }
}

// Draw a text label at (x,y) with the given color and scale.
// citro2d normalizes all BCFNT fonts so that scale 1.0 = 30px glyph height.
// The formula is: rendered_glyph_height = scale * 30.0 pixels.
//   scale 0.50 = 15px (too small for 3DS screens)
//   scale 0.65 = 20px (minimum readable)
//   scale 0.70 = 21px (good for body text)
//   scale 0.85 = 26px (CLI mode / large titles)
//   scale 1.0  = 30px (full system-font size)
void _3ds_draw_label(const char* label, float x, float y, u32 color, float scale) {
    if (!label || label[0] == '\0') return;
    C2D_Font f = custom_font ? custom_font : NULL;
    C2D_TextBufClear(tmp_text_buf);
    C2D_Text t;
    C2D_TextFontParse(&t, f, tmp_text_buf, label);
    C2D_TextOptimize(&t);
    C2D_DrawText(&t, C2D_WithColor, x, y, 0.6f, scale, scale, color);
}

// Draw label with inline {{icon.png|label}} tokens (mirrors render_text_with_icons for bottom screen)
void _3ds_draw_label_icons(const char* label, float x, float y, u32 color, float scale) {
    if (!label || label[0] == '\0') return;
    C2D_Font f = custom_font ? custom_font : NULL;
    float cx = x;
    float text_h = scale * 30.0f;
    float icon_h = (scale * 16.0f);
    if (icon_h < 11.0f) icon_h = 11.0f;
    float icon_y = y + (text_h - icon_h) / 2.0f;
    const char* p = label;
    while (*p) {
        // Find next {{
        const char* open = strstr(p, "{{");
        if (!open) {
            // Draw remaining as plain text
            C2D_TextBufClear(tmp_text_buf);
            C2D_Text t;
            C2D_TextFontParse(&t, f, tmp_text_buf, p);
            C2D_TextOptimize(&t);
            C2D_DrawText(&t, C2D_WithColor, cx, y, 0.6f, scale, scale, color);
            break;
        }
        if (open > p) {
            // Draw text segment before {{
            char seg[256];
            int len = (int)(open - p);
            if (len > 255) len = 255;
            memcpy(seg, p, len);
            seg[len] = '\0';
            C2D_TextBufClear(tmp_text_buf);
            C2D_Text t;
            C2D_TextFontParse(&t, f, tmp_text_buf, seg);
            C2D_TextOptimize(&t);
            C2D_DrawText(&t, C2D_WithColor, cx, y, 0.6f, scale, scale, color);
            // Measure width to advance cx
            float sw = 0;
            C2D_TextGetDimensions(&t, scale, scale, &sw, NULL);
            cx += sw;
        }
        // Find }}
        const char* close = strstr(open + 2, "}}");
        if (!close) break;
        // Parse inner: "file.png|label"
        char inner[256];
        int ilen = (int)(close - open - 2);
        if (ilen > 255) ilen = 255;
        memcpy(inner, open + 2, ilen);
        inner[ilen] = '\0';
        const char* bar = strchr(inner, '|');
        char file[256];
        if (bar) {
            int flen = (int)(bar - inner);
            memcpy(file, inner, flen);
            file[flen] = '\0';
        } else {
            strncpy(file, inner, 255);
            file[255] = '\0';
        }
        // Strip .png suffix for atlas lookup: "heart_06.png" -> "heart_06"
        char atlas_base[256];
        strncpy(atlas_base, file, 255);
        atlas_base[255] = '\0';
        char* dotpng = strstr(atlas_base, ".png");
        if (dotpng) *dotpng = '\0';
        char atlas_name[280];
        snprintf(atlas_name, sizeof(atlas_name), "icon_%s.png.t3x", atlas_base);
        C2D_Image img = _3ds_get_card_image(atlas_name, 0);
        if (img.tex) {
            float iw = icon_h * ((float)img.subtex->width / (float)img.subtex->height);
            C2D_DrawImageAt(img, cx, icon_y, 0.5f, NULL, iw / (float)img.subtex->width, icon_h / (float)img.subtex->height);
            cx += iw + scale * 6.0f;
        }
        p = close + 2;
    }
}

void _3ds_draw_card_at(CardSlot* slot, float x, float y, float w, float h) {
    if (!slot->active) return;

    C2D_Image img = _3ds_get_card_image(slot->atlas, slot->index);
    if (!img.tex || !img.subtex) {
        _3ds_draw_rect(x, y, w, h, COL_PRPL);
        return;
    }

    float iw = (float)img.subtex->width;
    float ih = (float)img.subtex->height;

    if (slot->tapped) {
        // Waited: 90° CCW rotation, image dimensions swap
        float scale = (w / ih) < (h / iw) ? (w / ih) : (h / iw);
        float cx = x + w * 0.5f;
        float cy = y + h * 0.5f;
        C2D_DrawImageAtRotated(img, cx, cy, 0.5f, 1.57079633f, NULL, scale, scale);
        return;
    }

    float scale = (w / iw) < (h / ih) ? (w / iw) : (h / ih);
    float dw = iw * scale;
    float dh = ih * scale;
    float cx = x + (w - dw) * 0.5f;
    float cy = y + (h - dh) * 0.5f;

    if (slot->flipped) {
        C2D_DrawImageAt(img, cx, cy, 0.5f, NULL, -scale, -scale);
    } else {
        C2D_DrawImageAt(img, cx, cy, 0.5f, NULL, scale, scale);
    }
}

// Zone height allocation for bottom screen (320x240).
// Energy uses PORTRAIT ratio cards so active/waited look the same.
// 12 energy cards fit at 15% with PORTRAIT ratio (e_w = e_sz * 0.711).
static void zone_heights(float h, float* live, float* stage, float* energy, float* hand) {
    float u = h - 3.0f;
    *live   = u * 0.15f;
    *stage  = u * 0.35f;
    *energy = u * 0.15f;
    *hand   = u * 0.35f;
}

static void draw_section(PlayerBoard* pb, float y0, float h, bool opponent, bool flip_cards) {
    const float W = 320.0f, M = 2.0f;
    const float PORTRAIT = 0.711f;
    const float LANDSCAPE = 1.41f;

    for (int i = 0; i < 3; i++) { pb->stage[i].flipped = flip_cards; pb->live[i].flipped = flip_cards; }
    for (int i = 0; i < MAX_SLOTS; i++) { pb->energy[i].flipped = flip_cards; pb->hand[i].flipped = flip_cards; }

    float live_h, stage_h, energy_h, hand_h;
    zone_heights(h, &live_h, &stage_h, &energy_h, &hand_h);

    float live_y, stage_y, energy_y, hand_y;
    if (opponent) {
        // Physical board: opponent zones reversed, drawn from bottom up
        hand_y = y0;
        energy_y = hand_y + hand_h + 1;
        stage_y = energy_y + energy_h + 1;
        live_y = stage_y + stage_h + 1;
    } else {
        live_y = y0;
        stage_y = live_y + live_h + 1;
        energy_y = stage_y + stage_h + 1;
        hand_y = energy_y + energy_h + 1;
    }

    float st_slot_h = stage_h - 4;
    float st_slot_w = st_slot_h;
    float util_x = M + 3 * (st_slot_w + 2) + 5;
    float util_w = W - util_x - M;

    // === LIVE ZONE ===
    _3ds_draw_rect(M, live_y, W - 2 * M, live_h, COL_ZONE_BG);
    _3ds_draw_border(M, live_y, W - 2 * M, live_h, COL_PINK, 1);
    float lx = M + 3;
    float live_card_h = live_h - 4;
    float live_slot_w = live_card_h * LANDSCAPE;
    for (int i = 0; i < 3; i++) {
        _3ds_draw_rect(lx, live_y + 1, live_slot_w, live_card_h, 0x33000000);
        if (pb->live[i].active) {
            _3ds_draw_card_at(&pb->live[i], lx, live_y + 1, live_slot_w, live_card_h);
        }
        lx += live_slot_w + 2;
    }
    // Draw own need hearts grid to the right of live cards (own player only)
    if (!opponent) {
        int pi = 0;
        int cols = (board_view == 2) ? 8 : 4;
        int rows = (board_view == 2) ? 1 : 2;
        float icon_sz = 10.0f;
        float gap = 1.0f;
        for (int i = 0; i < 8; i++) {
            if (need_hearts_counts[pi][i] == 0) continue;
            int col = i % cols;
            int row = i / cols;
            if (row >= rows) break;
            float ix = lx + 2 + col * (icon_sz + gap);
            float iy = live_y + 2 + row * (icon_sz + gap);
            char atlas_name[64];
            snprintf(atlas_name, sizeof(atlas_name), "icon_heart_%02d.png.t3x", i);
            C2D_Image img = _3ds_get_card_image(atlas_name, 0);
            if (img.tex) {
                C2D_DrawImageAt(img, ix, iy, 0.5f, NULL,
                    icon_sz / (float)img.subtex->width,
                    icon_sz / (float)img.subtex->height);
            }
        }
    }

// === STAGE + UTILITY ===
    float st_x = M;
    float st_card_w = st_slot_h * PORTRAIT;   // portrait card width within landscape slot
    float st_pad_x = (st_slot_w - st_card_w) * 0.5f;  // horizontal padding to center portrait
    float st_pad_y = 1.0f;
    // Stage slots: opponent displayed in reverse (R C L)
    for (int i = 0; i < 3; i++) {
        int si = opponent ? (2 - i) : i;
        float sy = stage_y + 1;
        _3ds_draw_rect(st_x, sy, st_slot_w, st_slot_h, COL_ZONE_BG);
        // Portrait card slot (solid border, centered)
        float psx = st_x + st_pad_x;
        _3ds_draw_border(psx, sy + st_pad_y, st_card_w, st_slot_h - 2, COL_BLUE, 1);
        if (!cli_mode && _is_highlighted(1, si, opponent)) {
            _3ds_draw_border(st_x, sy, st_slot_w, st_slot_h, COL_SEL, 2);
        }
        if (pb->stage[si].active) {
            _3ds_draw_card_at(&pb->stage[si], st_x + 1, sy + 2, st_slot_w - 2, st_slot_h - 4);
        }
        // Dotted rotated portrait rect showing wait-state preview (cross shape with solid rect)
        float ddw = st_slot_h - 2;
        float ddh = st_card_w;
        float ddx = st_x + (st_slot_w - ddw) * 0.5f;
        float ddy = sy + (st_slot_h - ddh) * 0.5f;
        _3ds_draw_dotted_rect(ddx, ddy, ddw, ddh, 0xFFFF8800);
        st_x += st_slot_w + 2;
    }

    // Utility counts — D E on top row, W S on bottom row
    _3ds_draw_rect(util_x, stage_y, util_w, stage_h, COL_ZONE_BG);
    _3ds_draw_border(util_x, stage_y, util_w, stage_h, COL_ZONE_BDR, 1);
    char buf[40];
    float fs = stage_h > 40 ? 0.70f : 0.65f;
    float fx = util_x + 1;
    float row_h = stage_h * 0.5f;
    snprintf(buf, sizeof(buf), "D:%d", pb->deck);
    _3ds_draw_label(buf, fx, stage_y + 1, COL_TEXT, fs);
    snprintf(buf, sizeof(buf), "E:%d", pb->edeck);
    _3ds_draw_label(buf, fx + util_w * 0.5f, stage_y + 1, COL_TEXT, fs);
    snprintf(buf, sizeof(buf), "W:%d", pb->discard);
    _3ds_draw_label(buf, fx, stage_y + 1 + row_h, COL_TEXT, fs);
    snprintf(buf, sizeof(buf), "S:%d", pb->success);
    _3ds_draw_label(buf, fx + util_w * 0.5f, stage_y + 1 + row_h, COL_TEXT, fs);

    // === ENERGY ===
    _3ds_draw_rect(M, energy_y, W - 2 * M, energy_h, COL_ZONE_BG);
    _3ds_draw_border(M, energy_y, W - 2 * M, energy_h, COL_GOLD, 1);
    float ex = M + 2;
    float e_sz = energy_h - 4;
    float e_w = e_sz * PORTRAIT;
    for (int i = 0; i < pb->energy_count && i < MAX_SLOTS; i++) {
        if (!cli_mode && _is_highlighted(2, i, opponent)) {
            _3ds_draw_border(ex, energy_y + 2, e_w, e_sz, COL_SEL, 2);
        }
        if (pb->energy[i].active) _3ds_draw_card_at(&pb->energy[i], ex, energy_y + 2, e_w, e_sz);
        else _3ds_draw_rect(ex, energy_y + 2, e_w, e_sz, 0x33000000);
        ex += e_w;
        if (ex > W - M) break;
    }

    // === HAND ===
    _3ds_draw_rect(M, hand_y, W - 2 * M, hand_h, COL_ZONE_BG);
    _3ds_draw_border(M, hand_y, W - 2 * M, hand_h, COL_TEXT, 1);
    float hx = M + 2;
    float hand_card_h = hand_h - 4;
    float h_slot_w = hand_card_h * PORTRAIT;
    for (int i = 0; i < pb->hand_count && i < MAX_SLOTS; i++) {
        if (pb->hand[i].active) _3ds_draw_card_at(&pb->hand[i], hx, hand_y + 2, h_slot_w, hand_card_h);
        if (!cli_mode && _is_highlighted(3, i, opponent)) {
            // Draw highlight border above card (z=0.6 > card z=0.5)
            C2D_DrawRectSolid(hx, hand_y + 2, 0.6f, h_slot_w, 2, COL_SEL);
            C2D_DrawRectSolid(hx, hand_y + hand_card_h, 0.6f, h_slot_w, 2, COL_SEL);
            C2D_DrawRectSolid(hx, hand_y + 2, 0.6f, 2, hand_card_h, COL_SEL);
            C2D_DrawRectSolid(hx + h_slot_w - 2, hand_y + 2, 0.6f, 2, hand_card_h, COL_SEL);
        }
        hx += h_slot_w + 1;
        if (hx > W - M) break;
    }
}

void _3ds_render_board() {
    C2D_SceneBegin(bot_target);
    C2D_TargetClear(bot_target, COL_BG);

    // Only draw active-player section border overlay in game mode
    if (!cli_mode && board_view == 2) {
        float half = 114.0f;
        float div_y = half + 2;
        _3ds_board_set_section_rect(2, half, true);
        draw_section(&o_board, 2, half, true, true);
        _3ds_draw_rect(0, div_y, 320, 4, COL_ZONE_BDR);
        _3ds_board_set_section_rect(div_y + 4, 240 - div_y - 4, false);
        draw_section(&p_board, div_y + 4, 240 - div_y - 4, false, false);
        // Active player: no border highlight
    } else if (board_view == 1) {
        _3ds_board_set_section_rect(0, 240, false);
        draw_section(&o_board, 0, 240, false, true);
    } else {
        _3ds_board_set_section_rect(0, 240, false);
        draw_section(&p_board, 0, 240, false, false);
    }

    // Action overlay panel (game mode, bottom-right)
    // Action overlay panel: scale 0.60 = 18px glyph, line height 24px.
    if (!cli_mode && overlay_count > 0) {
        float p_w = 210.0f, p_h = 24.0f * overlay_count + 8.0f;
        float p_x = 320.0f - p_w - 2.0f;
        float p_y = 240.0f - p_h - 2.0f;
        C2D_DrawRectSolid(p_x, p_y, 0.5f, p_w, p_h, C2D_Color32(10, 14, 26, 220));
        _3ds_draw_border(p_x, p_y, p_w, p_h, COL_ZONE_BDR, 1);
        for (int i = 0; i < overlay_count; i++) {
            float ly = p_y + 4.0f + i * 24.0f;
            if (i == overlay_selected) {
                C2D_DrawRectSolid(p_x + 1, ly - 1, 0.5f, p_w - 2, 22.0f, C2D_Color32(80, 100, 80, 100));
            }
            char line[OVERLAY_LINE_LEN + 2];
            snprintf(line, sizeof(line), "%s%s", i == overlay_selected ? ">" : " ", overlay_lines[i]);
            _3ds_draw_label(line, p_x + 3, ly, i == overlay_selected ? COL_SEL : COL_TEXT, 0.60f);
        }
    }

    // No view indicator or hand range labels — clean board
}

// ---- Text measurement ----
float _3ds_measure_text_width(const char* text, float scale) {
    if (!text || !text[0]) return 0.0f;
    C2D_Font f = custom_font ? custom_font : NULL;
    C2D_Text tmp;
    C2D_TextFontParse(&tmp, f, tmp_text_buf, text);
    C2D_TextOptimize(&tmp);
    float font_scale = 1.2f;
    float w, h;
    C2D_TextGetDimensions(&tmp, scale * font_scale, scale * font_scale, &w, &h);
    C2D_TextBufClear(tmp_text_buf);
    return w;
}

// Measure height of text after word-wrapping + icon parsing.
// Returns total pixel height for the text rendered at (x, scale) with max_w constraint.
float _3ds_text_wrapped_height(const char* text, float scale, float max_w) {
    if (!text || !text[0]) return 0.0f;
    C2D_Font f = custom_font ? custom_font : NULL;
    float font_scale = 1.2f;
    float s = scale * font_scale;
    float line_h = s * 30.0f;
    float cx = 0.0f;
    int lines = 1;
    const char* p = text;
    while (*p) {
        const char* open = strstr(p, "{{");
        if (!open) {
            // Measure remaining text and count wrapped lines
            C2D_TextBufClear(tmp_text_buf);
            C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, p);
            C2D_TextOptimize(&tmp_text_obj);
            float tw = 0, th = 0;
            C2D_TextGetDimensions(&tmp_text_obj, s, s, &tw, &th);
            if (cx + tw > max_w && cx > 0) lines++;
            break;
        }
        if (open > p) {
            char seg[256];
            int len = (int)(open - p);
            if (len > 255) len = 255;
            memcpy(seg, p, len);
            seg[len] = '\0';
            C2D_TextBufClear(tmp_text_buf);
            C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, seg);
            C2D_TextOptimize(&tmp_text_obj);
            float tw = 0, th = 0;
            C2D_TextGetDimensions(&tmp_text_obj, s, s, &tw, &th);
            if (cx + tw > max_w && cx > 0) { lines++; cx = 0; }
            cx += tw;
        }
        const char* close = strstr(open + 2, "}}");
        if (!close) break;
        // Icon width
        char inner[256];
        int ilen = (int)(close - open - 2);
        if (ilen > 255) ilen = 255;
        memcpy(inner, open + 2, ilen);
        inner[ilen] = '\0';
        const char* bar = strchr(inner, '|');
        char file[256];
        if (bar) { int flen = (int)(bar - inner); memcpy(file, inner, flen); file[flen] = '\0'; }
        else { strncpy(file, inner, 255); file[255] = '\0'; }
        float icon_h = (s * 16.0f);
        if (icon_h < 11.0f) icon_h = 11.0f;
        float iw = icon_h + s * 6.0f;
        if (cx + iw > max_w && cx > 0) { lines++; cx = 0; }
        cx += iw;
        p = close + 2;
    }
    return lines * line_h;
}

// ---- Icon dimension query ----
// Returns width/height ratio for an icon atlas, or 1.0 if not found.
float _3ds_icon_aspect(const char* atlas_name) {
    if (!atlas_name || !atlas_name[0]) return 1.0f;
    C2D_Image img = _3ds_get_card_image(atlas_name, 0);
    if (img.tex && img.subtex && img.subtex->height > 0) {
        return (float)img.subtex->width / (float)img.subtex->height;
    }
    return 1.0f;
}

// ---- Main render ----
void _3ds_swap_buffers() {
    // Re-parse top text
    if (text_dirty) {
        C2D_Font f = custom_font ? custom_font : NULL;
        if (top_text[0]) {
            C2D_TextBufClear(top_buf);
            C2D_TextFontParse(&top_obj, f, top_buf, top_text);
            C2D_TextOptimize(&top_obj);
            top_parsed = true;
        } else {
            top_parsed = false;
        }
        text_dirty = false;
    }

    C3D_FrameBegin(C3D_FRAME_SYNCDRAW);

    // TOP SCREEN — stereoscopic 3D: render twice (left + right eye)
    int eye_count = gfxIs3D() ? 2 : 1;
    float slider = osGet3DSliderState();
    for (int eye = 0; eye < eye_count; eye++) {
        C3D_RenderTarget* target = (eye == 0) ? top_target : top_target_right;
        float x_off = (eye == 1) ? -slider * 48.0f : 0.0f;

        if (cli_mode) {
            C2D_TargetClear(target, C2D_Color32(0, 0, 0, 255));
            C2D_SceneBegin(target);
            if (top_parsed) {
                float cli_scale = custom_font ? 0.85f : 0.66f;
                C2D_DrawText(&top_obj,
                    C2D_WithColor,
                    2.0f + x_off, 2.0f - (float)top_scroll_y, 0.5f,
                    cli_scale, cli_scale,
                    C2D_Color32(0, 255, 0, 255),
                    390.0f);
            }
        } else {
            C2D_TargetClear(target, COL_TOP_BG);
            C2D_SceneBegin(target);
            // Camera preview for QR scan (draws behind text overlays)
            _3ds_qr_draw_preview(x_off);
            C2D_TextBufClear(tmp_text_buf);
            C2D_Font f = custom_font ? custom_font : NULL;
            float font_scale = 1.2f;
            for (int i = 0; i < draw_op_count; i++) {
                if (draw_op_types[i] == OP_RECT) {
                    C2D_DrawRectSolid(draw_ops[i].x + x_off, draw_ops[i].y, 0.5f,
                        draw_ops[i].w, draw_ops[i].h, draw_ops[i].color);
                } else if (draw_op_types[i] == OP_TEXT) {
                    const char* full = draw_ops[i].text;
                    float x = draw_ops[i].x + x_off;
                    float y = draw_ops[i].y;
                    float s = draw_ops[i].scale * font_scale;
                    float text_h = s * 30.0f;
                    float icon_h = (s * 16.0f);
                    if (icon_h < 11.0f) icon_h = 11.0f;
                    float icon_y = y + (text_h - icon_h) / 2.0f;
                    const char* p = full;
                    while (*p) {
                        const char* open = strstr(p, "{{");
                        if (!open) {
                            C2D_TextBufClear(tmp_text_buf);
                            C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, p);
                            C2D_TextOptimize(&tmp_text_obj);
                            float max_w = fmaxf(390.0f - x, 0.0f);
                            C2D_DrawText(&tmp_text_obj, C2D_WithColor | C2D_WordWrap,
                                x, y, 0.5f, s, s, draw_ops[i].color, max_w);
                            break;
                        }
                        if (open > p) {
                            char seg[256];
                            int len = (int)(open - p);
                            if (len > 255) len = 255;
                            memcpy(seg, p, len);
                            seg[len] = '\0';
                            C2D_TextBufClear(tmp_text_buf);
                            C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, seg);
                            C2D_TextOptimize(&tmp_text_obj);
                            float max_w = fmaxf(390.0f - x, 0.0f);
                            C2D_DrawText(&tmp_text_obj, C2D_WithColor | C2D_WordWrap,
                                x, y, 0.5f, s, s, draw_ops[i].color, max_w);
                            float sw = 0;
                            C2D_TextGetDimensions(&tmp_text_obj, s, s, &sw, NULL);
                            x += sw;
                        }
                        const char* close = strstr(open + 2, "}}");
                        if (!close) break;
                        char inner[256];
                        int ilen = (int)(close - open - 2);
                        if (ilen > 255) ilen = 255;
                        memcpy(inner, open + 2, ilen);
                        inner[ilen] = '\0';
                        const char* bar = strchr(inner, '|');
                        char file[256];
                        if (bar) { int flen = (int)(bar - inner); memcpy(file, inner, flen); file[flen] = '\0'; }
                        else { strncpy(file, inner, 255); file[255] = '\0'; }
                        char atlas_base[256];
                        strncpy(atlas_base, file, 255);
                        atlas_base[255] = '\0';
                        char* dotpng = strstr(atlas_base, ".png");
                        if (dotpng) *dotpng = '\0';
                        char atlas_name[280];
                        snprintf(atlas_name, sizeof(atlas_name), "icon_%s.png.t3x", atlas_base);
                        C2D_Image img = _3ds_get_card_image(atlas_name, 0);
                        if (img.tex) {
                            float iw = icon_h * ((float)img.subtex->width / (float)img.subtex->height);
                            C2D_DrawImageAt(img, x, icon_y, 0.5f, NULL,
                                iw / (float)img.subtex->width, icon_h / (float)img.subtex->height);
                            x += iw + s * 6.0f;
                        }
                        p = close + 2;
                    }
                } else if (draw_op_types[i] == OP_CARD) {
                    C2D_Image img = _3ds_get_card_image(draw_ops[i].atlas, draw_ops[i].atlas_idx);
                    if (img.tex != NULL) {
                        float sx = draw_ops[i].w / (float)img.subtex->width;
                        float sy = draw_ops[i].h / (float)img.subtex->height;
                        C2D_DrawImageAt(img, draw_ops[i].x + x_off, draw_ops[i].y, 0.5f, NULL, sx, sy);
                    }
                }
            }
        }
    }

    // BOTTOM SCREEN: board or text (no 3D effect)
    if (board_mode) {
        _3ds_render_board();
    } else {
        C2D_SceneBegin(bot_target);
        C2D_TargetClear(bot_target, C2D_Color32(0, 0, 0, 255));
        // Setup menus render their draw-op queue onto the bottom screen.
        C2D_TextBufClear(tmp_text_buf);
        C2D_Font f = custom_font ? custom_font : NULL;
        float font_scale = 1.2f;
        for (int i = 0; i < bot_draw_op_count; i++) {
            if (bot_draw_op_types[i] == OP_RECT) {
                C2D_DrawRectSolid(bot_draw_ops[i].x, bot_draw_ops[i].y, 0.5f,
                    bot_draw_ops[i].w, bot_draw_ops[i].h, bot_draw_ops[i].color);
            } else if (bot_draw_op_types[i] == OP_TEXT) {
                C2D_TextBufClear(tmp_text_buf);
                C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, bot_draw_ops[i].text);
                C2D_TextOptimize(&tmp_text_obj);
                float s = bot_draw_ops[i].scale * font_scale;
                C2D_DrawText(&tmp_text_obj, C2D_WithColor | C2D_WordWrap,
                    bot_draw_ops[i].x, bot_draw_ops[i].y, 0.5f, s, s,
                    bot_draw_ops[i].color, fmaxf(320.0f - bot_draw_ops[i].x, 0.0f));
            }
        }
    }

    C3D_FrameEnd(0);

    // After GPU idle — no deferred free needed, qr_destroy_context handles cleanup
}

// ---- Input + system ----
int _3ds_main_loop() {
    return aptMainLoop();
}

void _3ds_debug_print(const char *msg) {
    svcOutputDebugString(msg, strlen(msg));
}

void _3ds_tdbg(const char *msg) {
    svcOutputDebugString(msg, strlen(msg));
    svcOutputDebugString("\n", 1);
}

void _3ds_scan_input() {
    hidScanInput();
}

u32 _3ds_keys_down() {
    return hidKeysDown();
}

u32 _3ds_keys_held() {
    return hidKeysHeld();
}

void _3ds_touch_read(u32* px, u32* py) {
    touchPosition t;
    hidTouchRead(&t);
    *px = t.px;
    *py = t.py;
}

bool _3ds_touch_down() {
    touchPosition t;
    hidTouchRead(&t);
    return t.px != 0 || t.py != 0;
}

u64 _3ds_system_tick() {
    return (u64)svcGetSystemTick();
}

static uint64_t state = 1;
ssize_t getrandom(void *buf, size_t buflen, unsigned int flags) {
    (void)flags;
    if (state == 1) {
        state = svcGetSystemTick();
        if (state == 0) state = 1;
    }
    uint8_t *b = (uint8_t *)buf;
    for (size_t i = 0; i < buflen; i++) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        b[i] = (uint8_t)state;
    }
    return buflen;
}

// ---- UDS local wireless multiplayer ----

#define UDS_WLAN_COMM_ID  0xFF150848
#define UDS_DATA_CHANNEL  1
#define UDS_MAX_NODES     2

static u32 uds_sharedmem_size = 0x8000; // 32KB — more packet buffering = fewer burst drops
static u32 uds_recv_buf_size = UDS_DEFAULT_RECVBUFSIZE;
static u8 uds_data_channel = UDS_DATA_CHANNEL;
static udsNetworkStruct uds_netstruct;
static udsBindContext uds_bindctx;
static bool uds_initialized = false;
static bool uds_is_host = false;
static bool uds_connected = false;

// App data for network identification (first 4 bytes = magic, rest = random)
static u8 uds_appdata[0x14] = {0x52, 0x42, 0x4B, 0x00}; // "RBK" + padding

int _3ds_uds_init(int is_host) {
    if (uds_initialized) return -1;

    Result ret = udsInit(uds_sharedmem_size, NULL);
    if (R_FAILED(ret)) return -2;

    uds_is_host = (is_host != 0);

    if (uds_is_host) {
        // Generate a random passphrase from the appdata
        char passphrase[0x14];
        memset(passphrase, 0, sizeof(passphrase));
        strncpy(passphrase, (char*)uds_appdata + 4, 12);

        udsGenerateDefaultNetworkStruct(&uds_netstruct, UDS_WLAN_COMM_ID, 0, UDS_MAX_NODES);

        ret = udsCreateNetwork(&uds_netstruct, passphrase, strlen(passphrase) + 1,
                               &uds_bindctx, uds_data_channel, uds_recv_buf_size);
        if (R_FAILED(ret)) {
            udsExit();
            return -3;
        }

        ret = udsSetApplicationData(uds_appdata, sizeof(uds_appdata));
        if (R_FAILED(ret)) {
            udsDestroyNetwork();
            udsUnbind(&uds_bindctx);
            udsExit();
            return -4;
        }

        // No spectators
        udsEjectSpectator();

        uds_initialized = true;
        uds_connected = true; // host is always "connected" once network is created
        return 0;
    } else {
        // Client: scanning and connecting done separately
        uds_initialized = true;
        uds_connected = false;
        return 0;
    }
}

// Scan for available host networks. Returns count of matching networks found.
// Each network's node_id is written to out_ids (up to max_out).
int _3ds_uds_scan_networks(unsigned short *out_ids, int max_out) {
    if (!uds_initialized || uds_is_host) return 0;

    u32 tmpbuf_size = 0x4000;
    u32 *tmpbuf = (u32*)malloc(tmpbuf_size);
    if (!tmpbuf) return 0;
    memset(tmpbuf, 0, tmpbuf_size);

    size_t total_networks = 0;
    udsNetworkScanInfo *networks = NULL;

    for (int i = 0; i < 5; i++) {
        Result ret = udsScanBeacons(tmpbuf, tmpbuf_size, &networks, &total_networks,
                                     UDS_WLAN_COMM_ID, 0, NULL, false);
        if (total_networks > 0) break;
        svcSleepThread(500000000); // 500ms between scans
    }

    int count = 0;
    for (size_t i = 0; i < total_networks && count < max_out; i++) {
        udsNetworkStruct *net = &networks[i].network;
        if (memcmp(net->appdata, uds_appdata, 4) == 0) {
            out_ids[count] = net->networkID;
            count++;
        }
    }

    free(tmpbuf);
    free(networks);
    return count;
}

// Connect to a specific network by node_id.
int _3ds_uds_connect_network(unsigned short node_id) {
    if (!uds_initialized || uds_is_host) return -1;

    u32 tmpbuf_size = 0x4000;
    u32 *tmpbuf = (u32*)malloc(tmpbuf_size);
    if (!tmpbuf) return -2;
    memset(tmpbuf, 0, tmpbuf_size);

    size_t total_networks = 0;
    udsNetworkScanInfo *networks = NULL;

    Result ret = udsScanBeacons(tmpbuf, tmpbuf_size, &networks, &total_networks,
                                 UDS_WLAN_COMM_ID, 0, NULL, false);
    free(tmpbuf);

    if (total_networks == 0 || !networks) { free(networks); return -3; }

    char passphrase[0x14];
    memset(passphrase, 0, sizeof(passphrase));
    strncpy(passphrase, (char*)uds_appdata + 4, 12);

    int result = -4;
    for (size_t i = 0; i < total_networks; i++) {
        udsNetworkStruct *net = &networks[i].network;
        if (net->networkID == node_id && memcmp(net->appdata, uds_appdata, 4) == 0) {
            ret = udsConnectNetwork(net, passphrase, strlen(passphrase) + 1,
                                    &uds_bindctx, UDS_BROADCAST_NETWORKNODEID,
                                    UDSCONTYPE_Client, uds_data_channel,
                                    uds_recv_buf_size);
            if (R_SUCCEEDED(ret)) { result = 0; uds_connected = true; break; }
        }
    }

    free(networks);
    return result;
}

void _3ds_uds_exit() {
    if (!uds_initialized) return;
    if (uds_is_host) {
        udsDestroyNetwork();
    } else {
        udsDisconnectNetwork();
    }
    udsUnbind(&uds_bindctx);
    udsExit();
    uds_initialized = false;
    uds_connected = false;
}

int _3ds_uds_send(const unsigned char *data, unsigned int len) {
    if (!uds_connected) return -1;
    Result ret = udsSendTo(UDS_BROADCAST_NETWORKNODEID, uds_data_channel,
                           UDS_SENDFLAG_Default, (void*)data, len);
    if (R_FAILED(ret)) return -2;
    return (int)len;
}

int _3ds_uds_recv(unsigned char *buf, unsigned int buf_len, unsigned int *out_len) {
    if (!uds_connected) return -1;
    u16 src_node = 0;
    Result ret = udsPullPacket(&uds_bindctx, buf, buf_len, (size_t*)out_len, &src_node);
    if (R_FAILED(ret)) {
        *out_len = 0;
        return -2;
    }
    return 0;
}

int _3ds_uds_is_connected() {
    return uds_connected ? 1 : 0;
}

// ── QR Code Scanning (FBI's capturecam + quirc, adapted for rabuka FFI) ──
#include "quirc.h"
#include <3ds/services/gspgpu.h>

#define QR_W 400
#define QR_H 240

typedef struct rabuka_qr_data_s {
    struct quirc* qr;
    C3D_Tex tex;
    bool tex_inited;
    bool capturing;
    capture_cam_data cam;
} rabuka_qr_data;

static const Tex3DS_SubTexture qr_subtex = { 400, 240, 0.0f, 1.0f, 400.0f/512.0f, 1.0f - (240.0f/256.0f) };

void _3ds_qr_draw_preview(float x_off) {
    if (!g_rqr || g_rqr->cam.finished || !g_rqr->cam.buffer) return;

    svcWaitSynchronization(g_rqr->cam.mutex, U64_MAX);

    // Upload camera buffer to texture (untiled, same as FBI's screen_load_texture_untiled)
    if (g_rqr->tex.data) {
        const u16 *src = g_rqr->cam.buffer;
        u16 *dst = (u16*)g_rqr->tex.data;
        for (int y = 0; y < QR_H; y++) {
            for (int x = 0; x < QR_W; x++) {
                int si = y * QR_W + x;
                int di = ((((y >> 3) * (512 >> 3) + (x >> 3)) << 6)
                    + ((x & 1) | ((y & 1) << 1) | ((x & 2) << 1) | ((y & 2) << 2)
                    | ((x & 4) << 2) | ((y & 4) << 3)));
                dst[di] = src[si];
            }
        }
        GSPGPU_FlushDataCache(g_rqr->tex.data, 512 * 256 * 2);
        C3D_TexFlush(&g_rqr->tex);
    }

    svcReleaseMutex(g_rqr->cam.mutex);

    C2D_Image img = { .tex = &g_rqr->tex, .subtex = &qr_subtex };
    C2D_DrawImageAt(img, x_off, 0.0f, 0.4f, NULL, 1.0f, 1.0f);
}

void *_3ds_qr_start(void) {
    // Clean up any previous session
    if (g_rqr) {
        rabuka_qr_data *old = g_rqr;
        g_rqr = NULL;
        if (!old->cam.finished) {
            svcSignalEvent(old->cam.cancelEvent);
            while (!old->cam.finished) svcSleepThread(1000000);
        }
        if (old->cam.buffer) { free(old->cam.buffer); old->cam.buffer = NULL; }
        if (old->tex_inited) { C3D_TexDelete(&old->tex); old->tex_inited = false; }
        if (old->qr) { quirc_destroy(old->qr); old->qr = NULL; }
        free(old);
    }

    rabuka_qr_data *data = (rabuka_qr_data*)calloc(1, sizeof(rabuka_qr_data));
    if (!data) return NULL;

    data->qr = quirc_new();
    if (!data->qr) { free(data); return NULL; }
    if (quirc_resize(data->qr, QR_W, QR_H) != 0) { quirc_destroy(data->qr); free(data); return NULL; }

    data->cam.width = QR_W;
    data->cam.height = QR_H;
    data->cam.camera = CAMERA_OUTER;
    data->cam.buffer = (u16*)calloc(1, QR_W * QR_H * sizeof(u16));
    if (!data->cam.buffer) { quirc_destroy(data->qr); free(data); return NULL; }

    C3D_TexInit(&data->tex, 512, 256, GPU_RGB565);
    C3D_TexSetFilter(&data->tex, GPU_LINEAR, GPU_LINEAR);
    data->tex_inited = true;

    // Start capture immediately (FBI does this in scan_qr_code → info_display → first update)
    Result capRes = task_capture_cam(&data->cam);
    if (R_FAILED(capRes)) {
        free(data->cam.buffer);
        C3D_TexDelete(&data->tex);
        quirc_destroy(data->qr);
        free(data);
        return NULL;
    }
    data->capturing = true;

    g_rqr = data;
    return data;
}

void _3ds_qr_stop(void *p) {
    rabuka_qr_data *data = (rabuka_qr_data*)p;
    if (!data) return;
    if (!data->cam.finished) {
        svcSignalEvent(data->cam.cancelEvent);
        while (!data->cam.finished) svcSleepThread(1000000);
    }
}

void _3ds_qr_free(void *p) {
    rabuka_qr_data *data = (rabuka_qr_data*)p;
    if (!data) return;
    if (g_rqr == data) g_rqr = NULL;

    if (!data->cam.finished) {
        svcSignalEvent(data->cam.cancelEvent);
        while (!data->cam.finished) svcSleepThread(1000000);
    }
    if (data->cam.buffer) { free(data->cam.buffer); data->cam.buffer = NULL; }
    if (data->tex_inited) { C3D_TexDelete(&data->tex); data->tex_inited = false; }
    if (data->qr) { quirc_destroy(data->qr); data->qr = NULL; }
    free(data);
}

int _3ds_qr_poll(void *p, char *out_text, unsigned int out_max) {
    rabuka_qr_data *data = (rabuka_qr_data*)p;
    if (!data || out_max == 0) return -1;
    if (!data->qr) return -1;

    // Check if camera thread died (FBI checks this AFTER capturing check)
    if (data->capturing && data->cam.finished) return -1;

    // FBI: quirc_begin → mutex-protected memcpy → quirc_end → decode
    int w = 0, h = 0;
    uint8_t *qrBuf = quirc_begin(data->qr, &w, &h);
    if (!qrBuf) return 0;

    svcWaitSynchronization(data->cam.mutex, U64_MAX);

    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            u16 px = data->cam.buffer[y * QR_W + x];
            qrBuf[y * w + x] = (u8)(((((px >> 11) & 0x1F) << 3) + (((px >> 5) & 0x3F) << 2) + ((px & 0x1F) << 3)) / 3);
        }
    }

    svcReleaseMutex(data->cam.mutex);

    quirc_end(data->qr);

    int qrCount = quirc_count(data->qr);
    for (int i = 0; i < qrCount; i++) {
        struct quirc_code code;
        struct quirc_data qdata;
        quirc_extract(data->qr, i, &code);
        if (quirc_decode(&code, &qdata) == QUIRC_SUCCESS) {
            int len = qdata.payload_len;
            if (len > (int)out_max - 1) len = (int)out_max - 1;
            if (len < 0) len = 0;
            memcpy(out_text, qdata.payload, len);
            out_text[len] = '\0';
            return len;
        }
    }
    return 0;
}


// ===================== AUDIO (NDSP streaming OGG — requires /3ds/dspfirm.cdc on SD) =====================
#include <3ds/ndsp/ndsp.h>

// Triple-buffered streaming decoder (pattern: Anemone3DS music.c). A worker
// thread decodes the OGG with Tremor into a few small wave buffers and hands
// each finished buffer back to NDSP, instead of decoding the whole song into
// RAM (the old code capped this at 10s and truncated long tracks).
#define AUDIO_NUM_BUFS    3
#define AUDIO_BUF_SAMPLES 4096   // ~93ms @44.1kHz per buffer

typedef struct {
    OggVorbis_File vf;
    int  opened;                    // vf is valid & owned
    int  rate;
    int  channels;
    s16* bufs[AUDIO_NUM_BUFS];
    ndspWaveBuf wb[AUDIO_NUM_BUFS];
    volatile bool stop;
    Thread thread;
} audio_ogg_t;

static audio_ogg_t s_audio = {0};

static void _3ds_audio_stop(void);

// Decode one buffer's worth of interleaved PCM16 into `data` and set wb->nsamples.
// Seamlessly loops by rewinding on EOF. Returns 1 on success, 0 on hard error.
static int audio_fill(audio_ogg_t* a, ndspWaveBuf* wb, s16* data) {
    int cap_samples = AUDIO_BUF_SAMPLES * a->channels; // interleaved PCM16 samples
    int total = 0;
    while (total < cap_samples) {
        long ret = ov_read(&a->vf, (char*)(data + total),
                           (cap_samples - total) * (int)sizeof(s16), NULL);
        if (ret == 0) {
            // EOF: rewind to the start so the track loops forever.
            if (ov_pcm_seek(&a->vf, 0) < 0) break;
            if (total == 0) break; // empty stream
            continue;
        }
        if (ret < 0) break; // decode error
        total += (int)(ret / sizeof(s16));
    }
    if (total <= 0) return 0;
    wb->nsamples = (u32)(total / a->channels);
    DSP_FlushDataCache(data, total * (int)sizeof(s16));
    return 1;
}

static void audio_thread(void* arg) {
    (void)arg;
    // Prefill every buffer so playback begins immediately.
    for (int i = 0; i < AUDIO_NUM_BUFS && !s_audio.stop; i++) {
        if (audio_fill(&s_audio, &s_audio.wb[i], s_audio.bufs[i]))
            ndspChnWaveBufAdd(0, &s_audio.wb[i]);
        else
            break;
    }
    // Keep feeding NDSP: whenever a buffer finishes playing, refill and re-add it.
    while (!s_audio.stop) {
        for (int i = 0; i < AUDIO_NUM_BUFS && !s_audio.stop; i++) {
            ndspWaveBuf* wb = &s_audio.wb[i];
            if (wb->nsamples > 0 && wb->status == NDSP_WBUF_DONE) {
                if (audio_fill(&s_audio, wb, s_audio.bufs[i]))
                    ndspChnWaveBufAdd(0, wb);
            }
        }
        svcSleepThread(3000000ULL); // 3ms poll
    }
}

int _3ds_audio_init(void) {
    Result res = ndspInit();
    if (R_FAILED(res)) {
        // Make the failure visible on-screen instead of only in the debugger.
        char buf[96];
        snprintf(buf, sizeof(buf), "[AUDIO] ndspInit FAILED (need /3ds/dspfirm.cdc)\n");
        _3ds_text_add_top(buf);
        return (int)res;
    }
    ndspSetOutputMode(NDSP_OUTPUT_STEREO);
    ndspSetMasterVol(1.0f);
    return 0;
}

int _3ds_audio_play_ogg(const char* path) {
    _3ds_audio_stop();

    FILE* f = fopen(path, "rb");
    if (!f) {
        char buf[128];
        snprintf(buf, sizeof(buf), "[AUDIO] fopen failed: %s\n", path);
        _3ds_text_add_top(buf);
        return -1;
    }
    ov_callbacks cb = { 0 };
    cb.read_func  = (size_t(*)(void*, size_t, size_t, void*))fread;
    cb.seek_func  = (int(*)(void*, int64_t, int))fseek;
    cb.close_func = (int(*)(void*))fclose;
    cb.tell_func  = (long(*)(void*))ftell;
    if (ov_open_callbacks(f, &s_audio.vf, NULL, 0, cb) < 0) {
        fclose(f);
        _3ds_text_add_top("[AUDIO] ov_open FAILED (not a valid Ogg Vorbis)\n");
        return -2;
    }
    s_audio.opened = 1;

    vorbis_info* vi = ov_info(&s_audio.vf, -1);
    if (!vi || vi->channels < 1 || vi->channels > 2) {
        _3ds_audio_stop();
        return -3;
    }
    s_audio.rate     = vi->rate;
    s_audio.channels = vi->channels;

    for (int i = 0; i < AUDIO_NUM_BUFS; i++) {
        s_audio.bufs[i] = (s16*)linearAlloc(AUDIO_BUF_SAMPLES * s_audio.channels * sizeof(s16));
        if (!s_audio.bufs[i]) {
            _3ds_audio_stop();
            return -4;
        }
        memset(&s_audio.wb[i], 0, sizeof(ndspWaveBuf));
        s_audio.wb[i].data_vaddr = s_audio.bufs[i];
    }

    ndspChnReset(0);
    ndspChnSetInterp(0, NDSP_INTERP_POLYPHASE);
    ndspChnSetRate(0, (float)s_audio.rate);
    ndspChnSetFormat(0, s_audio.channels == 2 ? NDSP_FORMAT_STEREO_PCM16 : NDSP_FORMAT_MONO_PCM16);
    float mix[12] = {0};
    if (s_audio.channels == 2) { mix[0] = 1.0f; mix[1] = 1.0f; }
    else                       { mix[0] = 0.5f; mix[1] = 0.5f; } // mono -> both speakers
    ndspChnSetMix(0, mix);

    s_audio.stop = false;
    s_audio.thread = threadCreate(audio_thread, NULL, 0x4000, 0x18, -1, true);
    if (!s_audio.thread) {
        _3ds_audio_stop();
        return -5;
    }
    return 0;
}

void _3ds_audio_stop(void) {
    s_audio.stop = true;
    if (s_audio.thread) {
        threadJoin(s_audio.thread, 0xFFFFFFFF);
        threadFree(s_audio.thread);
        s_audio.thread = NULL;
    }
    s_audio.stop = false;
    if (s_audio.opened) {
        ov_clear(&s_audio.vf);
        s_audio.opened = 0;
    }
    ndspChnWaveBufClear(0);
    ndspChnReset(0);
    for (int i = 0; i < AUDIO_NUM_BUFS; i++) {
        if (s_audio.bufs[i]) {
            linearFree(s_audio.bufs[i]);
            s_audio.bufs[i] = NULL;
        }
    }
}

void _3ds_audio_set_volume(float vol) {
    float mix[12] = {0};
    mix[0] = vol;
    mix[1] = vol;
    ndspChnSetMix(0, mix);
}
