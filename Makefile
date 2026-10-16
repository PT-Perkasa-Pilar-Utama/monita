.PHONY: setup run fmt check-fast check release-check e2e e2e-full release clean-stale

# Keep the laptop usable while rustc runs: 4 jobs at low priority. CI sets CARGO_BUILD_JOBS itself.
export CARGO_BUILD_JOBS ?= 4
CARGO = nice -n 10 cargo

setup:
	git config core.hooksPath .githooks
	cd e2e && bun install

run:
	MONITA_ENV=dev $(CARGO) run -- serve

fmt:
	cargo fmt

# Format, lint, unit tests. No browser, no network.
check-fast:
	cargo fmt --check
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) test --lib

# The local gate before hand-off.
check: check-fast
	$(CARGO) test --tests

# Optimized build with LTO: about 5 minutes. CI runs it; run it locally only before a release.
release-check: check
	$(CARGO) build --release

e2e:
	$(CARGO) build
	cd e2e && bun run test

e2e-full:
	$(CARGO) build
	cd e2e && bun run test:full

release:
	$(CARGO) build --release --target x86_64-unknown-linux-musl

# Drop build artifacts not used for 14 days (cargo's own cache GC).
clean-stale:
	cargo clean gc --max-download-age=14days 2>/dev/null || cargo clean
