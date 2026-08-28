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
- `0.10.2` — strict f64 handling on Metal, CPU `sync_cube`, worker-thread
  panic removal, CI/lint fixes. **Not published to crates.io** — consumed
  by zenmetrics as a `git` rev pin (see "Resync log" below).

## Resync log

### 2026-08-28 — upstream surveyed, fork deliberately held at v0.10.0

An `upstream` remote (`tracel-ai/cubecl`) was added and fetched. State at
the time of the survey:

| Ref | SHA | Date |
|---|---|---|
| `upstream/main` | `9b01400e7f630fcbbd10cc6eebf73b82640120a3` | 2026-08-27 |
| newest upstream tag `v0.11.0-pre.3` | `b566e954468010303cf41465fc8b6be6499e2001` | 2026-08-25 |
| fork base `v0.10.0` | `7cf203735e095e640a2c03b2400d0faa03196bb4` | 2026-05-07 |

`upstream/main` is 234 commits ahead of `v0.10.0` (867 files,
+94 645 / −55 652).

**The fork stays on `v0.10.0`.** This is a measured decision, not a
deferral. Upstream commit
[`cb87b0d2`](https://github.com/tracel-ai/cubecl/commit/cb87b0d2ba4a14234d751a536dabb0540646e190)
(2026-05-15, PR #1322, *"mega-refactor: Totally change the frontend to
enable references, among other things"*) replaced the kernel argument
model. **Every upstream tag published after `v0.10.0` is downstream of
that commit** (verified: `git merge-base --is-ancestor cb87b0d2
v0.11.0-pre.1` → true), so there is no newer tag that avoids it.

What the refactor changes, and why zenmetrics cannot absorb it as a
side-effect of a resync:

| 0.10 | 0.11-pre |
|---|---|
| `fn k(a: &Array<T>, b: &mut Array<T>)` | `fn k(a: &[T], b: &mut [T])` |
| `ArrayArg::from_raw_parts(h, n)` | `BufferArg::from_raw_parts(h, n)` |
| implicit scalar coercion — `x * 2.0` | explicit — `x * Vector::new(F::new(2.0f32))` |
| `cubecl::stream_id::StreamId` | moved |
| `StorageType` / `ExecutionMode` / `KernelSettings` in prelude | moved to `cubecl_ir` / `cubecl_ir::settings` |

Measured blast radius in `imazen/zenmetrics` (`grep -rn … crates/`):
**1 280 `ArrayArg` sites, 477 kernel-launch sites, 261 `#[cube]`
functions, ~111 k LOC of GPU kernel source** across the six `*-gpu`
crates. The third row of that table is the expensive one: dropping
implicit scalar coercion means every literal in every kernel's arithmetic
has to be rewritten and then **re-verified for bit-exact parity against
the CPU reference**, because a silently-changed rounding path is exactly
the class of defect these metrics exist to detect.

This was confirmed empirically, not assumed: `zenmetrics` was temporarily
repointed at a `v0.11.0-pre.3` checkout and built. `cubecl-wgpu` 0.11
itself compiled fine; `zenmetrics-gpu-core` (761 LOC, the *smallest*
consumer) failed with 20 errors from exactly two root causes
(`cubecl::stream_id` moved, `Array<T>` no longer `LaunchArg`). Scaling
that to 111 k LOC of kernels is a dedicated migration with its own parity
campaign, not a resync step.

Advancing to the last pre-refactor upstream commit (`cb87b0d2^`) was also
evaluated and rejected: that window is only 27 commits (2026-05-07 →
2026-05-15) and contains nothing touching the Metal or wgpu backends —
a Vulkan `shader_long_vector` feature, CPU-runtime perf, a metadata
tiled-layout API, and a CUDA stream-priority hint. It would cost a full
patch rebase for no downstream benefit.

**Action item for a future session:** the 0.11 migration is a standalone
project — port the kernel argument model, then re-run the full parity
grid before trusting a single score. Upstream's new native `cubecl-metal`
backend (added in 0.11, alongside `cubecl-environment` and `cubecl-llvm`)
is the strongest reason to eventually do it.

#### Fork commits reconciled this pass

| Commit | Disposition |
|---|---|
| `385373f6` `fix: stop panicking on worker threads…` (branch `fix/no-worker-thread-panics`) | **landed on `main`** — was never merged |
| `5d410fa5` `feat(wgpu): strict f64 handling…` (branch `feat-f64-metal-downgrade`) | **already on `main`** as `2509e67c` (PR #2); verified content-identical (`git diff` empty). Branch is redundant. |
| `7a3c9845` `feat: per-kernel fast-math control` (branch `feat-per-kernel-fast-math`) | **held, not landed** — see below |

`feat-per-kernel-fast-math` does not compile against the wgpu version
this fork resolves. Its `cubecl-wgpu` hunk sets
`wgpu::ShaderRuntimeChecks { fast_math, .. }`, but **wgpu 29.0.4 has no
`fast_math` field** (`grep -rn 'fast_math' wgpu-29.0.4/ wgpu-types-29.0.4/`
→ no matches; the struct's fields are `ray_query_initialization_tracking`,
`task_shader_dispatch_tracking`, `mesh_shader_primitive_indices_clamp`).
The commit message itself notes it "pairs with the wgpu-side change",
i.e. it presumes a wgpu fork that this repo does not carry.

Landing it with the enforcement line stubbed out was considered and
rejected: it would ship a `#[cube(precise)]` attribute that silently does
nothing on the Metal backend — the exact silent-no-op failure mode the
Metal `Atomic<f32>` capability patch exists to eliminate. The branch is
kept intact for whenever wgpu exposes the knob (or the fork takes a wgpu
patch of its own).

When upstream releases `0.11.x` stable, rebasing is a migration project,
not a version bump — read this section first.

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
