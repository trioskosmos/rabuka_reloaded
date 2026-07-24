use std::path::Path;
use std::process::Command;

fn main() {
    let devkitpro = std::env::var("DEVKITPRO").unwrap_or_else(|_| "C:/devkitPro".to_string());
    let devkitppc =
        std::env::var("DEVKITPPC").unwrap_or_else(|_| format!("{}/devkitPPC", devkitpro));

    let gcc = format!("{}/bin/powerpc-eabi-gcc", devkitppc);
    let libogc_inc = format!("{}/libogc/include", devkitpro);

    let out_dir = std::env::var("OUT_DIR").unwrap();

    // 1. Compile entry.c
    let entry_o = format!("{}/entry.o", out_dir);
    let status = Command::new(&gcc)
        .args(&[
            "-mrvl",
            "-meabi",
            "-mhard-float",
            "-c",
            "entry.c",
            "-o",
            &entry_o,
            "-I",
            &libogc_inc,
        ])
        .status()
        .expect("failed to run powerpc-eabi-gcc");
    assert!(status.success(), "entry.c compilation failed");

    // 2. Generate Shift-JIS card name lookup table via Python
    let baked_dir = Path::new("../../platforms/psp/baked");
    if baked_dir.join("decks.json").exists() {
        let out_dir_fwd = out_dir.replace("\\", "/");
        let result = Command::new("python")
            .arg("gen_sjis.py")
            .arg(baked_dir.to_str().unwrap())
            .arg(&out_dir_fwd)
            .output();
        match result {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("cargo:warning=Python SJIS conversion failed: {}", stderr);
                } else {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    println!("cargo:warning={}", stdout.trim());
                }
            }
            Err(e) => {
                println!(
                    "cargo:warning=Could not run Python: {}. Install Python 3.",
                    e
                );
            }
        }
    }

    println!(
        "cargo:rustc-link-search=native={}",
        format!("{}/libogc/lib/wii", devkitpro)
    );
    println!("cargo:rustc-link-arg={}", &entry_o);
}
