//! GBA SRAM access for saving deck data (`.sav` format).

use agb::flash::{Flash, FlashSize};
use rabuka_engine::game::sav::{decode_sav, SavDeck};

/// Write SAV image to SRAM (flash).
pub fn write_sav(data: &[u8]) {
    // GBA flash is 64KB or 128KB. We use the first 32KB for our SAV.
    // The `agb` crate provides Flash access.
    let mut flash = Flash::new(FlashSize::Size64K).expect("Flash init failed");
    
    // Erase the first sector (4KB) - SAV is ~14KB so we need a few sectors
    // Flash erase is done in 4KB sectors
    for sector in 0..4 {
        flash.erase_sector(sector * 4096).expect("Flash erase failed");
    }
    
    // Write data in 256-byte pages
    for (i, chunk) in data.chunks(256).enumerate() {
        let addr = i * 256;
        flash.write(addr, chunk).expect("Flash write failed");
    }
}

/// Read and decode SAV decks from SRAM (flash).
pub fn read_sav_decks() -> alloc::vec::Vec<SavDeck> {
    let mut flash = match Flash::new(FlashSize::Size64K) {
        Ok(f) => f,
        Err(_) => return alloc::vec![],
    };
    let mut buf = alloc::vec![0u8; 32 * 1024]; // 32KB max
    
    // Read first 32KB
    if flash.read(0, &mut buf).is_err() {
        return alloc::vec![];
    }
    
    // Decode SAV
    match decode_sav(&buf) {
        Ok(decks) => decks,
        Err(_) => alloc::vec![],
    }
}