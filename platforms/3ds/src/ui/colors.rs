// Precomputed color constants for game-mode rendering.
// C2D stores colors as 0xAABBGGRR in the u32 literal:
//   bits 31-24 = Alpha
//   bits 23-16 = Blue
//   bits 15-8  = Green
//   bits 7-0   = Red
// The GPU on 3DS reads little-endian memory bytes as RGBA,
// so the u32 literal must be AABBGGRR (A in MSB, R in LSB).

pub const COL_TOP_BG: u32 = 0xFF1A0E0A; // c2d(10,14,26,255)   dark navy background
pub const COL_PANEL: u32 = 0xFF3C2A1A; // c2d(26,42,60,255)   dark blue-gray panel
pub const COL_GOLD: u32 = 0xFF0B9EF5; // c2d(245,158,11,255) gold text
pub const COL_LIGHT: u32 = 0xFFDBD5D1; // c2d(209,213,219,255) light gray text
pub const COL_MED: u32 = 0xFF80726B; // c2d(107,114,128,255) medium gray text
pub const COL_SEL: u32 = 0xFF5C3A2A; // c2d(42,58,92,255)   selected-item background
pub const COL_DIM: u32 = 0x66231A33; // c2d(26,35,51,102)   semi-transparent dark
pub const COL_HIGHLIGHT: u32 = 0x330B9EF5; // c2d(245,158,11,51)  semi-transparent gold
pub const COL_CARD: u32 = 0x22231A22; // c2d(34,26,35,34)    card detail semi-transparent
pub const COL_CARD_OPAQUE: u32 = 0xFF231A22; // c2d(34,26,35,255) opaque card detail background
pub const COL_ABILITY: u32 = 0x33231A2A; // c2d(42,26,51,51)    ability queue semi-transparent
pub const COL_BLUE: u32 = 0xFFFF9E4A; // c2d(74,158,255,255) blue accent text
