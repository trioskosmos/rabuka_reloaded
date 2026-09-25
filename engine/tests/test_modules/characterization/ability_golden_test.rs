//! C2 shape guard for abilities.json.
//! If this fails, you changed the walker/parser and must deliberately re-golden.
use std::path::Path;

#[test]
fn abilities_shape_matches() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../cards/abilities.json");
    let bytes = std::fs::read(&p).expect("abilities.json readable");
    let val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let obj = val.as_object().expect("abilities.json is object");
    // Golden: top-level is 10 keys (walker output shape); 936 unique abilities is derived count in inventory
    assert_eq!(obj.len(), 10, "abilities.json top-level key count changed — re-golden deliberately; was 10");
}
