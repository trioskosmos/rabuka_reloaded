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
#include <3ds.h>
#include <citro2d.h>
#include <errno.h>

u32 __ctru_heap_size = 64 * 1024 * 1024;
u32 __stacksize__ = 2 * 1024 * 1024;

// ---- Text rendering (top screen: stats + card info) ----
#define TEXTLEN 8192
static char top_text[TEXTLEN];
static C3D_RenderTarget* top_target = NULL;
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
    char text[64];
} DrawOp;
#define OP_RECT 0
#define OP_TEXT 1
static DrawOp draw_ops[MAX_DRAW_OPS];
static int   draw_op_count = 0;
static int   draw_op_types[MAX_DRAW_OPS];
static u32   COL_TOP_BG = 0xFF0A0E1A; // very dark navy

// ---- Board HUD overlay state ----
static int   hud_turn = 0;
static char  hud_phase[32] = "";
static char  hud_player[8] = "";

// ---- Active-player highlight ----
static bool active_is_p1 = true;

// ---- Action highlight on board slots ----
static int hl_zone = -1;
static int hl_slot = -1;

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

// ---- init / exit ----
void _3ds_init() {
    gfxInitDefault();
    C3D_Init(C3D_DEFAULT_CMDBUF_SIZE);
    C2D_Init(C2D_DEFAULT_MAX_OBJECTS);
    C2D_Prepare();

    top_target = C2D_CreateScreenTarget(GFX_TOP, GFX_LEFT);
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
    hud_turn = 0; hud_phase[0] = '\0'; hud_player[0] = '\0';
    active_is_p1 = true;
    hl_zone = -1; hl_slot = -1;
    tmp_text_buf = C2D_TextBufNew(8192);
}

// Measure per-line height for the custom font at scale 0.85.
// Parses a two-line string, gets total height, divides by 2.
// At scale 0.85: ceil(0.85 * (30/42) * 31) = 19px per line.
// Fallback 30.0px if measurement fails (matches system font at scale 1.0).
float _3ds_bot_line_height() {
    C2D_Font f = custom_font ? custom_font : NULL;
    C2D_TextBuf tmp = C2D_TextBufNew(128);
    C2D_Text t;
    C2D_TextFontParse(&t, f, tmp, "A\nA\0");
    C2D_TextOptimize(&t);
    float w, h;
    C2D_TextGetDimensions(&t, 0.85f, 0.85f, &w, &h);
    C2D_TextBufDelete(tmp);
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
void _3ds_top_clear() { draw_op_count = 0; }

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
    strncpy(draw_ops[i].text, text, 63); draw_ops[i].text[63] = '\0';
}

// ---- Board HUD ----
void _3ds_board_set_hud(int turn, const char* phase, const char* player) {
    hud_turn = turn;
    strncpy(hud_phase, phase ? phase : "", 31); hud_phase[31] = '\0';
    strncpy(hud_player, player ? player : "", 7); hud_player[7] = '\0';
}

void _3ds_board_set_active_player(bool is_p1) { active_is_p1 = is_p1; }

// ---- Action highlight ----
void _3ds_board_set_action_highlight(int zone, int slot) { hl_zone = zone; hl_slot = slot; }
void _3ds_board_clear_action_highlight() { hl_zone = -1; hl_slot = -1; }

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
    C2D_TextBuf tmp = C2D_TextBufNew(256);
    C2D_Text t;
    C2D_TextFontParse(&t, f, tmp, label);
    C2D_TextOptimize(&t);
    C2D_DrawText(&t, C2D_WithColor, x, y, 0.6f, scale, scale, color);
    C2D_TextBufDelete(tmp);
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
// With font scale 0.65 (~20px glyph), each zone needs enough height for its
// label line plus content. Energy gets 15% (was 9%) to fit the "ENERGY" label.
static void zone_heights(float h, float* live, float* stage, float* energy, float* hand) {
    float u = h - 3.0f;
    *live   = u * 0.20f;
    *stage  = u * 0.25f;
    *energy = u * 0.15f;
    *hand   = u * 0.40f;
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
        if (!cli_mode && hl_zone == 0 && hl_slot == i) {
            _3ds_draw_border(lx, live_y + 1, live_slot_w, live_card_h, COL_SEL, 2);
        }
        if (pb->live[i].active) _3ds_draw_card_at(&pb->live[i], lx, live_y + 1, live_slot_w, live_card_h);
        lx += live_slot_w + 2;
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
        if (!cli_mode && hl_zone == 1 && hl_slot == si) {
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

    // Utility counts
    _3ds_draw_rect(util_x, stage_y, util_w, stage_h, COL_ZONE_BG);
    _3ds_draw_border(util_x, stage_y, util_w, stage_h, COL_ZONE_BDR, 1);
    // Utility counts: scale 0.70 = 21px glyph, 0.65 = 20px fallback.
    // Line spacing fy += 14 accommodates 21px glyph + 1px gap.
    char buf[40];
    float fs = stage_h > 40 ? 0.70f : 0.65f;
    float fy = stage_y + 1;
    snprintf(buf, sizeof(buf), "D:%d", pb->deck);
    _3ds_draw_label(buf, util_x + 1, fy, COL_TEXT, fs); fy += 14;
    snprintf(buf, sizeof(buf), "E:%d", pb->edeck);
    _3ds_draw_label(buf, util_x + 1, fy, COL_TEXT, fs); fy += 14;
    snprintf(buf, sizeof(buf), "W:%d", pb->discard);
    _3ds_draw_label(buf, util_x + 1, fy, COL_TEXT, fs); fy += 14;
    snprintf(buf, sizeof(buf), "S:%d", pb->success);
    _3ds_draw_label(buf, util_x + 1, fy, COL_TEXT, fs);

    // === ENERGY ===
    _3ds_draw_rect(M, energy_y, W - 2 * M, energy_h, COL_ZONE_BG);
    _3ds_draw_border(M, energy_y, W - 2 * M, energy_h, COL_GOLD, 1);
    float ex = M + 2;
    float e_sz = energy_h - 4;
    for (int i = 0; i < pb->energy_count && i < MAX_SLOTS; i++) {
        float e_w = e_sz * LANDSCAPE;
        if (!cli_mode && hl_zone == 2 && hl_slot == i) {
            _3ds_draw_border(ex, energy_y + 2, e_w, e_sz, COL_SEL, 2);
        }
        if (pb->energy[i].active) _3ds_draw_card_at(&pb->energy[i], ex, energy_y + 2, e_w, e_sz);
        else _3ds_draw_rect(ex, energy_y + 2, e_w, e_sz, 0x33000000);
        ex += e_w + 1;
        if (ex > W - M - e_w) break;
    }

    // === HAND ===
    _3ds_draw_rect(M, hand_y, W - 2 * M, hand_h, COL_ZONE_BG);
    _3ds_draw_border(M, hand_y, W - 2 * M, hand_h, COL_TEXT, 1);
    float hx = M + 2;
    float hand_card_h = hand_h - 4;
    float h_slot_w = hand_card_h * PORTRAIT;
    for (int i = 0; i < pb->hand_count && i < MAX_SLOTS; i++) {
        if (!cli_mode && hl_zone == 3 && hl_slot == i) {
            _3ds_draw_border(hx, hand_y + 2, h_slot_w, hand_card_h, COL_SEL, 2);
        }
        if (pb->hand[i].active) _3ds_draw_card_at(&pb->hand[i], hx, hand_y + 2, h_slot_w, hand_card_h);
        hx += h_slot_w + 2;
        if (hx > W - M - h_slot_w) break;
    }
    // Hand scroll indicator: ◀/▶ arrows + range "off+1-off+vis/total"
    // Shown at the right side of the hand zone, above any overflow
    float range_x = W - M - 60;
    float range_y = hand_y + 2;
    char rbuf[48];
    int show = hand_range_off + 1;
    int show_end = (hand_range_off + hand_range_vis) < hand_range_total
                    ? (hand_range_off + hand_range_vis) : hand_range_total;
    snprintf(rbuf, sizeof(rbuf), "%s%d-%d/%d%s",
        hand_range_off > 0 ? "<" : " ",
        show, show_end, hand_range_total,
        show_end < hand_range_total ? ">" : " ");
    _3ds_draw_label(rbuf, range_x, range_y, COL_GOLD, 0.45f);
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
        // Active player highlight: gold border around their section
        float hl_y = active_is_p1 ? (div_y + 4) : 2;
        float hl_h = active_is_p1 ? (240 - div_y - 4) : half;
        _3ds_draw_border(0, hl_y, 320, hl_h, COL_SEL, 2);
    } else if (board_view == 1) {
        _3ds_board_set_section_rect(0, 240, false);
        draw_section(&o_board, 0, 240, false, true);
    } else {
        _3ds_board_set_section_rect(0, 240, false);
        draw_section(&p_board, 0, 240, false, false);
    }

    // HUD overlay bar (game mode only)
    // HUD overlay bar: scale 0.65 = ~20px glyph. Bar height 22px fits one line.
    if (!cli_mode && hud_turn > 0) {
        C2D_DrawRectSolid(0, 0, 0.5f, 320, 22, C2D_Color32(0, 0, 0, 180));
        char hbuf[64];
        snprintf(hbuf, sizeof(hbuf), "T%d %s [%s]", hud_turn, hud_phase, hud_player);
        _3ds_draw_label(hbuf, 4, 1, COL_SEL, 0.65f);
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

    // View indicator + hand range: scale 0.70 = 21px, 0.65 = 20px.
    if (!cli_mode && overlay_count > 0) return; // overlay covers it
    const char* view_label = board_view == 0 ? "YOU" : (board_view == 1 ? "OPP" : "BOTH");
    _3ds_draw_label(view_label, 275, 218, COL_GOLD, 0.70f);
    char hbuf2[20];
    int e = (hand_range_off + hand_range_vis) < hand_range_total
              ? (hand_range_off + hand_range_vis) : hand_range_total;
    snprintf(hbuf2, sizeof(hbuf2), "%d-%d/%d", hand_range_off + 1, e, hand_range_total);
    _3ds_draw_label(hbuf2, 275, 206, COL_TEXT, 0.65f);
    if (hand_range_off > 0 && (hand_range_off + hand_range_vis) < hand_range_total) {
        _3ds_draw_label("< >", 275, 196, COL_GOLD, 0.65f);
    } else if (hand_range_off > 0) {
        _3ds_draw_label("<", 275, 196, COL_GOLD, 0.65f);
    } else if ((hand_range_off + hand_range_vis) < hand_range_total) {
        _3ds_draw_label(">", 275, 196, COL_GOLD, 0.65f);
    }
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

    // TOP SCREEN
    if (cli_mode) {
        // CLI/debug mode: green text on black at scale 0.85 = 26px glyph.
        C2D_TargetClear(top_target, C2D_Color32(0, 0, 0, 255));
        C2D_SceneBegin(top_target);
        if (top_parsed) {
            C2D_DrawText(&top_obj,
                C2D_WithColor,
                2.0f, 2.0f - (float)top_scroll_y, 0.5f,
                0.85f, 0.85f,
                C2D_Color32(0, 255, 0, 255),
                390.0f);
        }
    } else {
        // Game mode: render queued draw ops
        C2D_TargetClear(top_target, COL_TOP_BG);
        C2D_SceneBegin(top_target);
        C2D_TextBufClear(tmp_text_buf);
        C2D_Font f = custom_font ? custom_font : NULL;
        for (int i = 0; i < draw_op_count; i++) {
            if (draw_op_types[i] == OP_RECT) {
                C2D_DrawRectSolid(draw_ops[i].x, draw_ops[i].y, 0.5f,
                    draw_ops[i].w, draw_ops[i].h, draw_ops[i].color);
            } else if (draw_op_types[i] == OP_TEXT) {
                C2D_TextFontParse(&tmp_text_obj, f, tmp_text_buf, draw_ops[i].text);
                C2D_TextOptimize(&tmp_text_obj);
                C2D_DrawText(&tmp_text_obj, C2D_WithColor,
                    draw_ops[i].x, draw_ops[i].y, 0.5f,
                    draw_ops[i].scale, draw_ops[i].scale,
                    draw_ops[i].color);
            }
        }
    }

    // BOTTOM SCREEN: board or text
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
