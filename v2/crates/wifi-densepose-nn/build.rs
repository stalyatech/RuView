// StalyaTech RuView Phase 2: build script for the optional `cvitek` feature.
//
// When `--features cvitek` is requested we emit linker hints so the resulting
// binary dynamically links against the CviTek TPU userspace runtime
// (`libcviruntime.so`).  All other build configurations are a pure no-op so
// upstream RuView crates that disable this feature do not need anything in
// their environment.

fn main() {
    // Re-run only if relevant inputs change.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=CVITEK_SDK_DIR");
    println!("cargo:rerun-if-env-changed=CVITEK_LIB_DIR");
    println!("cargo:rerun-if-env-changed=CVITEK_INCLUDE_DIR");

    // The feature gate: CARGO_FEATURE_<NAME_UPPER>=1 when active.
    let cvitek_enabled = std::env::var("CARGO_FEATURE_CVITEK").is_ok();
    if !cvitek_enabled {
        return;
    }

    // Resolve the search path that holds libcviruntime.so.  Two ways:
    //   1. CVITEK_LIB_DIR — explicit override, must contain libcviruntime.so
    //   2. CVITEK_SDK_DIR — points at the SDK root; we append /lib
    // If neither is set we still emit the link directive so the host or
    // sysroot default search path can resolve it (target rootfs already
    // ships the lib under /mnt/system/lib, but cross-compile needs an
    // explicit -L).
    let lib_dir = std::env::var("CVITEK_LIB_DIR")
        .ok()
        .or_else(|| std::env::var("CVITEK_SDK_DIR").ok().map(|s| format!("{s}/lib")));

    if let Some(dir) = lib_dir {
        // `=native=` keeps the dir out of the embedded runtime path lookup
        // (we prefer the target rootfs path at runtime, set by the caller
        // via LD_LIBRARY_PATH on device).
        println!("cargo:rustc-link-search=native={dir}");
    }

    // Dynamic link only — no static fallback.  libcviruntime.so transitively
    // pulls libcvikernel.so + libstdc++ which we assume the rootfs ships.
    println!("cargo:rustc-link-lib=dylib=cviruntime");
}
