
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        // embed_resource::compile() returns () in this crate version (not a
        // Result), so success/failure can't be branched on here directly —
        // any real failure will show up as its own compiler diagnostic
        // during this build script's compilation or as a linker error.
        embed_resource::compile("resource.rc", embed_resource::NONE);
        println!("cargo:warning=CHECKSUM_ICON_EMBED_OK");
    }
}
