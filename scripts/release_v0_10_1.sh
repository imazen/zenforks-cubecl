#!/usr/bin/env bash
# Tag v0.10.1 and create GitHub release once all 11 zenforks-cubecl-*
# crates have published at 0.10.1.

set -eu
cd "$(dirname "$0")/.."

VERSION=v0.10.1
TAG_TARGET="main"  # The commit on main with all 0.10.1 patches + version bump

EXPECTED=(
    zenforks-cubecl-runtime
    zenforks-cubecl-core
    zenforks-cubecl-opt
    zenforks-cubecl-std
    zenforks-cubecl-cpp
    zenforks-cubecl-cuda
    zenforks-cubecl-hip
    zenforks-cubecl-spirv
    zenforks-cubecl-wgpu
    zenforks-cubecl-cpu
    zenforks-cubecl
)

echo "Verifying all 11 crates are on crates.io at 0.10.1..."
for c in "${EXPECTED[@]}"; do
    ver=$(curl -sf -A phase8f "https://crates.io/api/v1/crates/$c" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['crate']['newest_version'])" 2>/dev/null || echo "MISSING")
    if [[ "$ver" != "0.10.1" ]]; then
        echo "    !!! $c is at $ver, NOT 0.10.1 — aborting tag/release"
        exit 1
    fi
    printf '    OK: %-30s %s\n' "$c" "$ver"
done

echo "Creating tag $VERSION..."
git tag -a "$VERSION" -m "$(cat <<EOF
zenforks-cubecl-* v0.10.1 — adds PTX cache widening + Metal atomic capability honesty

Builds on 0.10.0 (vanilla rename + pinned-upload). Two new patches:

1. **Persistent PTX cache widening** (cubecl-cuda):
   Adds three axes to the cache directory layout:
   - CUBECL_GIT_SHA (captured at build time): invalidates on any
     fork-source change, not just upstream cubecl-common version bumps.
   - sm_arch (CudaArchitecture): arch-specific PTX safety.
   - driver_version (cuDriverGetVersion): per-driver JIT safety.

   Resulting layout:
     <root>/cuda/<cubecl-common-ver>/<git-sha>/<sm_arch>/<driver_ver>/ptx.json.log

   Eliminates the "fresh-process cold start = ~18s NVRTC recompile
   because cache key was too narrow" failure mode.

2. **Metal Atomic<f32> capability honesty** (cubecl-wgpu):
   Drops AtomicUsage::Add from the Metal backend's f32 atomic
   declaration. naga's MSL backend doesn't emit
   atomic_fetch_add_explicit for f32, so the previous declaration
   caused Atomic<f32>::fetch_add callers to silently no-op (every
   reduction returned 0.0).

   After this patch, callers get a hard error at construct time
   ("unsupported atomic operation on this backend") instead of
   silent wrong scores at runtime.

   Note: this is "Part A" only (capability honesty). The "Part B"
   CAS-loop WGSL codegen that would give Metal users a working
   f32-atomic-add is deferred to a follow-on; downstream zenmetrics
   workarounds remain the production correctness fix.

All renamed crates bumped to 0.10.1; non-renamed crates
(cubecl-common, cubecl-ir, cubecl-macros, cubecl-macros-internal,
cubecl-zspace) continue to come from upstream at 0.10.0.

See ZENFORKS_README.md and zenmetrics' crates/zenmetrics-api/docs/
ZENFORKS_CUBECL_STRATEGY.md for the full maintenance playbook.
EOF
)"

git push origin "$VERSION"

gh release create "$VERSION" \
    --title "$VERSION — PTX cache widening + Metal atomic capability honesty" \
    --notes "$(cat <<'EOF'
Patch release on top of 0.10.0.

## What's new in 0.10.1

### 1. Persistent PTX cache widening (`zenforks-cubecl-cuda`)

The existing disk-persistent PTX cache key was too narrow for our
usage. We add three axes:

- `CUBECL_GIT_SHA` — captured at build time. Invalidates on any
  zenforks-cubecl-cuda source change, not just upstream
  `cubecl-common`'s `Cargo.toml` version field bumps.
- `sm_arch` — NVRTC compiles arch-specific PTX. Serving sm_70 PTX
  to an sm_80 device is a correctness bug; appending the arch makes
  safety structural.
- `driver_version` — different driver versions JIT the same PTX
  into different SASS. Per-driver safety.

Resulting on-disk layout:

```
<root>/cuda/<cubecl-common-ver>/<git-sha>/<sm_arch>/<driver_ver>/ptx.json.log
```

Eliminates the "fresh-process cold start = ~18s NVRTC re-compile
because the cache key was too narrow" failure mode that hit
zenmetrics' fleet workers under cubecl rev bumps.

### 2. Metal `Atomic<f32>` capability honesty (`zenforks-cubecl-wgpu`)

cubecl-wgpu's Metal backend was declaring `Atomic<f32> + Add`
capable, but naga's MSL backend doesn't emit
`atomic_fetch_add_explicit` for f32 — so the WGSL `atomicAdd<f32>`
got silently dropped during translation, leaving every reduction
returning its default `0.0` value. Symptom: every `*-gpu` metric's
score collapsed to a fall-through constant on Metal.

This patch drops `AtomicUsage::Add` from Metal's f32 atomic
registration. Callers requesting `Atomic<f32>::fetch_add` now fail
at construct time with an actionable error instead of returning
wrong numbers at runtime.

**Not yet:** Part B (CAS-loop WGSL codegen lowering for f32-atomic-add)
which would let Metal users actually get correct `Atomic<f32>::fetch_add`.
That requires a wider change to cubecl-wgpu's WGSL Type system and
binding layer; deferred to a follow-on release. The downstream
[zenmetrics](https://github.com/imazen/zenmetrics) workarounds —
flipping `fast-reduction` default off on `butteraugli-gpu` and
`dssim-gpu`, Metal-reject on `cvvdp-gpu` — remain the production
correctness fix on Metal.

## What's unchanged

- 0.10.0's pinned-upload patch on `zenforks-cubecl-runtime` is still here.
- All 11 renamed crates have the same `package` -> `[lib]` shim
  ([lib] name is `cubecl_*`, package name is `zenforks-cubecl-*`),
  so consumer source code keeps reading `use cubecl_runtime::*;`
  unchanged.

## Consumer pin convention

```toml
[dependencies]
cubecl-runtime = { package = "zenforks-cubecl-runtime", version = "0.10.1" }
cubecl-cuda    = { package = "zenforks-cubecl-cuda",    version = "0.10.1" }
cubecl-wgpu    = { package = "zenforks-cubecl-wgpu",    version = "0.10.1" }
# Non-renamed crates stay on upstream:
cubecl-common  = "0.10.0"
cubecl-ir      = "0.10.0"
```
EOF
)" \
    --verify-tag

echo "+++ v0.10.1 tag + release created +++"
