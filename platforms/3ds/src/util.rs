// Small utilities: heart helpers, base64, yielding reader, misc.

#[cfg(feature = "3ds")]
use std::io::Read;

use rabuka_engine::card::HeartColor;
#[cfg(feature = "3ds")]
use rabuka_engine::game_setup;

#[cfg(feature = "3ds")]
use crate::ffi::_3ds_main_loop;
#[cfg(feature = "3ds")]
use crate::i18n::Lang;
#[cfg(feature = "3ds")]
use crate::lang::current_lang;
use crate::ui::text::heart_label_to_icon;

pub const TICK_HZ: u64 = 268_120_000;

/// Map HeartColor to index 0-6 (skip BAll/Draw/Score). Returns None for non-color hearts.
pub fn heart_color_index(color: &HeartColor) -> Option<usize> {
    match color {
        HeartColor::BAll | HeartColor::Draw | HeartColor::Score => None,
        _ => Some(color.index()),
    }
}

/// Format need hearts with text icons matching top screen format.
pub fn format_need_hearts_icons(hearts: &[u32]) -> String {
    let mut parts = Vec::new();
    for (i, &count) in hearts.iter().enumerate() {
        if count > 0 {
            let label = format!("h{:02}{}", i, count);
            parts.push(heart_label_to_icon(&label));
        }
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!("{{{{icon_heart_06.png|NEED}}}} {}", parts.join(" "))
    }
}

/// Translate stage area labels for display.
#[cfg(feature = "3ds")]
pub fn tl_area(area: &str) -> &str {
    if current_lang() == Lang::Japanese {
        match area {
            "left" => "左",
            "center" => "センター",
            "right" => "右",
            other => other,
        }
    } else {
        match area {
            "left" => "Left",
            "center" => "Center",
            "right" => "Right",
            other => other,
        }
    }
}

/// Reader wrapper that calls aptMainLoop() every `threshold` bytes without
/// any GPU buffer operations. Keeps the 3DS OS alive during long deserialization
/// without the overhead/cost of _3ds_keep_alive().
#[cfg(feature = "3ds")]
pub struct YieldReader<R> {
    pub inner: R,
    pub threshold: usize,
    pub counter: usize,
}

#[cfg(feature = "3ds")]
impl<R: Read> Read for YieldReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.inner.read(buf)?;
        self.counter += n;
        if self.counter >= self.threshold {
            self.counter = 0;
            if unsafe { _3ds_main_loop() } == 0 {
                // App should exit; return empty read to signal EOF
                return Ok(0);
            }
        }
        Ok(n)
    }
}

#[cfg(feature = "3ds")]
pub fn cn_or_empty(act: &game_setup::Action) -> String {
    act.parameters
        .as_ref()
        .and_then(|p| p.card_no.clone())
        .unwrap_or_default()
}

#[cfg(feature = "3ds")]
pub fn ticks_to_ms(ticks: u64) -> f64 {
    (ticks as f64) / (TICK_HZ as f64) * 1000.0
}

/// Check if a string looks like base64 (QR binary format).
pub fn looks_like_b64(s: &str) -> bool {
    if s.len() < 4 || s.len() > 3000 {
        return false;
    }
    // Must be all ASCII base64 chars, no spaces/newlines
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
}

/// Minimal base64 decoder (no_std friendly).
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    const TABLE: [i8; 128] = [
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
        -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, 62, -1, -1,
        -1, 63, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1, -1, -1, 0, 1, 2, 3, 4,
        5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, -1, -1, -1,
        -1, -1, -1, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45,
        46, 47, 48, 49, 50, 51, -1, -1, -1, -1, -1,
    ];
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits: i32 = 0;
    for &b in s.as_bytes() {
        if b == b'=' {
            break;
        }
        let val = TABLE.get(b as usize)?;
        if *val < 0 {
            return None;
        }
        buf = (buf << 6) | (*val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_decode_basic() {
        assert_eq!(base64_decode("aGVsbG8="), Some(b"hello".to_vec()));
        // Missing padding decodes too
        assert_eq!(base64_decode("aGVsbG8"), Some(b"hello".to_vec()));
        // Invalid input
        assert_eq!(base64_decode("!!!"), None);
        assert_eq!(base64_decode("aGVs#G8="), None);
    }

    #[test]
    fn looks_like_b64_detects() {
        assert!(looks_like_b64("aGVsbG8="));
        assert!(looks_like_b64("AAAA"));
        assert!(!looks_like_b64("ab c"));
        assert!(!looks_like_b64("ab"));
        assert!(!looks_like_b64(""));
    }

    #[test]
    fn format_need_hearts_icons_builds() {
        assert_eq!(
            format_need_hearts_icons(&[0, 2, 0, 1]),
            "{{icon_heart_06.png|NEED}} {{heart_01.png|h01}} 2 {{heart_03.png|h03}} 1"
        );
        assert_eq!(format_need_hearts_icons(&[0; 8]), "");
    }
}
