#!/usr/bin/env bash
# Tag v0.10.0 and create GitHub release once all 11 zenforks-cubecl-*
# crates have published at 0.10.0.

set -eu
cd "$(dirname "$0")/.."

VERSION=zenforks-v0.10.0
TAG_TARGET="d45a3868"  # The commit that all 0.10.0 crates were published from
# Tag name uses `zenforks-` prefix because the upstream `v0.10.0` tag
# from tracel-ai/cubecl is also present in this repo (we cloned upstream
# including all tags). This keeps our fork-tag namespace distinct so
# `git tag` shows both `v0.10.0` (upstream) and `zenforks-v0.10.0` (ours).

# Verify all 11 crates exist on crates.io at 0.10.0
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

echo "Verifying all 11 crates are on crates.io at 0.10.0..."
for c in "${EXPECTED[@]}"; do
    ver=$(curl -sf -A phase8f "https://crates.io/api/v1/crates/$c" 2>/dev/null | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['crate']['newest_version'])" 2>/dev/null || echo "MISSING")
    if [[ "$ver" != "0.10.0" && "$ver" != "0.10.1" ]]; then
        echo "    !!! $c is at $ver, NOT 0.10.0 — aborting tag/release"
        exit 1
    fi
    printf '    OK: %-30s %s\n' "$c" "$ver"
done

# Tag at d45a3868 (chore(metadata) — the commit all 0.10.0 publishes were from).
echo "Creating tag $VERSION at $TAG_TARGET..."
git tag -a "$VERSION" "$TAG_TARGET" -m "$(cat <<EOF
zenforks-cubecl-* v0.10.0 — vanilla rename + pinned-upload

The 11 renamed crates at version 0.10.0 publish a clean rename of
upstream tracel-ai/cubecl v0.10.0 (7cf20373) with one patch:

- pinned-host-buffer fast path for create_from_slice uploads
  (cubecl-runtime). ~4x HtoD speedup on CUDA workloads via direct
  DMA from pinned host memory at 12-25 GB/s on PCIe 4.0.
  Production-proven via lilith/cubecl@de2f9857 since 2026-05-10.

Renamed crates (all at 0.10.0 on crates.io):

  zenforks-cubecl              (umbrella)
  zenforks-cubecl-runtime      (pinned-upload patch)
  zenforks-cubecl-core
  zenforks-cubecl-opt
  zenforks-cubecl-cpp
  zenforks-cubecl-cpu
  zenforks-cubecl-cuda
  zenforks-cubecl-hip
  zenforks-cubecl-spirv
  zenforks-cubecl-std
  zenforks-cubecl-wgpu

The 5 non-renamed leaf crates (cubecl-common, cubecl-ir,
cubecl-macros, cubecl-macros-internal, cubecl-zspace) come from
upstream tracel-ai/cubecl's crates.io publication unchanged.

Consumer pin convention (no source-code changes needed thanks to the
[lib] name = "cubecl_*" shim):

  cubecl-runtime = { package = "zenforks-cubecl-runtime", version = "0.10.0" }

See ZENFORKS_README.md and the zenmetrics docs at
crates/zenmetrics-api/docs/ZENFORKS_CUBECL_STRATEGY.md for the full
maintenance playbook.
EOF
)"

git push origin "$VERSION"

gh release create "$VERSION" \
    --target "$TAG_TARGET" \
    --title "$VERSION — vanilla rename + pinned-upload" \
    --notes "$(cat <<'EOF'
First release of the `zenforks-cubecl-*` family on crates.io.

## What this is

A maintained fork of [tracel-ai/cubecl](https://github.com/tracel-ai/cubecl)
v0.10.0, with 11 of its 16 crates renamed and published to crates.io
under the `zenforks-cubecl-*` namespace. The renamed crates carry a
small number of internal-use patches that downstream `imazen`
projects (zenmetrics, six `*-gpu` perceptual-metric crates) depend on
while waiting for upstream PRs to merge.

## What's renamed vs not

**Renamed in 0.10.0** (published from this tag):
`zenforks-cubecl`, `zenforks-cubecl-runtime`, `zenforks-cubecl-core`,
`zenforks-cubecl-cuda`, `zenforks-cubecl-wgpu`, `zenforks-cubecl-cpu`,
`zenforks-cubecl-cpp`, `zenforks-cubecl-hip`, `zenforks-cubecl-spirv`,
`zenforks-cubecl-std`, `zenforks-cubecl-opt`.

**Stays upstream** (consume directly from `tracel-ai/cubecl`'s
crates.io publication at 0.10.0): `cubecl-common`, `cubecl-ir`,
`cubecl-macros`, `cubecl-macros-internal`, `cubecl-zspace`.

These are leaves of the dep graph; no transitive dep on a patched
crate, so no need to rename.

## What patches ship in 0.10.0

Only the **pinned-host-buffer fast path** for
`ComputeClient::create_from_slice` and friends, in `cubecl-runtime`.
~4× HtoD speedup on CUDA workloads via direct DMA from pinned host
memory at 12-25 GB/s on PCIe 4.0 (vs ~5-6 GB/s pageable bounce).
Drafted as upstream PR
[tracel-ai/cubecl#1334](https://github.com/tracel-ai/cubecl/pull/1334).

The PTX cache widening and Metal Atomic<f32> capability honesty
patches are coming in 0.10.1.

## Consumer pin convention

In your `Cargo.toml`, alias via the `package` field:

```toml
[dependencies]
cubecl-runtime = { package = "zenforks-cubecl-runtime", version = "0.10.0" }
cubecl-cuda    = { package = "zenforks-cubecl-cuda",    version = "0.10.0" }
# ...etc
# Non-renamed crates stay on upstream:
cubecl-common  = "0.10.0"
```

Then in source code, `use cubecl_runtime::*;` resolves to our
package because we keep `[lib] name = "cubecl_runtime"` unchanged.
No source rewrites needed.

## Acknowledgement

Built on the great work of the upstream
[tracel-ai/cubecl](https://github.com/tracel-ai/cubecl) maintainers.
This fork exists to ship downstream patches without waiting on
upstream review cycles, not to replace upstream.
EOF
)" \
    --verify-tag

echo "+++ v0.10.0 tag + release created +++"
