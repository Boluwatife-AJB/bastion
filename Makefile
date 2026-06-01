.PHONY: test bench flamegraph lint fmt check clean

# Run all tests with output (--nocapture so tracing output is visible)
test:
	cargo test -- --test-threads=4

# Run tests with output (useful for debugging)
test-verbose:
	cargo test -- --nocapture --test-threads=1

# Run integration tests only
test-integration:
	cargo test --test '*'

# Run unit tests only
test-unit:
	cargo test --lib

# Run criterion benchmarks
bench:
	cargo bench

# Run benchmarks and open HTML report
bench-open:
	cargo bench && open target/criterion/report/index.html

# Generate flamegraph (requires cargo-flamegraph)
flamegraph:
	./perf/scripts/flamegraph.sh

# Lint: clippy at pedantic level (what OSS maintainers expect)
lint:
	cargo clippy -- \
		-D warnings \
		-W clippy::pedantic \
		-A clippy::module_name_repetitions \
		-A clippy::must_use_candidate

# Format
fmt:
	cargo fmt --all

# Full check (what CI runs)
check:
	cargo fmt --all -- --check
	cargo clippy -- -D warnings
	cargo test
	cargo build --release

# Clean artifacts
clean:
	cargo clean
	rm -f flamegraph_*.svg
	rm -f perf.data*