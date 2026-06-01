#!/usr/bin/env bash
# Generate a flamegraph for a Bastion run.
#
# Prerequisites:
#   cargo install flamegraph
#   Linux: perf must be available (install linux-perf or linux-tools)
#   macOS: dtrace is used automatically by cargo-flamegraph
#
# Usage:
#   ./perf/scripts/flamegraph.sh https://httpbin.org/get 1000 50

set -euo pipefail

URL="${1:-http://localhost:8080}"
REQUESTS="${2:-5000}"
CONCURRENCY="${3:-50}"
OUTPUT="flamegraph_$(date +%Y%m%d_%H%M%S).svg"

echo "Building release binary with debug symbols..."
# We need debug symbols for meaningful flamegraphs, but release optimizations
# to see the actual hot paths (debug builds have different profiles)
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release

echo "Running flamegraph: $REQUESTS requests, $CONCURRENCY concurrent..."
cargo flamegraph \
    --output "$OUTPUT" \
    --bin bastion \
    -- \
    -u "$URL" \
    -n "$REQUESTS" \
    -c "$CONCURRENCY" \
    --no-progress

echo "Flamegraph written to $OUTPUT"
echo "Open with: firefox $OUTPUT  OR  open $OUTPUT (macOS)"