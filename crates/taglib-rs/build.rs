use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let taglib_dir = manifest_dir.join("vendor/taglib");

    println!(
        "cargo:rerun-if-changed={}",
        taglib_dir.join("CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        taglib_dir.join("bindings/c/CMakeLists.txt").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        taglib_dir.join("bindings/c/tag_c.h").display()
    );

    if !taglib_dir.join("CMakeLists.txt").exists() {
        panic!(
            "未找到 {}。\n请先初始化子模块：git submodule update --init --recursive",
            taglib_dir.join("CMakeLists.txt").display()
        );
    }

    let dst = cmake::Config::new(&taglib_dir)
        .define("BUILD_TESTING", "OFF")
        .define("BUILD_EXAMPLES", "OFF")
        .define("BUILD_BINDINGS", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .build();

    // Expose this crate's OUT_DIR to downstream build scripts so they can
    // locate the static TagLib archives (cargo:rustc-link-arg is not propagated
    // across crates).
    println!("cargo::metadata=OUT_DIR={}", env::var("OUT_DIR").unwrap());

    let lib_dir = dst.join("lib");
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    let lib64_dir = dst.join("lib64");
    if lib64_dir.exists() {
        println!("cargo:rustc-link-search=native={}", lib64_dir.display());
    }

    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    if target_vendor == "apple" {
        // Force-load the static archives on Apple targets. This avoids link-order
        // issues that cause `cargo test` binaries to fail with undefined TagLib
        // symbols, and guarantees C++ exception/zlib symbols are resolved.
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
    } else {
        println!("cargo:rustc-link-lib=static=tag_c");
        println!("cargo:rustc-link-lib=static=tag");

        if is_unix_target() {
            if is_musl_target() {
                println!("cargo:rustc-link-search=native=/usr/lib");
                println!("cargo:rustc-link-lib=static=z");
            } else {
                println!("cargo:rustc-link-lib=z");
            }
        }

        link_cpp_stdlib_if_needed();
    }
}

fn is_unix_target() -> bool {
    env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix")
}

fn is_musl_target() -> bool {
    env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("musl")
}

fn link_cpp_stdlib_if_needed() {
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        return;
    }

    let target_vendor = env::var("CARGO_CFG_TARGET_VENDOR").unwrap_or_default();
    let needed = if target_vendor == "apple" {
        "c++"
    } else {
        "stdc++"
    };
    if is_musl_target() && needed == "stdc++" {
        println!("cargo:rustc-link-lib=static={needed}");
    } else {
        println!("cargo:rustc-link-lib={needed}");
    }
}
