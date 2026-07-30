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
#include <math.h>
#include <3ds.h>
#include <citro2d.h>
#include <errno.h>
#include <tremor/ivorbisfile.h>

u32 __ctru_heap_size = 64 * 1024 * 1024;
u32 __stacksize__ = 2 * 1024 * 1024;

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



// ---- Action highlight on board slots (multiple) ----
#define MAX_HIGHLIGHTS 16
static int hl_count = 0;
static int hl_zones[MAX_HIGHLIGHTS];
static int hl_slots[MAX_HIGHLIGHTS];
static bool hl_opponent[MAX_HIGHLIGHTS];

// ---- Need hearts text per player (drawn next to live zone) ----
static char need_hearts_text[2][256] = {{0}, {0}};
void _3ds_set_need_hearts_text(int player, const char* text) {
    if (player < 0 || player > 1) return;
    if (!text) { need_hearts_text[player][0] = '\0'; return; }
    strncpy(need_hearts_text[player], text, 255);
    need_hearts_text[player][255] = '\0';
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
    for (int i = 0; i < atlas_count; i++) {
        C2D_SpriteSheetFree(atlases[i].sheet);
    }
    C2D_TextBufDelete(top_buf);
    if (tmp_text_buf) C2D_TextBufDelete(tmp_text_buf);
    if (custom_font) C2D_FontFree(custom_font);
    C2D_Fini();
    C3D_Fini();
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
        if (pb->live[i].active) _3ds_draw_card_at(&pb->live[i], lx, live_y + 1, live_slot_w, live_card_h);
        lx += live_slot_w + 2;
    }
    // Draw need hearts text to the right of live cards
    {
        int pi = opponent ? 1 : 0;
        if (need_hearts_text[pi][0] && lx < W - M - 4) {
            float fs = 0.40f;
            _3ds_draw_label_icons(need_hearts_text[pi], lx + 2, live_y + 2, COL_GOLD, fs);
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
    float w, h;
    C2D_TextGetDimensions(&tmp, scale, scale, &w, &h);
    C2D_TextBufClear(tmp_text_buf);
    return w;
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
                C2D_DrawText(&top_obj,
                    C2D_WithColor,
                    2.0f + x_off, 2.0f - (float)top_scroll_y, 0.5f,
                    0.85f, 0.85f,
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
            for (int i = 0; i < draw_op_count; i++) {
                if (draw_op_types[i] == OP_RECT) {
                    C2D_DrawRectSolid(draw_ops[i].x + x_off, draw_ops[i].y, 0.5f,
                        draw_ops[i].w, draw_ops[i].h, draw_ops[i].color);
                } else if (draw_op_types[i] == OP_TEXT) {
                    C2D_TextBufClear(tmp_text_buf);
                    C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, draw_ops[i].text);
                    C2D_TextOptimize(&tmp_text_obj);
                    float x = draw_ops[i].x + x_off;
                    float max_w = fmaxf(390.0f - x, 0.0f);
                    C2D_DrawText(&tmp_text_obj, C2D_WithColor | C2D_WordWrap,
                        x, draw_ops[i].y, 0.5f,
                        draw_ops[i].scale, draw_ops[i].scale,
                        draw_ops[i].color,
                        max_w);
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
        // Legacy text fallback (not used in board mode)
    }

    C3D_FrameEnd(0);
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

static u32 uds_sharedmem_size = 0x3000;
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
            if (R_SUCCEEDED(ret)) { result = 0; break; }
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

// ── QR Code Scanning ──
// Replaced with Anemone3DS/FBI-inspired implementation:
//   - Camera thread only captures and copies frames to shared buffer.
//   - Citro2D/C3D calls removed from camera thread (was causing GPU crashes).
//   - Event-based thread synchronization with clean shutdown (no forced threadFree).
//   - draw_preview runs on render thread via existing C3D/C2D pipeline.
#include "quirc.h"
#include <3ds/services/cam.h>
#include <3ds/services/gspgpu.h>

#define QR_W 400
#define QR_H 240

#define QR_MAX_RETRIES 3
#define QR_RETRY_DELAY_MS 500

#define EVENT_STOP 0
#define EVENT_RECV 1
#define EVENT_BUFFER_ERROR 2
#define EVENT_COUNT 3

static volatile bool cam_running = false;
static Handle cam_mutex = 0;
static Handle cam_stop_event = 0;
static Thread cam_thread = NULL;
static u8 *cam_shared_buf = NULL;
static struct quirc *qr = NULL;
static C3D_Tex cam_tex = {0};
static bool cam_tex_inited = false;
static const Tex3DS_SubTexture cam_subtex = { 400, 240, 0.0f, 1.0f, 400.0f/512.0f, 1.0f - (240.0f/256.0f) };

static void cam_thread_func(void *arg) {
    (void)arg;
    Result res = 0;

    u32 bufferSize = QR_W * QR_H * 2;
    u16 *local_buf = (u16*)linearAlloc(bufferSize);
    if (!local_buf) { cam_running = false; return; }

    int retries = 0;
    while (retries < QR_MAX_RETRIES) {
        if (R_FAILED(res = camInit())) {
            retries++;
            if (retries >= QR_MAX_RETRIES) { linearFree(local_buf); cam_running = false; return; }
            svcSleepThread(QR_RETRY_DELAY_MS * 1000000ULL);
            continue;
        }

        u32 cam = SELECT_OUT1;
        if (R_FAILED(res = CAMU_SetSize(cam, SIZE_CTR_TOP_LCD, CONTEXT_A))
            || R_FAILED(res = CAMU_SetOutputFormat(cam, OUTPUT_RGB_565, CONTEXT_A))
            || R_FAILED(res = CAMU_SetFrameRate(cam, FRAME_RATE_30))
            || R_FAILED(res = CAMU_SetNoiseFilter(cam, true))
            || R_FAILED(res = CAMU_SetAutoExposure(cam, true))
            || R_FAILED(res = CAMU_SetAutoWhiteBalance(cam, true))
            || R_FAILED(res = CAMU_Activate(cam))) {
            camExit();
            retries++;
            if (retries >= QR_MAX_RETRIES) { linearFree(local_buf); cam_running = false; return; }
            svcSleepThread(QR_RETRY_DELAY_MS * 1000000ULL);
            continue;
        }

        u32 transferUnit = 0;
        Handle events[EVENT_COUNT] = {0};
        events[EVENT_STOP] = cam_stop_event;

        if (R_FAILED(res = CAMU_GetBufferErrorInterruptEvent(&events[EVENT_BUFFER_ERROR], PORT_CAM1))
            || R_FAILED(res = CAMU_SetTrimming(PORT_CAM1, true))
            || R_FAILED(res = CAMU_SetTrimmingParamsCenter(PORT_CAM1, QR_W, QR_H, 400, 240))
            || R_FAILED(res = CAMU_GetMaxBytes(&transferUnit, QR_W, QR_H))
            || R_FAILED(res = CAMU_SetTransferBytes(PORT_CAM1, transferUnit, QR_W, QR_H))
            || R_FAILED(res = CAMU_ClearBuffer(PORT_CAM1))
            || R_FAILED(res = CAMU_SetReceiving(&events[EVENT_RECV], local_buf, PORT_CAM1, bufferSize, (s16)transferUnit))
            || R_FAILED(res = CAMU_StartCapture(PORT_CAM1))) {
            for (int i = 1; i < EVENT_COUNT; i++) { if (events[i]) { svcCloseHandle(events[i]); events[i] = 0; } }
            CAMU_Activate(SELECT_NONE); camExit();
            retries++;
            if (retries >= QR_MAX_RETRIES) { linearFree(local_buf); cam_running = false; return; }
            svcSleepThread(QR_RETRY_DELAY_MS * 1000000ULL);
            continue;
        }

        while (cam_running) {
            s32 idx = 0;
            if (R_FAILED(svcWaitSynchronizationN(&idx, events, EVENT_COUNT, false, U64_MAX))) break;
            if (idx == EVENT_STOP) break;

            if (idx == EVENT_BUFFER_ERROR) {
                svcCloseHandle(events[EVENT_RECV]);
                events[EVENT_RECV] = 0;
                CAMU_ClearBuffer(PORT_CAM1);
                if (R_FAILED(CAMU_SetReceiving(&events[EVENT_RECV], local_buf, PORT_CAM1, bufferSize, (s16)transferUnit))) break;
                CAMU_StartCapture(PORT_CAM1);
                continue;
            }

            if (idx == EVENT_RECV) {
                svcCloseHandle(events[EVENT_RECV]);
                events[EVENT_RECV] = 0;

                svcWaitSynchronization(cam_mutex, U64_MAX);
                memcpy(cam_shared_buf, local_buf, bufferSize);
                GSPGPU_FlushDataCache(cam_shared_buf, bufferSize);
                svcReleaseMutex(cam_mutex);

                res = CAMU_SetReceiving(&events[EVENT_RECV], local_buf, PORT_CAM1, bufferSize, (s16)transferUnit);
            }
        }

        CAMU_StopCapture(PORT_CAM1);
        bool busy = true;
        while (R_SUCCEEDED(CAMU_IsBusy(&busy, PORT_CAM1)) && busy) svcSleepThread(1000000);
        CAMU_ClearBuffer(PORT_CAM1);
        CAMU_Activate(SELECT_NONE);
        camExit();

        for (int i = 1; i < EVENT_COUNT; i++) { if (events[i]) { svcCloseHandle(events[i]); events[i] = 0; } }
        linearFree(local_buf);
        cam_running = false;
        return;
    }

    linearFree(local_buf);
    cam_running = false;
}

static void _3ds_qr_update_texture(const u16 *buf, int w, int h) {
    if (!cam_tex.data) return;
    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            int si = y * w + x;
            int di = ((((y >> 3) * (512 >> 3) + (x >> 3)) << 6) + ((x & 1) | ((y & 1) << 1) | ((x & 2) << 1) | ((y & 2) << 2) | ((x & 4) << 2) | ((y & 4) << 3)));
            ((u16*)cam_tex.data)[di] = buf[si];
        }
    }
    GSPGPU_FlushDataCache(cam_tex.data, 512 * 256 * 2);
    C3D_TexFlush(&cam_tex);
}

void _3ds_qr_draw_preview(float x_off) {
    if (!cam_running || !cam_shared_buf) return;

    svcWaitSynchronization(cam_mutex, U64_MAX);
    _3ds_qr_update_texture((const u16*)cam_shared_buf, QR_W, QR_H);
    svcReleaseMutex(cam_mutex);

    C2D_Image img = { .tex = &cam_tex, .subtex = &cam_subtex };
    C2D_DrawImageAt(img, x_off, 0.0f, 0.4f, NULL, 1.0f, 1.0f);
}

int _3ds_qr_start(void) {
    if (cam_running) return 0;

    // Clean up leftover resources from a previous session
    if (cam_tex_inited) { C3D_TexDelete(&cam_tex); cam_tex_inited = false; }
    if (cam_shared_buf) { linearFree(cam_shared_buf); cam_shared_buf = NULL; }
    if (cam_mutex) { svcCloseHandle(cam_mutex); cam_mutex = 0; }
    if (cam_stop_event) { svcCloseHandle(cam_stop_event); cam_stop_event = 0; }

    C3D_TexInit(&cam_tex, 512, 256, GPU_RGB565);
    C3D_TexSetFilter(&cam_tex, GPU_LINEAR, GPU_LINEAR);
    cam_tex_inited = true;

    svcCreateMutex(&cam_mutex, false);
    svcCreateEvent(&cam_stop_event, RESET_STICKY);

    cam_shared_buf = (u8*)linearAlloc(QR_W * QR_H * 2);
    if (!cam_shared_buf) {
        svcCloseHandle(cam_mutex); cam_mutex = 0;
        svcCloseHandle(cam_stop_event); cam_stop_event = 0;
        return -1;
    }

    qr = quirc_new();
    if (!qr) { linearFree(cam_shared_buf); cam_shared_buf = NULL; svcCloseHandle(cam_mutex); cam_mutex = 0; svcCloseHandle(cam_stop_event); cam_stop_event = 0; return -2; }
    if (quirc_resize(qr, QR_W, QR_H) < 0) { quirc_destroy(qr); qr = NULL; linearFree(cam_shared_buf); cam_shared_buf = NULL; svcCloseHandle(cam_mutex); cam_mutex = 0; svcCloseHandle(cam_stop_event); cam_stop_event = 0; return -3; }

    cam_running = true;
    cam_thread = threadCreate(cam_thread_func, NULL, 0x40000, 0x1A, -1, true);
    if (!cam_thread) {
        cam_running = false;
        quirc_destroy(qr); qr = NULL;
        linearFree(cam_shared_buf); cam_shared_buf = NULL;
        svcCloseHandle(cam_mutex); cam_mutex = 0;
        svcCloseHandle(cam_stop_event); cam_stop_event = 0;
        return -4;
    }

    return 0;
}

void _3ds_qr_stop(void) {
    if (!cam_running) return;
    cam_running = false;
    svcSignalEvent(cam_stop_event);
    if (cam_thread) {
        threadJoin(cam_thread, U64_MAX);
        threadFree(cam_thread);
        cam_thread = NULL;
    }
    if (qr) { quirc_destroy(qr); qr = NULL; }
    // Do NOT free cam_tex, cam_shared_buf, cam_mutex, or cam_stop_event here.
    // The render thread uses them in _3ds_qr_draw_preview and may still be
    // mid-draw when this function returns. They are freed in _3ds_qr_start()
    // before re-allocating, or on app exit.
}

int _3ds_qr_poll(char *out_text, unsigned int out_max) {
    if (out_max == 0) return -1;
    if (!cam_running || !cam_shared_buf) return -1;
    if (!qr) return -1;

    int w = 0, h = 0;
    uint8_t *qbuf = quirc_begin(qr, &w, &h);
    if (!qbuf) return 0;

    svcWaitSynchronization(cam_mutex, U64_MAX);

    for (int y = 0; y < h; y++) {
        for (int x = 0; x < w; x++) {
            u16 px = ((u16*)cam_shared_buf)[y * QR_W + x];
            qbuf[y * w + x] = (u8)(((((px >> 11) & 0x1F) << 3) + (((px >> 5) & 0x3F) << 2) + ((px & 0x1F) << 3)) / 3);
        }
    }

    svcReleaseMutex(cam_mutex);

    quirc_end(qr);

    int qrCount = quirc_count(qr);
    for (int i = 0; i < qrCount; i++) {
        struct quirc_code code;
        struct quirc_data data;
        quirc_extract(qr, i, &code);
        if (quirc_decode(&code, &data) == QUIRC_SUCCESS) {
            int len = data.payload_len;
            if (len > (int)out_max - 1) len = (int)out_max - 1;
            if (len < 0) len = 0;
            memcpy(out_text, data.payload, len);
            out_text[len] = '\0';
            return len;
        }
    }
    return 0;
}

// ===================== AUDIO (CSND — no dspfirm.cdc needed) =====================

static void* s_audio_data = NULL;
static u32 s_audio_size = 0;
static volatile int s_audio_playing = 0;
static volatile int s_audio_exit = 0;
static Thread s_audio_thread;

static void _3ds_audio_stop(void);

static int decode_ogg_to_mem(const char* path, int16_t** out_pcm, u32* out_samples, int* out_channels) {
    FILE* f = fopen(path, "rb");
    if (!f) return -1;

    ov_callbacks cb = { 0 };
    cb.read_func = (size_t(*)(void*, size_t, size_t, void*))fread;
    cb.seek_func = (int(*)(void*, int64_t, int))fseek;
    cb.tell_func = (long(*)(void*))ftell;

    OggVorbis_File vf;
    if (ov_open_callbacks(f, &vf, NULL, 0, cb) < 0) {
        fclose(f);
        return -1;
    }

    vorbis_info* vi = ov_info(&vf, -1);
    *out_channels = vi->channels;
    int rate = vi->rate;
    int max_samples = rate * 60 * vi->channels;  // up to 60s

    *out_pcm = (int16_t*)linearAlloc(max_samples * 2);
    if (!*out_pcm) {
        ov_clear(&vf);
        return -1;
    }

    int total = 0;
    while (total < max_samples) {
        long ret = ov_read(&vf, (char*)((*out_pcm) + total), (max_samples - total) * 2, NULL);
        if (ret <= 0) break;
        total += ret / 2;
    }
    ov_clear(&vf);

    *out_samples = total;
    return rate;
}

static void audio_loop_thread_func(void* arg) {
    (void)arg;
    while (!s_audio_exit) {
        // Wait until playback finishes
        while (s_audio_playing && !s_audio_exit) {
            u8 status = 0;
            csndIsPlaying(0, &status);
            if (!status) {
                // Restart playback automatically to loop music forever
                DSP_FlushDataCache(s_audio_data, s_audio_size);
                csndPlaySound(0,
                    SOUND_FORMAT_16BIT | SOUND_LINEAR_INTERP,
                    44100,
                    1.0f, 1.0f,
                    (u32*)s_audio_data, NULL,
                    s_audio_size);
            }
            for (int i = 0; i < 50 && !s_audio_exit; i++) {
                svcSleepThread(10000000ULL);  // 10ms
            }
        }
        if (!s_audio_exit) {
            svcSleepThread(10000000ULL);  // 10ms
        }
    }
}

void _3ds_audio_init(void) {
    csndInit();
}

void _3ds_audio_play_ogg(const char* path) {
    _3ds_audio_stop();

    int channels = 0;
    u32 samples = 0;
    int rate = decode_ogg_to_mem(path, (int16_t**)&s_audio_data, &samples, &channels);
    if (rate < 0 || !s_audio_data) return;

    s_audio_size = samples * sizeof(int16_t);
    DSP_FlushDataCache(s_audio_data, s_audio_size);
    s_audio_playing = 1;
    s_audio_exit = 0;

    csndPlaySound(0,
        SOUND_FORMAT_16BIT | SOUND_LINEAR_INTERP,
        rate,
        1.0f, 1.0f,
        (u32*)s_audio_data, NULL,
        s_audio_size);

    // Start looping thread
    s_audio_thread = threadCreate(audio_loop_thread_func, NULL, 0x4000, 0x18, -1, true);
}

void _3ds_audio_stop(void) {
    s_audio_exit = 1;
    s_audio_playing = 0;
    if (s_audio_thread) {
        threadJoin(s_audio_thread, 0xFFFFFFFF);
        threadFree(s_audio_thread);
        s_audio_thread = NULL;
    }
    csndPlaySound(0, 0, 0, 0, 0, NULL, NULL, 0);
    if (s_audio_data) {
        linearFree(s_audio_data);
        s_audio_data = NULL;
    }
}

void _3ds_audio_set_volume(float vol) {
    // csnd volume is set per-play, just store for next retrigger
    (void)vol;
}
