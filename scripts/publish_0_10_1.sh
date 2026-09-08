#!/usr/bin/env bash
# Publish all 11 zenforks-cubecl-* crates at 0.10.1.
# These are NEW VERSIONS of existing crates, so the rate limit is
# 30 burst + 1/min refill (much more lenient than new-crate). We can
# chain them in dep order with brief pauses just to be safe.

set -u
cd "$(dirname "$0")/.."

# Dep-order list. Matches the 0.10.0 ordering.
ORDER=(
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

LOGDIR=/tmp/zenforks-publish-0101
mkdir -p "$LOGDIR"

publish_one() {
    local crate=$1
    local logfile="$LOGDIR/$crate.log"
    echo "=== $(date -u +%Y-%m-%dT%H:%M:%SZ) publishing $crate v0.10.1 ==="
    if cargo publish -p "$crate" 2>&1 | tee "$logfile" | tail -3; then
        if grep -q "Published $crate" "$logfile"; then
            echo "+++ $(date -u +%Y-%m-%dT%H:%M:%SZ) $crate v0.10.1 published OK +++"
            return 0
        fi
        echo "--- $(date -u +%Y-%m-%dT%H:%M:%SZ) $crate v0.10.1 did NOT show Published line ---"
        return 1
    fi
    return 1
}

verify_one() {
    local crate=$1
    local url="https://crates.io/api/v1/crates/$crate"
    for attempt in 1 2 3 4 5 6 7 8 9 10; do
        local resp
        resp=$(curl -sf -A "phase8f-verify" "$url" 2>/dev/null) || true
        if echo "$resp" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['crate']['newest_version']=='0.10.1'" 2>/dev/null; then
            echo "+++ verified on crates.io: $crate 0.10.1 +++"
            return 0
        fi
        sleep 5
    done
    echo "--- VERIFY FAILED: $crate not at 0.10.1 on crates.io ---"
    return 1
}

for crate in "${ORDER[@]}"; do
    printf '%s claude-phase8f stage6-publishing-%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$crate" > .workongoing

    attempts=0
    while (( attempts < 6 )); do
        if publish_one "$crate"; then
            break
        fi
        if grep -q "Too Many Requests" "$LOGDIR/$crate.log"; then
            echo "    Rate limited; sleeping 65s..."
            sleep 65
            attempts=$((attempts + 1))
            continue
        fi
        echo "!!! Non-rate-limit error on $crate. Stopping."
        exit 1
    done

    if (( attempts >= 6 )); then
        echo "!!! 6 rate-limit retries exhausted on $crate. Stopping."
        exit 1
    fi

    # Verify the new version is on crates.io before publishing the next
    # crate (which may depend on it).
    if ! verify_one "$crate"; then
        echo "!!! Verify failed for $crate. Stopping to avoid breaking dep order."
        exit 1
    fi

    # Brief pause between PublishUpdate calls. Refill is 1/min but the
    # 30 burst should soak this whole wave anyway.
    if [[ "$crate" != "${ORDER[-1]}" ]]; then
        sleep 5
    fi
done

echo "=== ALL 11 v0.10.1 CRATES PUBLISHED $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
