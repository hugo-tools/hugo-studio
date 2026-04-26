.PHONY: image shell dev tauri-dev fe-build cargo-check fmt lint clean reset hugo-sidecar

# Build the dev image (run once, then on Dockerfile changes).
image:
	docker compose build

# Interactive shell inside the dev container.
shell:
	docker compose run --rm --service-ports dev bash

# Vite dev server only (frontend without Tauri shell).
dev:
	docker compose run --rm --service-ports dev npm run dev

# Tauri dev (spawns the desktop binary — needs an X server / Xvfb on this host).
tauri-dev:
	docker compose run --rm --service-ports dev npm run tauri dev

# Production frontend bundle.
fe-build:
	docker compose run --rm dev npm run build

# Rust type-check the backend.
cargo-check:
	docker compose run --rm dev cargo check --manifest-path src-tauri/Cargo.toml

# Run the backend test suite (includes the bindings round-trip later).
cargo-test:
	docker compose run --rm dev cargo test --manifest-path src-tauri/Cargo.toml

# Regenerate src/lib/tauri/bindings.ts from the live command list.
gen-bindings:
	docker compose run --rm dev bash -lc "cd src-tauri && cargo run --quiet --bin gen-bindings"

# Format both sides.
fmt:
	docker compose run --rm dev bash -lc "cargo fmt --manifest-path src-tauri/Cargo.toml && npm run fmt"

# Lint both sides (clippy + eslint).
lint:
	docker compose run --rm dev bash -lc "cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings && npm run lint"

# Stop containers and drop named volumes (cargo cache, node_modules, target).
clean:
	docker compose down -v

# Same as clean but also removes the image — use sparingly.
reset: clean
	docker rmi hugo-studio-dev:latest || true

# Stage the Hugo sidecar binary for the local Linux dev build. The dev
# image already has a system-wide Hugo, so the cargo build has something
# to bundle even outside CI. Override TARGET to fetch a different triple.
hugo-sidecar:
	docker compose run --rm dev bash -lc \
	  "scripts/fetch-hugo-sidecar.sh $${TARGET:-x86_64-unknown-linux-gnu}"
