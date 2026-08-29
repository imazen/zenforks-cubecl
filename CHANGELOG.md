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

#### Fixed

- **CI reported green while the entire test suite was skipped.** `ci.yml`
  gated `linux-std-tests` and `linux-miri-tests` on
  `github.repository == 'tracel-ai/cubecl'` (`e0c5981a`, 2026-06-11), so on
  `imazen/zenforks-cubecl` both jobs were skipped — logged as `-` in `0s` —
  while the run still reported ✓. Every push to this fork since then, and
  every fork-specific change in it (the crate renames, the pinned-upload
  feature, the Metal f64 downgrade, the worker-thread panic removal, the
  storage-binding alignment fix) landed with **zero** executed tests. The
  MSRV leg (`RUST_PREVIOUS_VERSION = 1.92.0`) and the miri UB run rode along
  in the same skip. Only `prepare-checks`, `code-quality` and
  `documentation` ever ran.

  The guard was a symptom fix, not the cause. `runs-on` targets tracel's
  self-provisioned GCP runner labels (`@id:cubecl-…`, `n2-standard-16`),
  which resolve to nothing outside upstream; before the guard the jobs
  queued forever and held the run hostage. Both jobs now run on a
  GitHub-hosted `ubuntu-24.04` runner (matching upstream's
  `ubuntu-2404-lts-amd64` image family) with no repository guard.
  (`75582530`)

  First execution — run `33258627184`, read from the job logs rather than
  the check marks — passed clean: **1,483 tests across 31 binaries on each
  of the stable (1.98.0) and MSRV (1.92.0) legs, 0 failed**, plus 38
  `cubecl-common` tests under `cargo miri test` in UB-only mode. Across the
  21 CI runs preceding it, these two jobs had concluded `skipped` 30 times
  and `cancelled` 11 times, and `success` **zero** times. That clean result
  is only clean because the `xtask test` breakage below was fixed in the
  same push; run as it stood, the job would have failed at
  `-p cubecl-wgpu`. The fork's divergence from upstream (crate renames,
  pinned upload, Metal f64 downgrade, worker-thread panic removal,
  storage-binding alignment) turns out not to have broken anything the suite
  covers — but that is now established rather than assumed.
- **`xtask test` still referenced pre-rename package names.** `xtask test --ci`
  excluded `cubecl-cuda`/`cubecl-hip` and ran its extra
  `exclusive-memory-only` pass against `-p cubecl-wgpu` — none of which are
  package names in this workspace since the `zenforks-cubecl-*` rename.
  `3823bd0b` fixed the same staleness in `check.rs`, which runs on every CI
  run; `test.rs` was missed because it had never executed once. Both spellings
  fail, differently: `-p cubecl-wgpu` is a hard `package ID specification …
  did not match any packages` error, while `--exclude cubecl-cuda` is only a
  warning — so the exclusion silently would not have applied and CUDA/HIP
  would have been pulled into a CI run with no toolkit installed.
  (`8fdd5bc1`)
- **`runtime_tests` was outside the lint gate entirely.** `xtask check lint`
  runs `cargo clippy --no-deps` with default features, so it compiles no test
  or bench targets and nothing behind a feature gate. cubecl-core's
  `runtime_tests` — the GPU suite that ships in the published crate, gated on
  `export_tests` — was never linted. An explicit `--all-targets` clippy pass
  now covers it (`87ba7451`); the 26 `float_literal_f32_fallback` findings it
  surfaced are annotated in `d4a78e36`. `Float::new` takes `f32`, so those
  literals were already `f32` by fallback — the suffixes are annotations, not
  value changes.
- **Typos gate read git SHAs as prose.** Citing commit `87ba7451` failed the
  check: the hash was split into words and a two-letter fragment of it was
  reported as a misspelling. Since every entry here must name its commit,
  hashes are permanent content and future ones can collide with dictionary
  words by chance. `typos.toml` now ignores backtick-quoted hex via
  `extend-ignore-re`, rather than adding that fragment as a global word
  exception. Verified against typos 1.23.4 — the version CI installs — that
  the repo is clean and that the same fragment appearing in ordinary prose is
  still flagged. (`4570872b`)

#### Changed

- **`prepare-checks` given a real step.** Its only step was `Do Nothing`
  guarded `if: false`. The job is not vestigial: the `env` context is
  unavailable to `jobs.<id>.strategy` but available to `jobs.<id>.outputs`,
  so the MSRV matrix leg of `linux-std-tests` can only reach
  `RUST_PREVIOUS_VERSION` through
  `needs.prepare-checks.outputs.rust-prev-version`, and a job must declare at
  least one step. It now echoes the value it exports. (`75582530`)

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

## zenforks-cubecl-wgpu

### [Unreleased]

#### Fixed

- **Memory pool aligned to the wrong limit.** `create_server` derived
  `MemoryDeviceProperties.alignment` from `min_uniform_buffer_offset_alignment`
  alone, but the main pool is created with `BufferUsages::STORAGE` and the
  uniform pool with `UNIFORM | STORAGE`, so a sub-allocation from either can be
  bound as a storage buffer — governed by `min_storage_buffer_offset_alignment`.
  On a device reporting a larger storage alignment, cubecl's own pool offsets
  would be rejected by `create_bind_group`. Now takes the max of both. Latent on
  Apple Silicon (both report 256), but wrong by construction. (`f528c4b5`)
- **Unaligned binding offsets panicked on the device-service thread with a
  message naming neither the cause nor the caller.** `register_pipeline` handed
  resource offsets to `create_bind_group` unchecked; wgpu validates there, and
  on a `DSD-*` thread the error arrives as a panic no `Result` on the caller's
  thread can observe (the caller saw `client.rs: called Result::unwrap() on an
  Err value: CallError`). Now asserts each offset against the device's
  `min_storage_buffer_offset_alignment` — cached in `WgpuStream` at construction,
  since `Device::limits()` clones the whole `Limits` struct — and reports the
  offset, the required alignment, and the fact that cubecl's own allocations are
  always aligned, so an unaligned one came from a caller-built
  `Handle::offset_start(..)` sub-view. Same failure class as `385373f6`. This
  makes the failure honest and actionable; it does not make an unaligned
  sub-view work (the caller must round down and pass the leftover element offset
  to the kernel). Found via imazen/zenmetrics cvvdp-gpu's Mode B strip walker,
  whose image-pyramid levels have row strides below 256 bytes;
  imazen/zenmetrics#24. (`f528c4b5`)

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
