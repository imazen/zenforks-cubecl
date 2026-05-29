# Pinned-staging design notes (task142, 2026-05-29)

Why `reserve_staging` / `create_from_slice_pinned` use `submit_blocking`,
and why the async "right level" alternative was measured and rejected for
our pipeline.

## The path

`reserve_staging(sizes)` calls `server.staging(...)` via `submit_blocking`
(blocking round-trip to the device-runner thread). The caller then packs
its source bytes directly into the returned pinned `Bytes` and hands them
to the async `create()`. butteraugli-gpu / iwssim-gpu / cvvdp-gpu /
dssim-gpu all do this to pack sRGB `u8×3` → `u32`-per-pixel directly into
pinned host memory, so the CUDA HtoD copy DMAs from pinned memory
(~12–25 GB/s on PCIe 4.0) instead of the pageable bounce (~5–6 GB/s).

## Why the block is acceptable here

`ComputeServer::staging` → CUDA `reserve_cpu` / `reserve_pinned` is a
**pure host-side pinned-memory-pool allocation**. It never touches the
CUDA stream and never syncs the GPU. The `submit_blocking` block is
therefore just a CPU round-trip to the runner thread for a host alloc.

Measured (RTX 5070, CUDA 13.2, 16 MP, N=8):
- round-trip, idle runner: 0.006–0.13 ms
- round-trip, runner loaded with queued `submit()` closures: 0.19–0.49 ms
- per-upload pack + create: ~12–21 ms  → the block is ≤ 2% of that

## Why the async "right level" alternative is WORSE for us

Prototype (`create_from_slice_pinned_async_probe`, reverted from
`client.rs`, preserved in zenmetrics' `bench_staging_block.rs` behind the
`staging-async-probe` feature): run reserve + pack + upload all on the
runner thread in one non-blocking `submit()`. The caller no longer blocks
(0.008 ms/call) — but:

- end-to-end ~5× slower (885–920 ms vs 170 ms for 8 uploads), and
- pipelined throughput regresses 1.6–1.8× (68–83 ms/pair vs 43–47 ms/pair).

Reason: our upload is a *heavy* host-side pack (~10 ms for 16 MP), not a
plain memcpy. Moving it onto the runner thread serializes it against
kernel-launch dispatch on that same thread, starving compute. Keeping the
heavy pack on the caller thread and blocking only for the cheap pinned-pool
reservation leaves the runner free to dispatch compute — the right tradeoff
for a transfer/compute-overlap orchestrator.

## Decision

Keep the blocking `reserve_staging` + caller-thread pack. Do NOT switch to
the async-on-runner variant for the pack-into-pinned case. (For a plain
`&[u8]` → device upload with no pack, an async-at-the-right-level path
would be fine — the regression is specific to the heavy pack.)

Full numbers + reproducible bench:
`zenmetrics/benchmarks/task142_staging_block_2026-05-29.md`.
