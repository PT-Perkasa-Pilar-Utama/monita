.PHONY: setup web web-check run dev fmt check-fast check release-check e2e e2e-full release brand clean-stale

# Keep the laptop usable while rustc runs: 4 jobs at low priority. CI sets CARGO_BUILD_JOBS itself.
export CARGO_BUILD_JOBS ?= 4
CARGO = nice -n 10 cargo

setup:
	git config core.hooksPath .githooks
	cd e2e && bun install
	cd web && bun install

# The React frontend (web/), built into web/dist, which the binary embeds.
web:
	cd web && bun install --frozen-lockfile && bun run build

# Type-check and unit-test the frontend.
web-check:
	cd web && bun install --frozen-lockfile && bunx tsgo --noEmit && bun test src

run: web
	MONITA_ENV=dev $(CARGO) run -- serve

# Dev server that reads web/dist from disk: after a web change, `cd web && bun run build`
# and refresh the browser. No Rust rebuild, no restart.
dev: web
	MONITA_ENV=dev $(CARGO) run --no-default-features -- serve

fmt:
	cargo fmt

# Format, lint, unit tests. No browser, no network.
check-fast:
	cargo fmt --check
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) test --lib

# The local gate before hand-off.
check: check-fast web-check
	$(CARGO) test --tests

# Size-optimised build with fat LTO: about 6 minutes. CI runs it; run it locally only before a release.
release-check: check web
	$(CARGO) build --release

e2e: web
	$(CARGO) build
	cd e2e && bun run test

e2e-full: web
	$(CARGO) build
	cd e2e && bun run test:full

release: web
	$(CARGO) build --release --target x86_64-unknown-linux-musl

# The logo and Moni as files (docs/assets/brand), drawn by the app's own components.
brand:
	cd web && bun scripts/brand.tsx
	cd e2e && bun brand-png.ts

# Drop build artifacts not used for 14 days (cargo's own cache GC).
clean-stale:
	cargo clean gc --max-download-age=14days 2>/dev/null || cargo clean
