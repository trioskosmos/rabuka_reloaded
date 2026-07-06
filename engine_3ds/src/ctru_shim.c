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
#include <3ds.h>
#include <errno.h>
// Increase default heap size to 64MB for parsing large JSON files
u32 __ctru_heap_size = 64 * 1024 * 1024;

void _3ds_init() {
    gfxInitDefault();
    consoleInit(GFX_TOP, NULL);
    romfsInit();
}

void _3ds_exit() {
    romfsExit();
    gfxExit();
}

int _3ds_main_loop() {
    return aptMainLoop();
}

void _3ds_swap_buffers() {
    gspWaitForVBlank();
    gfxSwapBuffers();
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
