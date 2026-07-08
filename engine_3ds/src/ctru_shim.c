// Rabuka 3DS — C shim using citro2d for board rendering + text display.

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
} CardSlot;

static bool board_mode = false;
static bool show_opponent = false;
static int board_sel_slot = -1;     // -1 = no selection
static int board_sel_type = 0;      // 0=stage, 1=live, 2=energy, 3=hand

// Slots
static CardSlot bot_stage[3];
static CardSlot bot_live[3];
static CardSlot bot_energy[MAX_SLOTS];
static int bot_energy_count;
static CardSlot bot_hand[MAX_SLOTS];
static int bot_hand_count;
// Utility counts
static int bot_deck_count, bot_edeck_count, bot_discard_count, bot_success_count;

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

// ---- Forward declarations ----
C2D_Image _3ds_get_card_image(const char* atlas_name, int index);

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

    // Initialize board state
    atlas_count = 0;
    board_mode = false;
}

float _3ds_bot_line_height() {
    C2D_Font f = custom_font ? custom_font : NULL;
    C2D_TextBuf tmp = C2D_TextBufNew(128);
    C2D_Text t;
    C2D_TextFontParse(&t, f, tmp, "A\nA\0");
    C2D_TextOptimize(&t);
    float w, h;
    C2D_TextGetDimensions(&t, 0.75f, 0.75f, &w, &h);
    C2D_TextBufDelete(tmp);
    if (h <= 0) return 26.0f;
    return h / 2.0f;
}

void _3ds_exit() {
    for (int i = 0; i < atlas_count; i++) {
        C2D_SpriteSheetFree(atlases[i].sheet);
    }
    C2D_TextBufDelete(top_buf);
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
void _3ds_board_enable(bool on) {
    board_mode = on;
}

void _3ds_board_toggle_side() {
    show_opponent = !show_opponent;
}

bool _3ds_board_is_opponent() {
    return show_opponent;
}

void _3ds_board_clear_cache() {
    for (int i = 0; i < atlas_count; i++) {
        C2D_SpriteSheetFree(atlases[i].sheet);
    }
    atlas_count = 0;
}

void _3ds_board_set_slot(CardSlot* slot, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    slot->active = active;
    if (atlas) {
        strncpy(slot->atlas, atlas, 63);
        slot->atlas[63] = '\0';
    } else {
        slot->atlas[0] = '\0';
    }
    slot->index = index;
    slot->landscape = landscape;
    slot->tapped = tapped;
}

void _3ds_board_set_stage(int slot_i, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    if (slot_i >= 0 && slot_i < 3)
        _3ds_board_set_slot(&bot_stage[slot_i], active, atlas, index, landscape, tapped);
}

void _3ds_board_set_live(int slot_i, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    if (slot_i >= 0 && slot_i < 3)
        _3ds_board_set_slot(&bot_live[slot_i], active, atlas, index, landscape, tapped);
}

void _3ds_board_set_energy(int slot_i, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    if (slot_i >= 0 && slot_i < MAX_SLOTS)
        _3ds_board_set_slot(&bot_energy[slot_i], active, atlas, index, landscape, tapped);
}

void _3ds_board_set_energy_count(int count) {
    bot_energy_count = count > MAX_SLOTS ? MAX_SLOTS : count;
}

void _3ds_board_set_hand(int slot_i, bool active, const char* atlas, int index, bool landscape, bool tapped) {
    if (slot_i >= 0 && slot_i < MAX_SLOTS)
        _3ds_board_set_slot(&bot_hand[slot_i], active, atlas, index, landscape, tapped);
}

void _3ds_board_set_hand_count(int count) {
    bot_hand_count = count > MAX_SLOTS ? MAX_SLOTS : count;
}

void _3ds_board_set_utility(int deck, int edeck, int discard, int success) {
    bot_deck_count = deck;
    bot_edeck_count = edeck;
    bot_discard_count = discard;
    bot_success_count = success;
}

void _3ds_board_set_selection(int slot_idx, int slot_type) {
    board_sel_slot = slot_idx;
    board_sel_type = slot_type;
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

    float sx = w / (float)img.subtex->width;
    float sy = h / (float)img.subtex->height;
    C2D_DrawImageAt(img, x, y, 0.5f, NULL, sx, sy);

    if (slot->tapped) {
        _3ds_draw_rect(x, y, w, h, COL_TAPPED);
    }
}

// ---- Board render (bottom screen) ----
void _3ds_render_board() {
    C2D_SceneBegin(bot_target);
    C2D_TargetClear(bot_target, COL_BG);

    C2D_Font f = custom_font ? custom_font : NULL;
    float depth = 0.0f;

    // ======= LAYOUT CONSTANTS (320x240) =======
    const float W = 320.0f;
    const float H = 240.0f;
    const float M = 2.0f;  // margin

    // Live zone: top
    float live_y = M;
    float live_h = 48.0f;
    float slot_w = 90.0f;
    float slot_h = live_h - 4;

    // Stage row: below live
    float stage_y = live_y + live_h + M;
    float stage_h = 80.0f;
    float st_slot_w = 60.0f;
    float st_slot_h = stage_h - 4;

    // Utility column: right of stage
    float util_x = M + 3 * (st_slot_w + 2) + 90;
    float util_w = W - util_x - M;

    // Energy: below stage
    float energy_y = stage_y + stage_h + M;
    float energy_h = 20.0f;

    // Hand: bottom
    float hand_y = energy_y + energy_h + M;
    float hand_h = H - hand_y - M;

    // ======= 1. LIVE ZONE =======
    _3ds_draw_rect(M, live_y, W - 2 * M, live_h, COL_ZONE_BG);
    _3ds_draw_border(M, live_y, W - 2 * M, live_h, COL_PINK, 1.0f);
    // Slot backgrounds
    float lx = M + 4;
    for (int i = 0; i < 3; i++) {
        float sy = live_y + 2;
        _3ds_draw_rect(lx, sy, slot_w, slot_h, 0x33000000);
        if (bot_live[i].active) {
            _3ds_draw_card_at(&bot_live[i], lx, sy, slot_w, slot_h);
        }
        lx += slot_w + 2;
    }
    _3ds_draw_label("LIVE", M + 4, live_y + live_h - 12, COL_PINK, 0.5f);

    // ======= 2. STAGE + UTILITY =======
    float st_x = M;
    for (int i = 0; i < 3; i++) {
        float sy = stage_y + 2;
        _3ds_draw_rect(st_x, sy, st_slot_w, st_slot_h, COL_ZONE_BG);
        _3ds_draw_border(st_x, sy, st_slot_w, st_slot_h, COL_BLUE, 1.0f);
        if (bot_stage[i].active) {
            _3ds_draw_card_at(&bot_stage[i], st_x + 1, sy + 1, st_slot_w - 2, st_slot_h - 2);
        }
        st_x += st_slot_w + 2;
    }

    // Utility column
    _3ds_draw_rect(util_x, stage_y, util_w, stage_h, COL_ZONE_BG);
    _3ds_draw_border(util_x, stage_y, util_w, stage_h, COL_ZONE_BDR, 1.0f);

    char util_buf[64];
    // We'll draw small text labels
    snprintf(util_buf, sizeof(util_buf), "D:%d", bot_deck_count);
    _3ds_draw_label(util_buf, util_x + 2, stage_y + 2, COL_TEXT, 0.45f);
    snprintf(util_buf, sizeof(util_buf), "E:%d", bot_edeck_count);
    _3ds_draw_label(util_buf, util_x + 2, stage_y + 14, COL_TEXT, 0.45f);
    snprintf(util_buf, sizeof(util_buf), "W:%d", bot_discard_count);
    _3ds_draw_label(util_buf, util_x + 2, stage_y + 26, COL_TEXT, 0.45f);
    snprintf(util_buf, sizeof(util_buf), "S:%d", bot_success_count);
    _3ds_draw_label(util_buf, util_x + 2, stage_y + 38, COL_TEXT, 0.45f);

    // Stage label
    _3ds_draw_label("STAGE", M + 4, stage_y + stage_h - 12, COL_BLUE, 0.5f);

    // ======= 3. ENERGY =======
    _3ds_draw_rect(M, energy_y, W - 2 * M, energy_h, COL_ZONE_BG);
    _3ds_draw_border(M, energy_y, W - 2 * M, energy_h, COL_GOLD, 1.0f);
    float ex = M + 2;
    float e_sz = energy_h - 4;
    for (int i = 0; i < bot_energy_count && i < MAX_SLOTS; i++) {
        if (bot_energy[i].active) {
            _3ds_draw_card_at(&bot_energy[i], ex, energy_y + 2, e_sz * 0.7f, e_sz);
        } else {
            _3ds_draw_rect(ex, energy_y + 2, e_sz * 0.7f, e_sz, 0x33000000);
        }
        ex += e_sz * 0.7f + 1;
        if (ex > W - M - e_sz) break;
    }
    _3ds_draw_label("ENERGY", M + 4, energy_y + energy_h - 12, COL_GOLD, 0.5f);

    // ======= 4. HAND =======
    _3ds_draw_rect(M, hand_y, W - 2 * M, hand_h, COL_ZONE_BG);
    _3ds_draw_border(M, hand_y, W - 2 * M, hand_h, COL_TEXT, 1.0f);
    float hx = M + 2;
    float h_slot_w = 40.0f;
    float h_slot_h = hand_h - 4;
    for (int i = 0; i < bot_hand_count && i < MAX_SLOTS; i++) {
        if (bot_hand[i].active) {
            _3ds_draw_card_at(&bot_hand[i], hx, hand_y + 2, h_slot_w, h_slot_h);
        }
        hx += h_slot_w + 1;
        if (hx > W - M - h_slot_w) break;
    }
    _3ds_draw_label("HAND", M + 4, hand_y + hand_h - 12, COL_TEXT, 0.5f);

    // ======= 5. SELECTION HIGHLIGHT =======
    if (board_sel_slot >= 0 && board_sel_type == 0 && board_sel_slot < 3) {
        float sx = M + board_sel_slot * (st_slot_w + 2);
        _3ds_draw_border(sx, stage_y + 2, st_slot_w, st_slot_h, COL_SEL, 2.0f);
    }
    if (board_sel_slot >= 0 && board_sel_type == 1 && board_sel_slot < 3) {
        float sx = M + 4 + board_sel_slot * (slot_w + 2);
        _3ds_draw_border(sx, live_y + 2, slot_w, slot_h, COL_SEL, 2.0f);
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

    // TOP SCREEN: text info (stats, card details)
    C2D_TargetClear(top_target, C2D_Color32(0, 0, 0, 255));
    C2D_SceneBegin(top_target);
    if (top_parsed) {
        C2D_DrawText(&top_obj,
            C2D_WithColor | C2D_WordWrap,
            2.0f, 2.0f, 0.5f,
            0.85f, 0.85f,
            C2D_Color32(0, 255, 0, 255),
            390.0f);
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
