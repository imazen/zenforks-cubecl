# Changelog

All notable changes to the zenforks-cubecl-* crates are documented here.
The repo carries multiple publishable crates; each crate's release notes
live under its own heading. Shared changes (CI, workspace config) go
under `## Workspace`.

This fork is based on [tracel-ai/cubecl](https://github.com/tracel-ai/cubecl).
Upstream release notes are not duplicated here — see
[the upstream changelog](https://github.com/tracel-ai/cubecl/releases)
for vanilla cubecl history.

## Workspace

### [Unreleased]

#### Changed

- **All 11 renamed crates bumped to `0.10.2`** (workspace `version`, plus
  the 40 inter-crate `version = "0.10.1"` pins). The 5 non-renamed leaves
  (`cubecl-common`, `cubecl-ir`, `cubecl-macros`, `cubecl-macros-internal`,
  `cubecl-zspace`) stay pinned at upstream's `0.10.0`. **Not published to
  crates.io** — `imazen/zenmetrics` consumes this as a `git` rev pin.

#### Documentation

- `ZENFORKS_README.md` gains a **Resync log** recording the 2026-08-28
  upstream survey: `upstream/main` at `9b01400e`, newest upstream tag
  `v0.11.0-pre.3` at `b566e954`, and the measured reason the fork stays on
  `v0.10.0` — upstream `cb87b0d2` (PR #1322) replaced the kernel argument
  model, and every upstream tag after `v0.10.0` is downstream of it.

## zenforks-cubecl-runtime

### [Unreleased]

#### Fixed

- **Worker-thread panics converted to caller-visible errors.** A panic on
  a cubecl-internal thread is invisible to any `Result` on the calling
  thread, so the caller receives `Ok` carrying a garbage result with no way
  to detect the failure — in a batch pipeline that is data poisoning
  (plausible in-range scores, exit 0). (`385373f6`, refs #4,
  imazen/zenmetrics#41)

## zenforks-cubecl-cuda

### [Unreleased]

#### Fixed

- **Missing CUDA toolkit no longer panics inside `Context::compile_kernel`.**
  Adds fallible `try_cuda_path` / `try_include_path` / `try_cccl_include_path`
  and maps failure to `CompilationError::Generic` in a function that already
  returned `Result`. `try_include_path` also verifies `include/cuda_runtime.h`
  actually exists — `cuda_path()` accepts any directory that merely exists, so
  a driver-only install yielded a header-less path and NVRTC then failed later
  with a far less obvious message. The panicking wrappers are kept (now
  documented as panicking) so no consumer breaks. Also replaces
  `to_str().unwrap()` on include paths with `to_string_lossy`. (`385373f6`)

## zenforks-cubecl-cpu

### [0.10.2] - 2026-05-28

#### Fixed

- **Multi-cube SharedMemory + sync_cube isolation.** The MLIR visitor
  generated 3 nested `scf::for` loops over `CubeCount*` inside the
  per-unit kernel body, but the global `sync_cube` barrier in
  `compute_task.rs` (counted in `cube_dim_size` arrivals) lost
  shared-memory isolation between cubes — different units could
  advance to different cube iterations between syncs, so cube k's
  units could read shared memory written by cube k+1's unit 0.
  Surfaced on cvvdp-gpu's `downscale_tiled_kernel` (LDS-tiled 5x5
  gauss reduce, 16x16 workgroup + 36x36 `SharedMemory` tile): worked
  correctly at 32×32 (1 workgroup) but diverged by 1.3 cells on
  73×91 inputs (3x3 workgroups). End-to-end downstream impact for
  the cvvdp JOD metric: ~1.73 JOD divergence vs pycvvdp v0.5.4 at
  73×91 odd-dim, dropping to f32-precision parity (~1e-6 JOD) after
  the fix. Fix: emit an implicit `sync_cube` call at the end of every
  cube-iteration body in the visitor's innermost `scf::for`. (93dd86d9)
- Pre-existing test compilation error: `FastMath::all().difference(...)`
  expected `EnumSet<FastMath>` but received a bare enum variant.
  Apply the compiler-suggested `.into()` coercion. (04e4ffad)

#### Tests

- New regression test `test_sync_cube_multi_cube_writes_pos_cpu`:
  3 cubes × 4 units; cube k's unit 0 writes `CUBE_POS_X = k` to
  shared memory; all 4 units in cube k must read `k`. Without the
  fix: `[0,0,0,0, 1,2,1,1, 2,2,2,2]`. With the fix:
  `[0,0,0,0, 1,1,1,1, 2,2,2,2]`. (93dd86d9)

## Workspace

### [0.10.1] - 2026-05-27

- Initial rename pass: 11 zenforks-cubecl-* crates published on
  crates.io. See `PHASE8F_STATE.md` for the full provenance map
  and the per-patch scope split between 0.10.0 (vanilla rename
  + pinned-upload) and 0.10.1 (PTX cache widening + Metal Atomic
  capability honesty).
- `zenforks-` prefix added to release tags (`zenforks-v0.10.1`)
  to disambiguate from upstream `v0.10.1` if it ever ships.

### [0.10.0] - 2026-05-27

- Vanilla fork of `tracel-ai/cubecl` at `v0.10.0`
  (`7cf203735e095e640a2c03b2400d0faa03196bb4`) plus the
  pinned-upload patch that ships `client.create_from_slice_pinned`
  for production-proven 4x HtoD throughput.
