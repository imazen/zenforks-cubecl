# DRAFT response to PR #1334 review — DO NOT POST without user approval

> **Status: DRAFT. tracel-ai/cubecl is a third-party repo. This must be
> reviewed and posted by the user — not auto-submitted.** (task142)

Target: tracel-ai/cubecl PR #1334 review by @nathanielsimard.

Reviewer comments being addressed:
1. "submit_blocking is expensive and shouldn't be used for data
   transfer, only for data fetching."
2. "the new reserve staging doesn't seem to be implemented at the right
   level."
3. "create_from_slice is mostly used for testing."

---

## Draft reply

Thanks for the review — these are good points and I dug into each one
with measurements on our production pipeline (perceptual image-quality
metrics: butteraugli / SSIMULACRA2 / CVVDP on cubecl). Summary up front:
you're right that `submit_blocking` is the wrong default *as a general
rule*, but for this specific upload path the measured cost is negligible,
and the "right level" async alternative actually regresses our pipelined
throughput. Details below in case they're useful for deciding how (or
whether) to land this.

### 1. `submit_blocking` cost — measured, and it's not a GPU stall

The `submit_blocking` in `reserve_staging` blocks on a round-trip to the
device-runner thread, but the task it runs (`ComputeServer::staging` ->
CUDA `reserve_cpu` / `reserve_pinned`) is a **pure host-side
pinned-memory-pool allocation**. It never touches the CUDA stream and
never syncs the GPU. So the block is a CPU-thread round-trip for a host
allocation, not a transfer or a fetch.

Measured on an RTX 5070 / CUDA 13.2, 16 MP (4096×4096), 8 back-to-back
uploads, u32-packed staging (64 MB/upload):

| | time |
|---|---|
| `reserve_staging` round-trip, idle runner | 0.006–0.13 ms |
| `reserve_staging` round-trip, runner loaded with queued `submit()` closures | 0.19–0.49 ms |
| full pinned pack + create, per upload | ~12–21 ms |
| pipelined upload + compute, per pair | ~43–47 ms |

The block is ≤ 2% of per-upload cost and < 1% of the pipelined per-pair
cost. I agree it would be expensive if `staging` did device work behind
the blocking call — it doesn't here, which is why the impact stays small.

### 2. "Right level" — I prototyped the async version and it's slower for us

I built exactly what I think you're suggesting: a single non-blocking
`submit()` that reserves the pinned buffer, runs the pack closure, and
queues the upload all on the runner thread — caller never blocks for
staging. The caller-thread per-call time drops to ~0.008 ms (the block is
gone). But end-to-end it's ~5× slower, and **pipelined throughput
regresses 1.6–1.8×** (≈ 68–83 ms/pair vs 43–47 ms/pair).

Root cause: our upload isn't a plain memcpy — it's a host-side pack
(sRGB `u8×3` → `u32` per pixel, ~10 ms for 16 MP) that has to happen
before/into the staging buffer. Moving that heavy pack onto the runner
thread serializes it against kernel-launch dispatch on the *same* runner
thread, so compute can't be dispatched while the pack runs. Keeping the
heavy pack on the caller thread (and blocking only for the cheap
pinned-pool reservation) leaves the runner free to dispatch compute — which
is what we want in a pipelined orchestrator overlapping transfer and
compute across CUDA streams.

So for a *plain* `&[u8]` → device upload (no pack), the async-at-the-right-
level approach is clearly correct and I'd happily route through it. For a
pack-into-pinned upload, the blocking reservation + caller-thread pack
measured strictly better. If upstream prefers the async-only shape, the
pack-into-pinned ergonomic could be dropped and callers could pack into
their own pinned buffer obtained some other way — but then we lose the
single-pass pack that's the whole point of the helper.

### 3. "create_from_slice is mostly used for testing" — not for image/codec pipelines

This assumption doesn't hold for our use case. Our production input is
`&[u8]` (sRGB bytes straight out of an image decoder), so the slice path
**is** the production upload path, not a test convenience. The pinned
variant exists specifically to make that production path fast:
`create_from_slice` (pageable → pinned, two host memcpys) measured
~100 ms/upload at 16 MP vs ~12–21 ms/upload for the pinned single-pass
pack — a ~6.5× difference dominated by the pageable bounce + the extra
host copy. For codec/imaging workloads the slice → device path is hot, and
worth a fast path.

### What we're doing on our side

Given the measurements, we're keeping the blocking `reserve_staging` +
caller-thread pack in our fork (it's the fastest option we measured for
this pattern) rather than switching to the async variant. We're not
asking upstream to take the patch as-is — sharing the numbers in case
they help shape whichever direction you prefer. Happy to rework toward an
async-only API if you'd rather the helper not block, with the caveat above
about the pack location.

Bench + full numbers (reproducible): `examples/bench_staging_block.rs`
(`--features staging-async-probe` enables the async-probe comparison).

---

## Notes for the user before posting

- This is a **third-party repo** (tracel-ai/cubecl). Per house rules it
  needs your explicit approval before posting.
- Tone is collaborative and avoids any shade — frames our findings as our
  measurements, not as a correction of the maintainer.
- The async-probe method was prototyped + measured in the zenmetrics
  workspace (gated bench), then reverted from the fork's `client.rs` so
  the published production path stays untouched. The probe code is
  preserved in `examples/bench_staging_block.rs` behind the
  `staging-async-probe` feature for reproducibility.
- Full measurement table:
  `zenmetrics/benchmarks/task142_staging_block_2026-05-29.md`.
