fn main() {
    let devkitpro = std::env::var("DEVKITPRO").unwrap_or_else(|_| "C:/devkitPro".to_string());
    let devkitppc =
        std::env::var("DEVKITPPC").unwrap_or_else(|_| format!("{}/devkitPPC", devkitpro));
    let gcc = format!("{}/bin/powerpc-eabi-gcc", devkitppc);
    let inc = format!("{}/libogc/include", devkitpro);
    let lib = format!("{}/libogc/lib/wii", devkitpro);
    let out = std::env::var("OUT_DIR").unwrap();

    let eo = format!("{}/entry.o", out);
    let s = std::process::Command::new(&gcc)
        .args(&[
            "-mrvl",
            "-meabi",
            "-mhard-float",
            "-c",
            "entry.c",
            "-o",
            &eo,
            "-I",
            &inc,
        ])
        .status()
        .unwrap();
    assert!(s.success(), "entry.c failed");

    println!("cargo:rustc-link-search=native={}", lib);
    println!("cargo:rustc-link-arg={}", &eo);
}
