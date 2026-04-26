<!--
Title: use a Conventional Commit prefix matching the lead change.
Examples:
  feat(v1.9): per-section schema overrides
  fix: GitPanel Pull button stayed disabled when behind > 0
  docs: clarify .hugoeditor/theme-schema.json fieldType list
-->

## Summary

<!-- One or two sentences. What changes for the user / contributor? -->

## Why

<!-- The motivation. Linked issue numbers go here. Skip if obvious from the title. -->

## How

<!-- Brief outline of the approach when the diff alone wouldn't make it clear. -->

## Test plan

- [ ] `make lint` passes (frontend `tsc` + `eslint` + `prettier`, backend `cargo fmt --check` + `cargo clippy -- -D warnings`).
- [ ] `cargo test --lib` passes (when the diff touches Rust).
- [ ] Manually exercised the affected surface in `npm run tauri dev` (when UI-visible).

## Screenshots / recordings

<!-- For UI changes. Drag images / GIFs in or paste links. -->

## Notes for reviewers

<!-- Anything non-obvious worth flagging — risky areas, follow-up PRs, deferred work. -->
