# Hugo Studio

> A native desktop editor for [Hugo](https://gohugo.io/) sites — content,
> menus, data, theme files, media, git — all behind schema-driven forms
> that keep your TOML / YAML / JSON byte-for-byte intact when they don't
> need to change.

<img width="1281" height="802" alt="image" src="https://github.com/user-attachments/assets/3598e5f5-b4a3-4572-a244-5e43f23b485d" />

[![Latest release](https://img.shields.io/github/v/release/sirmmo/hugo-studio?include_prereleases&display_name=tag&color=blue)](https://github.com/sirmmo/hugo-studio/releases/latest)
[![Release workflow](https://img.shields.io/github/actions/workflow/status/sirmmo/hugo-studio/release.yml?label=release%20build)](https://github.com/sirmmo/hugo-studio/actions/workflows/release.yml)
[![CI](https://img.shields.io/github/actions/workflow/status/sirmmo/hugo-studio/build.yml?label=CI)](https://github.com/sirmmo/hugo-studio/actions/workflows/build.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](./LICENSE)
[![Built with Tauri 2](https://img.shields.io/badge/built%20with-Tauri%202-24c8db)](https://tauri.app/)

Hugo Studio targets non-technical content editors and theme authors who
want a real desktop app for their Hugo site — no terminal, no YAML,
no markdown table guesswork — but it also stays useful for power users
because it never reformats files it doesn't have to touch.

---

## Why

Hugo is a great static-site generator with a sharp edge for editors:
front-matter is YAML/TOML/JSON, content is Markdown with shortcodes,
data lives in CSV/JSON, themes pull params from any of three different
files. CMS layers on top usually solve one slice (just content, or just
config) and lose the rest. Hugo Studio covers the whole site —
content, menus, data, theme files, media, git — in one app that runs
locally against the actual filesystem.

Format-preservation is non-negotiable: if you save a post and only
changed the title, the diff is one line. Comments, key order,
indentation, line endings, BOM — all kept.

## What's in the box

| Area               | What you can do                                                                                                                                                                                                                                                                                                                   |
| ------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Content**        | Browse the `content/` tree, edit `.md` and `.html` files. Schema-driven front-matter form (21 standard Hugo fields curated; custom fields inferred from siblings). Markdown body in CodeMirror, rich Markdown WYSIWYG via Milkdown, rich HTML WYSIWYG via TipTap. New-content wizard with archetype dropdown and language picker. |
| **Menus**          | Visual editor for `[menu.<name>]` blocks — name / URL / weight / identifier / parent — add, remove, reorder, save through the format-preserving codecs.                                                                                                                                                                           |
| **Data**           | Browse `data/*` files. CSV opens as a spreadsheet with **resizable columns**; JSON / GeoJSON / YAML / TOML open in a syntax-highlighted editor. Drag-drop CSV / JSON / GeoJSON files from the OS into the panel to import.                                                                                                        |
| **Media**          | Browse `static/`, `assets/`, and the current page bundle. Thumbnail grid (live image previews via Tauri's asset protocol). Drag-drop OS files in, or pick from the editor's "Insert media" modal. Copy URLs, delete, rename.                                                                                                      |
| **Theme**          | Edit theme params with a three-tier schema cascade (manifest > defaults > inferred). Browse and edit raw theme source files (layouts / partials / SCSS / JS / archetypes / i18n) with the right CodeMirror language.                                                                                                              |
| **Site config**    | Curated form for `title`, `baseURL`, `theme`, `paginate`, language settings, and friends. Read-only JSON view for everything else.                                                                                                                                                                                                |
| **Git**            | Clone (HTTPS or SSH — vendored libssh2), branch switcher / new branch, status, stage / unstage, commit, pull, push, stash, force-pull. No system git required.                                                                                                                                                                    |
| **Live preview**   | Embedded `hugo server` (kill-on-drop), iframe pane, collapsible Hugo console. Hugo extended is bundled as a Tauri sidecar — no separate install.                                                                                                                                                                                  |
| **Across the app** | Workspace of multiple sites, native folder picker, persistent per-site state, light / dark / system theme, file-watcher driven refresh.                                                                                                                                                                                           |

## Install

Pre-built installers are attached to every GitHub Release.

| Platform                | Bundle               | First-launch notes                                                                                                                          |
| ----------------------- | -------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| **macOS** (arm64 + x64) | `.dmg`               | Ad-hoc signed, **not notarised**. Right-click → `Open` the first time, or `xattr -dr com.apple.quarantine "/Applications/Hugo Studio.app"`. |
| **Windows** (x64)       | NSIS `.exe`          | Standard installer. SmartScreen may warn on the first run; "More info" → "Run anyway".                                                      |
| **Linux** (x64)         | `.AppImage` + `.deb` | AppImage runs anywhere; `.deb` for Debian / Ubuntu.                                                                                         |

Get them from the [latest release](https://github.com/sirmmo/hugo-studio/releases/latest).

## Build from source

The whole toolchain runs **inside Docker** (`.docker/Dockerfile.dev`),
so the host only needs Docker + Docker Compose. Tauri 2's Linux deps,
Rust 1.85+, Node 20, and Hugo extended all live in the dev image.

```bash
make image        # build the dev image (once, then on Dockerfile changes)
make shell        # interactive shell inside the container
make dev          # Vite dev server only, http://localhost:1420
make tauri-dev    # full Tauri dev (needs an X server / Xvfb on the host)
make fe-build     # production frontend bundle
make cargo-check  # type-check the Rust backend
make fmt          # rustfmt + prettier
make lint         # clippy + eslint
```

A native (non-Docker) setup works too if your host has Rust 1.85+,
Node 20, and Tauri 2's [Linux deps](https://v2.tauri.app/start/prerequisites/#linux):

```bash
npm install
cd src-tauri && cargo build --release
```

## Architecture

```
hugo-studio/
├── .docker/Dockerfile.dev    # dev/build image (Rust + Node + webkit + Hugo)
├── Makefile                  # docker-compose convenience wrappers
├── package.json              # frontend (React 19 + Vite + TypeScript)
├── src/
│   ├── app/                  # entry point, providers, routing
│   ├── components/ui/        # shadcn/ui primitives
│   ├── features/             # one folder per product surface
│   │   ├── content-tree/     editor/
│   │   ├── data/             media/
│   │   ├── menus/            theme/
│   │   ├── git/              preview/
│   │   ├── settings/         site/
│   │   └── workspace/
│   ├── lib/
│   │   ├── tauri/            # typed client generated by tauri-specta
│   │   └── dnd/              # OS drag-drop region registry
│   └── store/                # Zustand stores
└── src-tauri/
    ├── Cargo.toml
    └── src/
        ├── main.rs / lib.rs  # Tauri Builder + commands registry
        ├── commands/         # one file per command surface
        ├── config/           # site-config codecs (toml_edit / yaml line-patch / preserve_order json)
        ├── content/          # tree, FM split + save, schema inference, archetypes
        ├── data/             # data/* file management
        ├── theme/            # theme params schema cascade
        ├── theme_files/      # theme source-file browser
        ├── assets/           # media library (bundle / static / assets)
        ├── git/              # vendored libgit2 wrapper
        ├── preview/          # hugo server lifecycle
        └── watcher/          # debounced notify watcher
```

**Backend ↔ frontend** types are generated from Rust via
[`tauri-specta`](https://github.com/specta-rs/tauri-specta) — every
command is callable from TypeScript with the same name and the same
typed return shape (`src/lib/tauri/bindings.ts`).

**State management** is intentionally minimal: TanStack Query owns
server cache (one query key per `(site, surface)`), Zustand owns the
handful of pieces of UI state that need to persist across components
(active site, current selection, theme mode, preview lifecycle).

## For theme authors: `.hugoeditor/theme-schema.json`

Hugo Studio first looks for an explicit schema at
`themes/<theme>/.hugoeditor/theme-schema.json`. If it's there, the UI
shows a `Manifest` badge and renders **only** the fields you declared.
Minimal example:

```json
{
  "fields": [
    {
      "key": "author",
      "label": "Author name",
      "fieldType": "string",
      "required": true,
      "default": null,
      "enumValues": null,
      "group": "Identity",
      "hidden": false,
      "description": "Visible byline below each post"
    },
    {
      "key": "showReadingTime",
      "label": "Show reading time",
      "fieldType": "boolean",
      "required": false,
      "default": true,
      "enumValues": null,
      "group": "Article meta",
      "hidden": false,
      "description": null
    }
  ],
  "unknownFieldsPolicy": "preserve"
}
```

`fieldType` accepts: `string`, `text`, `number`, `boolean`, `date`,
`dateTime`, `tags` (chip input with autocomplete), `stringArray` (chip
input without autocomplete), `json` (catch-all for objects and mixed
arrays).

Without a manifest, Hugo Studio falls back to reading `[params]` from
`themes/<theme>/config.{toml,yaml,json}` or
`themes/<theme>/theme.toml` (badge `Theme defaults`); failing that it
infers types from the params currently set in the site (badge
`Inferred`).

## For site authors: `.hugoeditor/schema.json` (per-section override)

The same shape works **per content section** to override what Hugo
Studio infers from existing posts. Drop a file at
`<site>/.hugoeditor/schema.json` and the editor will use it for
matching sections.

Until the override loader lands you can still control the form by:

1. naming a custom front-matter field clearly (the editor labels it
   from the key, title-cased), and
2. filling at least one post with a representative value so the type
   inference latches onto it (a tag in any item turns the field into
   a chip input on every other item in the same section).

## Live preview prerequisites

The preview pane spawns `hugo server` against the active site. Hugo
extended is **bundled as a Tauri sidecar** in the release builds, so
end users don't need to install it separately. The lookup order is:

1. **App settings** → cog icon in the workspace header → pick the
   `hugo` binary explicitly. Persisted in `app_data_dir/settings.json`.
2. **`HUGO_STUDIO_HUGO_PATH`** environment variable.
3. **Bundled sidecar** (the one shipped with the app).
4. **`PATH`** lookup of `hugo`.

When building from source, install [hugo extended](https://gohugo.io/installation/)
≥ 0.130 yourself or run the bundled fetch script:

```bash
scripts/fetch-hugo-sidecar.sh "$(rustc -vV | sed -n 's/host: //p')"
```

## Releases

Cross-platform installers are produced by the `release` GitHub Action
when a `v*` tag is pushed. See
[`.github/workflows/release.yml`](./.github/workflows/release.yml).
Each release ships:

- `.dmg` for macOS arm64 and x64 (ad-hoc signed)
- `.AppImage` and `.deb` for Linux x64
- `.exe` (NSIS) for Windows x64
- Hugo extended bundled per platform as a Tauri sidecar

## Roadmap & decisions

- [`ROADMAP.md`](./ROADMAP.md) — what's shipped, what's next, what's
  on the wish list.
- [`DECISIONS.md`](./DECISIONS.md) — one line per design decision
  with the rationale that drove it.

## Contributing

[`CONTRIBUTING.md`](./CONTRIBUTING.md) covers dev setup, conventional
commits, testing expectations, and the PR flow. Issues, ideas and
PRs welcome — Hugo Studio is meant to be community-driven.

If you're a theme author, the most useful thing you can do is ship a
`.hugoeditor/theme-schema.json` with your theme so editors get a real
form instead of inferred fields.

## License

[MIT](./LICENSE) © Hugo Studio contributors. Hugo itself is a separate
project under Apache-2.0.
