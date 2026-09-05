//! GBA SRAM access for saving deck data (`.sav` format).
//! 
//! NOTE: This is a stub implementation. The agb crate's flash API has changed.
//! Proper SRAM/flash access needs to be implemented using the current agb API.

use rabuka_engine::game::sav::{decode_sav, SavDeck};

/// Write SAV image to SRAM (flash) - STUB
pub fn write_sav(_data: &[u8]) {
    // TODO: Implement using agb's current flash/SRAM API
    // For now, this is a no-op
}

/// Read and decode SAV decks from SRAM (flash) - STUB
pub fn read_sav_decks() -> alloc::vec::Vec<SavDeck> {
    // TODO: Implement using agb's current flash/SRAM API
    alloc::vec![]
}