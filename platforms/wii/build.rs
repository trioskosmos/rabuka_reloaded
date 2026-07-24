fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();

    // Generate SJIS map if baked data exists
    let baked = std::path::Path::new(&manifest).join("../../platforms/psp/baked");
    if baked.join("decks.json").exists() {
        let out_fwd = out_dir.replace("\\", "/");
        std::process::Command::new("python")
            .args(&[
                "gen_sjis.py",
                &baked.to_str().unwrap().replace("\\", "/"),
                &out_fwd,
            ])
            .output()
            .ok();
    }
}
