// Rabuka 3DS — C shim bridging libctru services to Rust.
//
// This file fulfills several roles that ctru-rs would normally handle
// via safe wrappers, but we avoid ctru-rs here because its build-time
// bindgen dependency creates toolchain issues on Windows (libclang.dll,
// header paths, etc.). Instead we call libctru directly.
//
// Services provided:
//   getrandom()      — required by Rust std + rand crate on unsupported targets
//   _3ds_init/deinit — GPU, console, RomFS lifecycle
//   _3ds_swap_buffers — flip screen buffers for display updates
//   aptMainLoop()    — called from Rust's _3ds_main_loop()
//
// Key details:
// - gfxInitDefault + consoleInit: sets up stdout to render on the 3DS top screen.
// - romfsInit: required for romfs:/ path access (cards.json, decks).
// - gspWaitForVBlank: synchronizes frame timing.
// - svcGetSystemTick: 64-bit tick counter used as entropy source.

#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <3ds.h>
#include <errno.h>
// Increase default heap size to 64MB for parsing large JSON files
u32 __ctru_heap_size = 64 * 1024 * 1024;
// Increase default stack size to 4MB (ctru-rs uses __stacksize__, not __ctru_stack_size).
// The default ~32KB stack is insufficient for large Rust functions with many HashMap/Vec
// locals when the compiler aggressively inlines callees into the caller's stack frame.
u32 __stacksize__ = 2 * 1024 * 1024;

static PrintConsole top_console;
static PrintConsole bot_console;

void _3ds_init() {
    gfxInitDefault();
    consoleInit(GFX_TOP, &top_console);
    consoleInit(GFX_BOTTOM, &bot_console);
    consoleSelect(&bot_console);
    romfsInit();
}

void _3ds_select_top() {
    consoleSelect(&top_console);
}

void _3ds_select_bottom() {
    consoleSelect(&bot_console);
}

void _3ds_clear_console() {
    consoleClear();
}

void _3ds_exit() {
    romfsExit();
    gfxExit();
}

int _3ds_main_loop() {
    return aptMainLoop();
}

// Called periodically during long operations (JSON parsing, etc.)
// to keep the 3DS OS responsive.  Returns 0 when the app should exit.
int _3ds_keep_alive() {
    int alive = aptMainLoop();
    gfxFlushBuffers();
    gfxSwapBuffers();
    return alive;
}

void _3ds_swap_buffers() {
    gfxFlushBuffers();
    gfxSwapBuffers();
    gspWaitForVBlank();
}

void _3ds_debug_print(const char *msg) {
    svcOutputDebugString(msg, strlen(msg));
}

// Print a debug message to both:
// 1. Debug console (via svcOutputDebugString) — ALWAYS works, visible in Luma3DS/emulator
// 2. Top screen (via consoleSelect + printf) — visible on physical 3DS screen
void _3ds_tdbg(const char *msg) {
    svcOutputDebugString(msg, strlen(msg));
    svcOutputDebugString("\n", 1);
    consoleSelect(&top_console);
    printf("%s\n", msg);
    consoleSelect(&bot_console);
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
