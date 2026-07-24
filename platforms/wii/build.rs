use std::path::Path;
use std::process::Command;

fn main() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let devkitpro = std::env::var("DEVKITPRO").unwrap_or_else(|_| "C:/devkitPro".to_string());
    let devkitppc =
        std::env::var("DEVKITPPC").unwrap_or_else(|_| format!("{}/devkitPPC", devkitpro));
    let gcc = format!("{}/bin/powerpc-eabi-gcc", devkitppc);
    let libogc_inc = format!("{}/libogc/include", devkitpro);
    let libogc_lib = format!("{}/libogc/lib/wii", devkitpro);
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let gen_py = Path::new(&manifest).join("gen_sjis.py");
    let baked = Path::new(&manifest).join("../../platforms/psp/baked");

    // Generate SJIS map header
    let out_fwd = out_dir.replace("\\", "/");
    let _ = Command::new("python")
        .args(&[
            gen_py.to_str().unwrap(),
            &baked.to_str().unwrap().replace("\\", "/"),
            &out_fwd,
        ])
        .output();

    // Verify sjis_map.h was created
    let hdr = Path::new(&out_dir).join("sjis_map.h");
    assert!(hdr.exists(), "sjis_map.h not generated at {:?}", hdr);

    // Compile C files
    let entry_o = format!("{}/entry.o", out_dir);
    cc(&gcc, &manifest, "entry.c", &entry_o, &libogc_inc, &[]);
    let display_o = format!("{}/display.o", out_dir);
    cc(
        &gcc,
        &manifest,
        "display.c",
        &display_o,
        &libogc_inc,
        &[&out_dir],
    );

    println!("cargo:rustc-link-search=native={}", libogc_lib);
    println!("cargo:rustc-link-arg={}", &entry_o);
    println!("cargo:rustc-link-arg={}", &display_o);
}

fn cc(gcc: &str, cwd: &str, src: &str, out: &str, inc: &str, extra: &[&str]) {
    let mut a: Vec<&str> = vec![
        "-mrvl",
        "-meabi",
        "-mhard-float",
        "-c",
        src,
        "-o",
        out,
        "-I",
        inc,
    ];
    for e in extra {
        a.push("-I");
        a.push(e);
    }
    let s = Command::new(gcc)
        .current_dir(cwd)
        .args(&a)
        .status()
        .unwrap();
    assert!(s.success(), "{} failed", src);
}
