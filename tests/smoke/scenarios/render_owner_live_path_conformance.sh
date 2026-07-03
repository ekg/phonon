#!/usr/bin/env bash
# Smoke: live-path unification conformance suite (I5 / render-owner swap).
#
# Runs the path×model conformance matrix (tests/live_path_conformance.rs):
# every frontend swap path (phonon-live / phonon-audio / modal) driven through
# the shared concurrent primitive under BOTH the shared-cell baseline and the
# render-owner model, asserting the baseline exposes the R1/R2/R3 windows and
# the render-owner model closes every one with an identical, all-green invariant
# vector across all paths.
#
# Exit 0 = PASS, non-zero = FAIL (a regression in the swap-path invariants).
# See docs/audits/design-render-owner-swap-2026-07.md §6.B.
set -euo pipefail

# Resolve the crate root from this script's own location
# (<root>/tests/smoke/scenarios/<this>.sh) so it works regardless of cwd.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/../../.."
if [ ! -f Cargo.toml ]; then
    echo "smoke: could not locate the crate root (no Cargo.toml at $(pwd))" >&2
    exit 1
fi

echo "smoke[render_owner_live_path_conformance]: running the conformance matrix..."
exec cargo test --quiet --test live_path_conformance -- --nocapture
