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

    provide_ngrams_table();
}

/// Provide the n-gram suggestion table to `OUT_DIR` for `include_bytes!`.
///
/// The committed `data/ngrams.bin` (aggregate counts derived from `ainu-corpora`,
/// cleared for distribution — see `data/README.md`) is copied to `OUT_DIR`. If it
/// is ever absent (e.g. deleted locally), an EMPTY table is embedded instead so
/// the crate still builds — just with suggestions disabled.
fn provide_ngrams_table() {
    use std::path::Path;
    println!("cargo:rerun-if-changed=data/ngrams.bin");
    let out = std::env::var("OUT_DIR").expect("OUT_DIR");
    let dst = Path::new(&out).join("ngrams.bin");
    let local = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/ngrams.bin");
    if local.exists() {
        std::fs::copy(&local, &dst).expect("copy ngrams.bin to OUT_DIR");
    } else {
        // Empty v2 table: magic, version=2, 0 unigrams, 0 bigram-ctx, 0 trigram-ctx.
        let mut empty = Vec::with_capacity(20);
        empty.extend(b"AKNG");
        empty.extend(2u32.to_le_bytes());
        empty.extend(0u32.to_le_bytes());
        empty.extend(0u32.to_le_bytes());
        empty.extend(0u32.to_le_bytes());
        std::fs::write(&dst, empty).expect("write empty ngrams.bin");
    }
}
