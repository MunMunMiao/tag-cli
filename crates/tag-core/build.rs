use std::env;
use std::path::PathBuf;

fn main() {
    // cargo:rustc-link-arg instructions emitted by taglib-rs/build.rs are not
    // propagated to dependent crates. tag-core links against the static TagLib
    // archives on Apple targets, so we repeat the required link arguments here
    // for tag-core's own test binaries.
    let taglib_out_dir = PathBuf::from(
        env::var("DEP_TAG_C_OUT_DIR").expect("DEP_TAG_C_OUT_DIR must be set by taglib-rs"),
    );
    let lib_dir = taglib_out_dir.join("lib");

    // Re-export the native library search path so the linker can resolve
    // transitive references when plain link-lib instructions are used.
    println!("cargo:rustc-link-search=native={}", lib_dir.display());

    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    if target_vendor == "apple" {
        let tag_c_path = lib_dir.join("libtag_c.a");
        let tag_path = lib_dir.join("libtag.a");
        println!(
            "cargo:rustc-link-arg=-Wl,-force_load,{}",
            tag_c_path.display()
        );
        println!(
            "cargo:rustc-link-arg=-Wl,-force_load,{}",
            tag_path.display()
        );
        println!("cargo:rustc-link-arg=-lz");
        println!("cargo:rustc-link-arg=-lc++");
    }
}
