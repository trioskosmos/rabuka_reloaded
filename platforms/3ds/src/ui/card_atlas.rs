// CardAtlas — texture atlas loader mapping card_no -> (atlas_filename, index).

use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use rabuka_engine::card::Card;

#[cfg(feature = "3ds")]
#[derive(Clone)]
pub struct CardAtlas {
    /// Map card_no -> (atlas_filename, index)
    map: HashMap<String, (String, usize)>,
}

#[cfg(feature = "3ds")]
impl CardAtlas {
    pub fn load() -> Self {
        let path = Path::new("romfs:/cards_manifest.json");
        let mut f = match File::open(path) {
            Ok(f) => f,
            Err(_) => {
                return CardAtlas {
                    map: HashMap::new(),
                }
            }
        };
        let mut s = String::new();
        if f.read_to_string(&mut s).is_err() {
            return CardAtlas {
                map: HashMap::new(),
            };
        }
        let raw: HashMap<String, serde_json::Value> = match serde_json::from_str(&s) {
            Ok(m) => m,
            Err(_) => {
                return CardAtlas {
                    map: HashMap::new(),
                }
            }
        };
        let map = raw
            .into_iter()
            .filter_map(|(k, v)| {
                let atlas = v.get("atlas")?.as_str()?.to_string();
                let index = v.get("index")?.as_u64()? as usize;
                Some((k, (atlas, index)))
            })
            .collect();
        CardAtlas { map }
    }

    pub fn lookup(&self, card_no: &str) -> Option<&(String, usize)> {
        self.map.get(card_no)
    }

    /// Build sorted card list from loaded Card database (matches cards.json order).
    /// Build sorted indices into the cards slice by normalized card_no.
    /// Returns just Vec<usize> (18KB) instead of cloning all card strings.
    /// Temporarily allocates normalized strings for sorting, then drops them.
    pub fn build_qr_sorted(cards: &[Card]) -> Option<Vec<usize>> {
        let n = cards.len();
        let mut pairs: Vec<(String, usize)> = Vec::new();
        pairs.try_reserve(n).ok()?;
        for (i, c) in cards.iter().enumerate() {
            let norm = c
                .card_no
                .replace('\u{FF0B}', "+")
                .replace('\u{FF0D}', "-")
                .replace('\u{30FC}', "-")
                .to_uppercase();
            pairs.push((norm, i));
        }
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        let indices: Vec<usize> = pairs.into_iter().map(|(_, idx)| idx).collect();
        Some(indices)
    }

    /// Decode binary QR data: [count+1] [idx_hi+1 idx_lo+1 qty+1] ...
    /// Uses sorted indices to look up card_no from the original cards slice
    /// instead of cloning card_no strings into the sorted list.
    pub fn decode_qr_binary(
        sorted_indices: &[usize],
        cards: &[Card],
        data: &[u8],
    ) -> Option<Vec<String>> {
        if data.is_empty() {
            return None;
        }
        let count = (data[0] as usize).wrapping_sub(1);
        if count == 0 || data.len() < 1 + count * 3 {
            return None;
        }
        let mut result = Vec::with_capacity(count);
        for i in 0..count {
            let base = 1 + i * 3;
            let idx = (((data[base] as usize).wrapping_sub(1)) << 8)
                | ((data[base + 1] as usize).wrapping_sub(1));
            let qty = data[base + 2].wrapping_sub(1).max(1) as usize;
            let card_idx = *sorted_indices.get(idx)?;
            let card_no = &cards.get(card_idx)?.card_no;
            for _ in 0..qty {
                result.push(card_no.to_string());
            }
        }
        Some(result)
    }
}
