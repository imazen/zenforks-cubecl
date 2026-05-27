# `zenforks-cubecl-*` — Imazen-maintained rename of CubeCL

This repository is the source tree for the `zenforks-cubecl-*` family of
crates published to [crates.io](https://crates.io). It is a fork of the
upstream [tracel-ai/cubecl](https://github.com/tracel-ai/cubecl) v0.10.0,
with the following crates renamed (package name only — `[lib] name`
stays as the upstream `cubecl_*` so source-code paths like
`use cubecl_runtime::*` continue to resolve unchanged):

| Renamed crate (crates.io) | Upstream equivalent |
|---|---|
| `zenforks-cubecl-runtime` | `cubecl-runtime` |
| `zenforks-cubecl-cuda`    | `cubecl-cuda` |
| `zenforks-cubecl-wgpu`    | `cubecl-wgpu` |
| `zenforks-cubecl-core`    | `cubecl-core` |
| `zenforks-cubecl-opt`     | `cubecl-opt` |
| `zenforks-cubecl-cpp`     | `cubecl-cpp` |
| `zenforks-cubecl-cpu`     | `cubecl-cpu` |
| `zenforks-cubecl-hip`     | `cubecl-hip` |
| `zenforks-cubecl-spirv`   | `cubecl-spirv` |
| `zenforks-cubecl-std`     | `cubecl-std` |
| `zenforks-cubecl`         | `cubecl` (umbrella) |

The non-renamed crates (`cubecl-common`, `cubecl-ir`, `cubecl-macros`,
`cubecl-macros-internal`, `cubecl-zspace`) continue to come from upstream
on crates.io. They were left alone because:

- They sit at the leaves of the dep graph (no transitive dep on a patched crate)
- This keeps the fork surface minimal

## Why this fork exists

We carry a small number of patches against the upstream we need for the
[zenmetrics](https://github.com/imazen/zenmetrics) workspace and the six
`*-gpu` perceptual-metric crates. They are all in flight upstream:

- **pinned-upload fast path** (cubecl-runtime) — ~4x HtoD speedup on
  CUDA workloads. Drafted as upstream PR
  [#1334](https://github.com/tracel-ai/cubecl/pull/1334).
- **persistent PTX cache widening** (cubecl-cuda) — addresses cold-start
  re-compile by including cubecl SHA + GPU compute cap + CUDA runtime
  version in the cache key.
- **Metal `Atomic<f32>` capability honesty** (cubecl-wgpu) — fixes silent
  no-op reductions on the wgpu Metal backend.

All three patches are well documented in the zenmetrics repo under
`crates/zenmetrics-api/docs/` (PINNED_UPLOAD_UPSTREAM_PR.md,
CUBECL_PERSISTENT_PTX_CACHE_PATCH.md, CUBECL_METAL_ATOMIC_FIX.md).

## Versioning

The fork's versions track upstream:

- `0.10.0` — vanilla rename + pinned-upload patch (production-proven
  via `lilith/cubecl@de2f9857` since 2026-05-10).
- `0.10.1` — adds PTX cache widening + Metal atomic capability fix.

When upstream releases `0.11.x`, the next `zenforks-cubecl-*` release
will rebase onto it.

## Using it

In your Cargo.toml, pin to the rename via the `package` field. No source
rewrites are needed because the `[lib]` name is unchanged:

```toml
[dependencies]
cubecl         = { package = "zenforks-cubecl",         version = "0.10.1" }
cubecl-runtime = { package = "zenforks-cubecl-runtime", version = "0.10.1" }
cubecl-cuda    = { package = "zenforks-cubecl-cuda",    version = "0.10.1" }
cubecl-wgpu    = { package = "zenforks-cubecl-wgpu",    version = "0.10.1" }
# Non-renamed crates stay on upstream:
cubecl-common  = "0.10.0"
cubecl-ir      = "0.10.0"
```

Then in source code, write `use cubecl_runtime::*;` as usual — the
shim resolves it to the renamed package.

## Relationship to upstream

We respect the work of the upstream maintainers and submit patches there
first whenever possible. This fork exists to ship downstream work
without waiting on upstream review cycles, not to replace it. When
upstream merges a patch we carry, the next `zenforks-cubecl-*` release
drops our carry of that patch.

The original [README.md](README.md) (upstream's) documents the actual
CubeCL programming model — read that to learn the library.
