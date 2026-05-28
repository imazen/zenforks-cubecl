# Changelog

All notable changes to the zenforks-cubecl-* crates are documented here.
The repo carries multiple publishable crates; each crate's release notes
live under its own heading. Shared changes (CI, workspace config) go
under `## Workspace`.

This fork is based on [tracel-ai/cubecl](https://github.com/tracel-ai/cubecl).
Upstream release notes are not duplicated here — see
[the upstream changelog](https://github.com/tracel-ai/cubecl/releases)
for vanilla cubecl history.

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
