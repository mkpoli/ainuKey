fn main() {
    println!("cargo:rerun-if-changed=resources.rc");
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/mode_kana.ico");
    println!("cargo:rerun-if-changed=assets/mode_latn.ico");

    // Embed the .ico resources into the DLL. embed-resource only does real work
    // on the MSVC target with rc.exe available; on a `cargo check` (no link, no
    // rc) it is effectively a no-op. Made non-fatal: a missing Windows SDK /
    // rc.exe must never block the build, since the profile icon is cosmetic and
    // an absent icon is harmless (the profile still registers and works).
    match embed_resource::compile("resources.rc", embed_resource::NONE).manifest_optional() {
        Ok(()) => {}
        Err(e) => println!(
            "cargo:warning=ainuKey: icon resource embed skipped ({e}); the profile icon will be absent"
        ),
    }

    // Belt-and-suspenders DLL export table for MSVC: some toolchains do not honor
    // `#[no_mangle]` for cdylib exports, so point the linker at the .def file.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        let def = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("ainuKey.def");
        println!("cargo:rustc-cdylib-link-arg=/DEF:{}", def.display());
    }
}
