#!/usr/bin/env bash
# After the 0.10.0 publish loop has completed all 11 publishes, this
# script:
#   1. Tags v0.10.0 + creates GH release (Stage 4 close)
#   2. Removes the 0.10.0 worktree
#   3. Publishes the 0.10.1 wave from main (Stage 6)
#   4. Tags v0.10.1 + creates GH release (Stage 6 close)

set -eu
cd "$(dirname "$0")/.."

echo "===== STAGE 4 close: tag + release v0.10.0 ====="
./scripts/release_v0_10_0.sh

echo "===== Cleanup: remove 0.10.0 worktree ====="
if git worktree list | grep -q "zenforks-cubecl-work-0.10.0"; then
    git worktree remove /home/lilith/work/zenforks-cubecl-work-0.10.0 || \
        git worktree remove --force /home/lilith/work/zenforks-cubecl-work-0.10.0
fi
git worktree list

echo "===== STAGE 6: publish v0.10.1 from main ====="
./scripts/publish_0_10_1.sh

echo "===== STAGE 6 close: tag + release v0.10.1 ====="
./scripts/release_v0_10_1.sh

echo "===== Stage 4 + Stage 6 complete ====="
