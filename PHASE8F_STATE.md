# Phase 8f — zenforks-cubecl execution state

Tracking doc for the imazen/zenforks-cubecl crates.io rename work.
Companion to `/home/lilith/work/zen/zenmetrics--phase8f/` workspace.

## Base

- Upstream: `tracel-ai/cubecl`
- Tag: `v0.10.0` (`7cf203735e095e640a2c03b2400d0faa03196bb4`, released 2026-05-11)
- Repo: <https://github.com/imazen/zenforks-cubecl>

## User decisions (locked 2026-05-27)

1. Minimal scope — fork only what needs patches + transitive dep graph
2. 0.10.0 = vanilla rename + pinned-upload patch (production-proven)
3. 0.10.1 patches gated on smoke-build of all patches together
4. `[lib] name = "cubecl_*"` shim — rename `[package] name` only

## Patches (per-patch scope)

| Patch | Source doc | Files touched |
|---|---|---|
| pinned-upload | `PINNED_UPLOAD_UPSTREAM_PR.md` | `crates/cubecl-runtime/src/client.rs` + new `examples/upload_bench/` |
| PTX cache widening | `CUBECL_PERSISTENT_PTX_CACHE_PATCH.md` | `crates/cubecl-cuda/src/compute/context.rs` + new `crates/cubecl-cuda/build.rs` |
| Metal Atomic fix | `CUBECL_METAL_ATOMIC_FIX.md` | `crates/cubecl-wgpu/src/backend/metal.rs` + `crates/cubecl-wgpu/src/compiler/wgsl/instructions.rs` + `crates/cubecl-wgpu/src/compiler/wgsl/base.rs` |

## Dep graph trace at v0.10.0

Forward closure of patched crates (cubecl-runtime, cubecl-cuda, cubecl-wgpu):
all of these need rename so they don't collide with upstream on crates.io.

### Renamed to `zenforks-cubecl-*` (11 crates)

| Crate | Reason | Patched? |
|---|---|---|
| `cubecl-runtime` | pinned-upload patch | YES |
| `cubecl-cuda` | PTX cache patch | YES |
| `cubecl-wgpu` | Metal atomic patch | YES |
| `cubecl-core` | depends on cubecl-runtime | no |
| `cubecl-opt` | depends on cubecl-core | no |
| `cubecl-cpp` | depends on cubecl-runtime + cubecl-core + cubecl-opt | no |
| `cubecl-cpu` | depends on cubecl-runtime + cubecl-core + cubecl-opt + cubecl-std | no |
| `cubecl-hip` | depends on cubecl-runtime + cubecl-cpp + cubecl-core | no |
| `cubecl-spirv` | depends on cubecl-runtime + cubecl-core + cubecl-opt | no |
| `cubecl-std` | depends on cubecl-runtime + cubecl-core | no |
| `cubecl` | umbrella, depends on all the above | no |

### Stays upstream (5 crates, no rename)

| Crate | Why no rename |
|---|---|
| `cubecl-common` | leaf — no cubecl-* deps |
| `cubecl-ir` | only depends on cubecl-common + cubecl-macros-internal (both leaves) |
| `cubecl-macros` | only depends on cubecl-common |
| `cubecl-macros-internal` | leaf |
| `cubecl-zspace` | leaf |

## [lib] name shim approach

For each renamed crate's Cargo.toml:
```toml
[package]
name = "zenforks-cubecl-runtime"   # ← renamed (was: cubecl-runtime)
version = "0.10.0"

[lib]
name = "cubecl_runtime"             # ← KEEP — so `use cubecl_runtime::*` still works
```

For inter-fork deps (in renamed crates):
```toml
[dependencies]
cubecl-runtime = { package = "zenforks-cubecl-runtime", path = "../cubecl-runtime", version = "0.10.0" }
```

The `cubecl_*` lib names stay unchanged → no source-code rewrites in
zenmetrics or in the renamed cubecl crates themselves.

## Stages

- [x] Stage 1 — create imazen/zenforks-cubecl repo. Cloned upstream v0.10.0 to `/home/lilith/work/zenforks-cubecl-work`, set up remotes (upstream=tracel-ai, origin=imazen/zenforks-cubecl), branched `main` from v0.10.0.
- [x] Stage 2 — minimal-scope rename with [lib] name shim. `cargo check --workspace --no-default-features` green in 45s; `cargo build --workspace --no-default-features --lib` green in 18s. Commit `92c87709` on imazen/zenforks-cubecl main.
- [x] Stage 3 — apply pinned-upload patch. Patch from de2f9857 (one-commit cherry on top of v0.10.0) applied clean via `git apply`. Smoke build green. Test results:
   - `zenforks-cubecl-runtime` (the patched crate): 63/63 lib tests pass
   - 7 other renamed crates with tests pass individually (-p ... --lib)
   - `--workspace --lib` fails on cuda/wgpu/cpu/hip due to UPSTREAM v0.10.0 bug:
     `cubecl-core/src/runtime_tests/binary.rs:56` uses
     `FastMath::all().difference(FastMath::NotNaN)` which doesn't compile
     against the resolved `enumset 1.1.13` (needs `.into()`). The same
     failure reproduces on a stock upstream v0.10.0 checkout
     (`~/.cargo/git/checkouts/cubecl-.../7cf20373`). This is **not** caused
     by our rename or patch — it's a pre-existing upstream bug in the
     `export_tests`-gated runtime-tests module, and `export_tests` is
     never enabled in published downstream consumption.
- [ ] Stage 4 — publish 0.10.0 to crates.io (dep-order). Tag v0.10.0 + GH release.
- [ ] Stage 5 — apply PTX cache + Metal atomic patches. Smoke build + test.
- [ ] Stage 6 — publish 0.10.1. Tag + GH release.
- [ ] Stage 7 — workspace switch in zenmetrics--phase8f (Cargo.toml package-alias only).
- [ ] Stage 8 — parity sweep verification.
- [ ] Stage 9 — documentation: ZENFORKS_CUBECL_STRATEGY.md.
