use cudarc::driver::sys::CUDA_VERSION;

fn main() {
    println!("cargo::rustc-check-cfg=cfg(cuda_12050)");
    println!("cargo::rustc-check-cfg=cfg(cuda_12080)");

    if CUDA_VERSION >= 12050 {
        println!("cargo:rustc-cfg=cuda_12050");
    }
    if CUDA_VERSION >= 12080 {
        println!("cargo:rustc-cfg=cuda_12080");
    }

    // Capture the cubecl crate-graph HEAD SHA so the persistent PTX
    // cache key includes it. Without this, bumping our fork rev for
    // codegen-only changes (where cubecl-common's Cargo.toml version
    // is unchanged) leaves the cache reading stale PTX into the new
    // codegen pipeline. Falls back gracefully when built from
    // crates.io (no .git directory) or in CI sandboxes that strip git
    // history — the cache key just degrades to a fallback path layout.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .current_dir(&manifest_dir)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| std::env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=CUBECL_GIT_SHA={sha}");

    // Best-effort: re-run when the cubecl repo HEAD moves. Walks up
    // from the manifest dir to find a .git directory (handles
    // checkout-as-workspace-member layouts).
    let mut p = std::path::PathBuf::from(&manifest_dir);
    loop {
        let head = p.join(".git").join("HEAD");
        if head.exists() {
            println!("cargo:rerun-if-changed={}", head.display());
            break;
        }
        if !p.pop() {
            break;
        }
    }
}
