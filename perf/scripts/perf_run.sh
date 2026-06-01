#!/usr/bin/env bash
# Run Bastion under Linux perf for detailed CPU profiling.
# Linux only. Produces a perf.data file for analysis.

set -euo pipefail

URL="${1:-http://localhost:8080}"

echo "Building with debug symbols in release mode..."
CARGO_PROFILE_RELEASE_DEBUG=true cargo build --release

echo "Running under perf record..."
perf record \
    --call-graph dwarf \
    --freq 997 \
    target/release/bastion \
    -u "$URL" \
    -n 10000 \
    -c 100 \
    --no-progress

echo "Generating report..."
perf report --stdio | head -50

echo "For interactive flamegraph:"
echo "  perf script | stackcollapse-perf.pl | flamegraph.pl > out.svg"