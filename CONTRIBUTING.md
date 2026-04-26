# Contributing to Hugo Studio

Thanks for your interest in contributing — bug reports, feature ideas
and pull requests are all welcome.

## Reporting bugs / asking for features

Open an issue using the
[bug](https://github.com/sirmmo/hugo-studio/issues/new?template=bug.yml)
or
[feature](https://github.com/sirmmo/hugo-studio/issues/new?template=feature.yml)
template. The more reproducible the bug, the faster the fix; for
features, describe the workflow you want to enable, not just the
control you want added — that helps us pick the right surface.

## Development setup

Hugo Studio's dev toolchain runs **inside Docker** so you don't have
to fight Tauri 2's Linux deps, Rust 1.85+, Node 20, and Hugo extended
on your host.

```bash
git clone https://github.com/sirmmo/hugo-studio.git
cd hugo-studio
make image     # builds .docker/Dockerfile.dev once
make shell     # interactive shell inside the container
```

Inside the container you have:

```bash
npm run dev           # Vite dev server only (no Tauri shell)
npm run tauri dev     # full Tauri dev (needs an X server / Xvfb on the host)
npm run typecheck     # tsc --noEmit
npm run lint          # eslint
npm run fmt           # prettier --write
cd src-tauri
cargo build           # Rust backend
cargo test --lib      # unit tests
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

### Without Docker

If your host has Rust 1.85+, Node 20, and Tauri 2's
[Linux deps](https://v2.tauri.app/start/prerequisites/#linux), you can
skip Docker:

```bash
npm install
cd src-tauri && cargo build
npm run tauri dev
```

For the live preview to work locally, either install
[hugo extended](https://gohugo.io/installation/) ≥ 0.130 or run the
sidecar fetcher:

```bash
scripts/fetch-hugo-sidecar.sh "$(rustc -vV | sed -n 's/host: //p')"
```

## Conventions

### Commits

[Conventional Commits](https://www.conventionalcommits.org/), with the
type prefixes you'd expect:

- `feat:` — user-visible feature
- `fix:` — user-visible bug fix
- `refactor:` — code restructuring with no behaviour change
- `docs:` — README / ROADMAP / inline doc changes
- `test:` — tests only
- `chore:` — tooling, deps, CI
- `style:` — formatter output

For features that span a release, prefix the version: `feat(v1.6):`.

### Branches & PRs

- Work on a feature branch off `main`.
- PR titles use the same conventional-commit prefix as the lead commit.
- Keep diffs small and focused — split unrelated changes into
  separate PRs.
- The PR description should answer "what" and "why"; CI logs answer
  "how".

### Tests

Backend changes need unit tests when they touch:

- a codec / parser (`src-tauri/src/config/`, `src-tauri/src/content/document.rs`),
- a sandbox check or path-resolution function,
- anything that walks the filesystem,
- new commands (one happy-path + one error case is the floor).

Run the suite locally before opening a PR:

```bash
cd src-tauri && cargo test --lib
```

Frontend changes don't have a unit-test gate today, but `tsc --noEmit`,
`eslint`, and `prettier --check` must pass — `make lint` runs all
three.

### Style

- Rust: `cargo fmt`, no `unwrap()` / `expect()` in non-test code (use
  the typed `AppError` instead).
- TypeScript: `prettier`, `eslint`, no `any` unless paired with a
  one-line comment justifying it.
- Comments document *why*, not *what*. The CI green light proves
  *what*; only the *why* survives in the diff months later.

### Architecture decisions

Decisions that change the shape of the app go in
[`DECISIONS.md`](./DECISIONS.md) — one line per decision with the
rationale. If you're unsure whether a change qualifies, err on the
side of writing it down; future contributors will thank you.

## Releasing (maintainers only)

1. Bump the version in `package.json`, `src-tauri/Cargo.toml`, and
   `src-tauri/tauri.conf.json` (keep them in sync).
2. `cargo build --bin gen-bindings` to refresh `Cargo.lock`.
3. Conventional `feat:`/`fix:` commit summarising the release.
4. `git tag -a vX.Y.Z -m "vX.Y.Z — short summary"` and push the tag.
5. `release.yml` builds DMG / AppImage / `.deb` / `.exe` and attaches
   them to a draft GitHub Release. Promote the draft when the binaries
   smoke-test cleanly on each platform.

## Code of conduct

Be kind. Disagree about ideas, not people. If something feels off in
an issue or PR, flag it to a maintainer rather than escalating in
public.
