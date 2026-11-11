fn main() {
    // The target, not the host: `#[cfg(windows)]` here would describe the
    // machine running the build script, and a check for Linux or macOS from a
    // Windows machine then tried to embed a Windows icon and failed.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winres::WindowsResource::new();
        // Windows takes only an .ico inside an executable, never a .png.
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            // A message, not a panic with a backtrace: the one way this fails
            // is `windres` missing from PATH, and the fix is in CLAUDE.md.
            eprintln!("error: could not embed the icon: {e}");
            eprintln!("       on the GNU toolchain, `windres` (from MinGW) must be on PATH;");
            eprintln!("       see CLAUDE.md, Build and test.");
            std::process::exit(1);
        }

        // On the GNU toolchain (x86_64-pc-windows-gnu), winres compiles the
        // icon into OUT_DIR/resource.o and packs it into a static library
        // linked with `-lresource`. That object holds only resource data, with
        // no symbol anything references, and `ld` pulls a member out of a
        // static library only to resolve a pending symbol - so it drops the
        // object and the icon vanishes from the .exe with no build error.
        // Linking the .o directly (outside an archive) sidesteps that
        // selection: an object passed straight to the linker is always kept.
        if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu") {
            let out_dir = std::env::var("OUT_DIR").expect("cargo sets OUT_DIR");
            println!("cargo:rustc-link-arg-bins={out_dir}/resource.o");
        }
    }

    // So the executable is rebuilt when the picture changes.
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");
}
