#!/usr/bin/env bash
# Publish the remaining 6 zenforks-cubecl-* crates with 10-min rate-limit waits.
# Dep order: cuda -> hip -> spirv -> wgpu -> cpu -> cubecl
# (After spirv is published, wgpu and cpu can both go; we serialize anyway
# to keep within rate limits.)

set -u
cd "$(dirname "$0")/.."

REMAINING=(
    zenforks-cubecl-cuda
    zenforks-cubecl-hip
    zenforks-cubecl-spirv
    zenforks-cubecl-cpu
    zenforks-cubecl-wgpu
    zenforks-cubecl
)

LOGDIR=/tmp/zenforks-publish
mkdir -p "$LOGDIR"

publish_one() {
    local crate=$1
    local logfile="$LOGDIR/$crate.log"
    echo "=== $(date -u +%Y-%m-%dT%H:%M:%SZ) publishing $crate ==="
    if cargo publish -p "$crate" 2>&1 | tee "$logfile" | tail -3; then
        if grep -q "Published $crate" "$logfile"; then
            echo "+++ $(date -u +%Y-%m-%dT%H:%M:%SZ) $crate published OK +++"
            return 0
        fi
        echo "--- $(date -u +%Y-%m-%dT%H:%M:%SZ) $crate did NOT show Published line ---"
        return 1
    fi
    return 1
}

verify_one() {
    local crate=$1
    local url="https://crates.io/api/v1/crates/$crate"
    for attempt in 1 2 3 4 5; do
        local resp
        resp=$(curl -sf -A "phase8f-verify" "$url" 2>/dev/null) || true
        if echo "$resp" | python3 -c "import sys,json; d=json.load(sys.stdin); assert d['crate']['newest_version']=='0.10.0'" 2>/dev/null; then
            echo "+++ verified on crates.io: $crate 0.10.0 +++"
            return 0
        fi
        sleep 10
    done
    echo "--- VERIFY FAILED: $crate not at 0.10.0 on crates.io ---"
    return 1
}

for crate in "${REMAINING[@]}"; do
    # Update marker
    printf '%s claude-phase8f stage4-publishing-%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$crate" > .workongoing

    # Try publish with up to 6 retries on rate-limit
    attempts=0
    while (( attempts < 6 )); do
        if publish_one "$crate"; then
            break
        fi
        # Check if it's rate-limit; if so wait 10 min + jitter
        if grep -q "Too Many Requests" "$LOGDIR/$crate.log"; then
            echo "    Rate limited; sleeping 610s..."
            sleep 610
            attempts=$((attempts + 1))
            continue
        fi
        # Other error: stop
        echo "!!! Non-rate-limit error on $crate. Stopping."
        exit 1
    done

    if (( attempts >= 6 )); then
        echo "!!! 6 rate-limit retries exhausted on $crate. Stopping."
        exit 1
    fi

    # Verify
    if ! verify_one "$crate"; then
        echo "!!! Verify failed for $crate. Continuing anyway (it may take longer to index)."
    fi

    # Always wait between publishes to avoid the second-tier rate limit
    if [[ "$crate" != "${REMAINING[-1]}" ]]; then
        echo "    Sleeping 605s before next publish to stay under rate limit..."
        sleep 605
    fi
done

echo "=== ALL 6 REMAINING CRATES PUBLISHED $(date -u +%Y-%m-%dT%H:%M:%SZ) ==="
