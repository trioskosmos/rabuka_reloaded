use core::ffi::c_void;
use core::ptr;

extern "C" {
    fn timer_ms_gettime64() -> u64;
}

// Simple xorshift64 PRNG seeded from the KOS timer
fn kos_getrandom(buf: &mut [u8]) -> Result<(), getrandom::Error> {
    let seed = unsafe { timer_ms_gettime64() };
    let mut state = if seed == 0 { 1 } else { seed };
    for chunk in buf.chunks_mut(8) {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let bytes = state.to_le_bytes();
        let len = chunk.len().min(8);
        chunk.copy_from_slice(&bytes[..len]);
    }
    Ok(())
}
