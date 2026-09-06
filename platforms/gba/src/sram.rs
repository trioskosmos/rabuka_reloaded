//! GBA SRAM deck slots (`.sav` image): raw 32 KiB SRAM at `0x0E000000`.
//!
//! The image layout is owned by engine `game::sav` (magic, version,
//! per-entry checksums); this module only moves bytes. The `SRAM_Vnnn`
//! marker tells emulators and flashcarts to provide SRAM and persist
//! `romname.sav` — same marker `agb` emits internally (its marker module
//! is private, so we emit the identical bytes ourselves).
//!
//! Deliberately NOT `agb`'s slot manager: it reformats storage whose
//! magic it doesn't recognise, which would wipe converter-written files
//! on first boot. Our own magic + checksum validates instead; anything
//! unrecognised (fresh/erased SRAM) reads as "no decks".
//!
//! Single-threaded GBA, synchronous use only.

extern crate alloc;

use alloc::vec::Vec;
use rabuka_engine::game::sav::{decode_sav, SavDeck};
const SRAM_BASE: *mut u8 = 0x0E00_0000 as *mut u8;
const SRAM_LEN: usize = 32 * 1024;

#[repr(align(4))]
struct Align<T>(T);

/// Save-type marker scanned by emulators/flashcarts (see GBATEK "Backup
/// Media"). `#[used]` keeps it in ROM; the `black_box` in each accessor
/// keeps the compiler from proving it unread.
#[used]
static SRAM_MARKER: Align<[u8; 12]> = Align(*b"SRAM_Vnnn\0\0\0");

/// Write a `.sav` image to SRAM. `data` must fit (the codec caps images
/// at ~14 KiB, well under 32 KiB) — enforced, fail loud, never truncate.
pub fn write_sav(data: &[u8]) {
    core::hint::black_box(&SRAM_MARKER);
    assert!(
        data.len() <= SRAM_LEN,
        "sav image {}B exceeds SRAM {}B",
        data.len(),
        SRAM_LEN
    );
    unsafe {
        core::ptr::copy_nonoverlapping(data.as_ptr(), SRAM_BASE, data.len());
    }
}

/// Read and validate SRAM decks. Empty, erased, corrupt, or foreign SRAM
/// yields an empty vec — the game boots normally with baked decks only.
pub fn read_sav_decks() -> Vec<SavDeck> {
    core::hint::black_box(&SRAM_MARKER);
    let bytes =
        unsafe { core::slice::from_raw_parts(SRAM_BASE as *const u8, SRAM_LEN) };
    match decode_sav(bytes) {
        Ok(decks) => {
            log::debug!("[SRAM] {} deck(s) loaded", decks.len());
            decks
        }
        Err(e) => {
            log::debug!("[SRAM] no decks ({:?})", e);
            Vec::new()
        }
    }
}
