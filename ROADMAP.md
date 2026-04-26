# Hugo Studio — Roadmap

A snapshot of what's shipped, what's next, and what's on the wish list.
The list is descriptive, not a contract — priorities shift with
contributor energy and user feedback.

## Status

**Current release: v1.8.0.** Hugo Studio is feature-complete for the
common Hugo authoring loop (content, menus, data, theme files,
media, git, live preview) on macOS / Windows / Linux.

## Shipped

### v1.0 cycle — the original M0–M9 plan

| Tag             | Theme                                                                                                                                                                           |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **M0**          | Tauri 2 + React + TS bootstrap, Specta-typed Rust↔TS bridge, Docker dev image, CI matrix.                                                                                       |
| **M1**          | Workspace of registered sites, native folder picker, persistent state, Hugo config detection (hugo.\* / config.\* / config/\_default/).                                         |
| **M2**          | Site-config editor with format-preserving codecs (toml_edit, yaml line-patching, preserve_order JSON). Save of an unchanged field doesn't rewrite the file.                     |
| **M3**          | Content tree with kind classification (Section / Branch / Leaf / Single), multilingual detection (Mono / Filename / Directory), debounced `notify` file watcher.                |
| **M4**          | Schema-driven content editor: 21 standard Hugo fields curated, custom fields inferred from siblings, CodeMirror 6 body editor, atomic save with byte-identical no-op detection. |
| **M5**          | Theme params editor with three-tier schema cascade (manifest > defaults > inferred); save lands in the right file (single-file vs `_default/params.*`).                         |
| **M6**          | Live preview — embedded `hugo server` with `kill_on_drop`, iframe pane, collapsible Hugo console, parser for the "Web Server is available at" line.                             |
| **M7**          | Asset import — drag-drop into the editor, bundle / static / assets target picker, sandbox checks, sidebar with insert / delete.                                                 |
| **M8**          | New-content wizard with archetype dropdown, language picker for multilingual sites, Go-template stubs (`.Name`, `.Title`, `.Slug`, `.Section`, `.Date`).                        |
| **M9 / v1.0.0** | App settings dialog with Hugo binary picker, dark mode, bundle code-splitting.                                                                                                  |

### v1.1+ — post-1.0 features

| Tag               | Theme                                                                                                                                                                                 |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **v1.1.0**        | Branch switcher + new-branch UI in GitPanel, custom app icon, Hugo bundled as a Tauri sidecar (no separate install).                                                                  |
| **v1.1.1**        | Dark-mode CodeMirror fix — `@uiw/react-codemirror`'s default `theme="light"` was overriding the CSS variables.                                                                        |
| **v1.2.0**        | Milkdown WYSIWYG as a third "Rich" editor tab for Markdown content.                                                                                                                   |
| **v1.3.0**        | Media library — top-level browse / upload / delete for `static/`, `assets/`, and the current bundle. Inline image previews via Tauri's `asset://` protocol.                           |
| **v1.4.0**        | OS drag-drop into the Media library, `.html` content support (CodeMirror html mode, Rich tab hidden, link insertion uses `<img>`/`<a>` with attribute escaping).                      |
| **v1.4.1**        | HTML pages land on the Body tab by default; `document::save` no longer corrupts bare-body files when the user adds front matter.                                                      |
| **v1.5.0**        | HTML WYSIWYG (TipTap, lazy-loaded), git SSH/HTTPS transports re-enabled, Settings dialog handles long Hugo paths.                                                                     |
| **v1.5.1**        | Hugo menu editor — visual `[menu.<name>]` editing, add / remove / reorder, save through existing config codecs.                                                                       |
| **v1.6.0**        | Data file manager — CSV grid + JSON source editor, sandboxed list / read / write / create / delete.                                                                                   |
| **v1.7.0**        | Theme files browser — categorised file rail (layouts / partials / assets / archetypes / i18n / data / static), CodeMirror with html / css / scss / js / json / markdown highlighting. |
| **v1.7.1–v1.7.3** | Tab layout polish: unified panel shell, fixed Radix `[hidden]` UA-rule override that was leaving inactive tabs in the layout.                                                         |
| **v1.8.0**        | CSV grid resizable columns, OS drag-drop of `.csv` / `.json` / `.geojson` files into `data/`.                                                                                         |

## Up next

Concrete items the next contributor could pick up. Roughly ordered by
"would help most users".

- **Per-section schema overrides** — load `<site>/.hugoeditor/schema.json`
  so editors can override the inferred form per content section.
  Backend cascade is already there (M5 pattern); needs a loader and
  a small UI badge.
- **Image picker for FM fields** — when a front-matter field is named
  `image` / `cover` / `featured`, render an asset picker instead of a
  plain text input. Wire through the existing media library modal.
- **Push diff preview** — before `git push`, surface a one-pane summary
  of the commits about to leave the local branch.
- **Pull non-FF with inline merge UI** — currently force-pull is the
  only escape hatch when the local branch diverges. A small
  three-way merge surface would close that gap.
- **`hugo build` from the UI** — the preview spawns `hugo server`;
  add a "Build to public/" button that runs `hugo` once and surfaces
  the output / error count.
- **Localisation of the UI** — `i18next` already wires cleanly; UI
  strings are in English today.
- **Accessibility audit pass** — keyboard navigation across tabs,
  focus rings inside the data grid, proper ARIA on the resize grips.

## Future / wish-list

Not committed, but interesting:

- **Telemetry (opt-in)** — anonymised "feature ever used" counts to
  guide which surfaces matter most. Hard requirement: explicit
  opt-in, no defaults that send data without consent.
- **Plugin SDK** — community-supplied panels (e.g. SEO checker,
  Cloudinary picker, rss-fixer) loaded from `.hugoeditor/plugins/`.
- **Multi-window editor** — open more than one site at once.
- **Inline comment / preview pane on the body editor** — render Hugo
  shortcodes contextually, show resolved page params.
- **JSON / YAML schema-driven editor** — when a `data/` file matches a
  known schema (e.g. JSON Schema in `.hugoeditor/data-schema.json`),
  render a form instead of raw source.
- **GeoJSON map preview** — render a small Leaflet map for `.geojson`
  files in the data panel.

## Out of scope (intentional)

- **A built-in static-site generator** — Hugo Studio is an editor for
  Hugo, not a Hugo replacement. The `hugo` binary stays the source
  of truth for what the site looks like.
- **Hosting integrations** — Netlify / Vercel / Cloudflare Pages
  deploys are easier through their own CLIs / git push hooks. Hugo
  Studio's git surface is enough to push to whatever your hosting
  consumes.
- **Web version** — the file-watcher, sidecar process management,
  and OS drag-drop all rely on the desktop runtime. A web build
  would mean a different product.

## How to contribute to the roadmap

- For a small feature: open an issue with the
  [feature template](https://github.com/sirmmo/hugo-studio/issues/new/choose),
  describe the workflow it enables, and propose a UI surface.
- For something larger: open a discussion or a draft PR with a short
  design note. Add a one-line entry under "Up next" in the same PR.
- Decisions that change the architecture get one line in
  [`DECISIONS.md`](./DECISIONS.md) with the rationale.
