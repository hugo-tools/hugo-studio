# Hugo Studio — Roadmap

Le milestone seguono il prompt iniziale. Ogni voce è atomica e testabile.

## M0 — Bootstrap

- [x] Repo inizializzato (Tauri 2 + React + TS + Vite + Tailwind + shadcn/ui)
- [x] `specta` + `tauri-specta` per codegen Rust↔TS
- [x] CI minima (matrix build su macOS / Linux / Windows)
- [x] Comando Rust `health_check` chiamato dal frontend con TanStack Query
- [x] Ambiente di sviluppo Docker (`Dockerfile.dev` + `docker-compose.yml` + `Makefile`)
- [x] DECISIONS.md + ROADMAP.md
- [x] Primo commit `chore: bootstrap hugo studio (M0)`
- [ ] **Criterio**: app si apre su mac/linux/win e mostra "ready" — verifica visiva su un host con display ancora da fare (in Docker headless è validato build + comando)

## M1 — Workspace e site detection

- [ ] Tipi `Workspace`, `SiteRef`, `Site` in Rust con codegen TS via specta
- [ ] Comandi `workspace_*` + `site_open`
- [ ] Persistenza workspace in `app_data_dir/workspace.json`
- [ ] UI: schermata iniziale con elenco siti, "Add site" (file picker), "Open"
- [ ] Detection `hugo.{toml,yaml,json}` / `config.{toml,yaml,json}` / `config/_default/`
- [ ] **Criterio**: aggiungo 2 siti, switch tra loro, l'app ricorda la lista al riavvio

## M2 — Lettura della config del sito

- [ ] Parser multi-formato con preservazione: `toml_edit` per TOML, parser custom/yaml-rust2 per YAML, `serde_json` con `preserve_order` per JSON
- [ ] Merge della cascata `config/_default/` + environment override
- [ ] Comandi `config_get` / `config_save`
- [ ] UI: pannello "Site Settings" con form generato + sezione "Advanced" key/value
- [ ] **Criterio**: round-trip byte-identico per le sezioni non modificate (test automatici)

## M3 — Content tree e lettura contenuti

- [ ] Scansione `content/` con classificazione `SinglePage` / `LeafBundle` / `BranchBundle`
- [ ] Detection multilingua (filename vs directory)
- [ ] Comando `content_list` con cache TanStack Query
- [ ] UI: file tree con icone per kind, indicatore draft, switcher di lingua
- [ ] File watcher `notify` debounced 200ms
- [ ] **Criterio**: tree popolato per Hugo Coder / PaperMod, modifica esterna riflessa in <1s

## M4 — Editor di contenuto (cuore del prodotto)

- [ ] Schema inference del front matter (campi standard Hugo + custom inferred + override `.hugoeditor/schema.json`)
- [ ] Form renderer da schema → componenti shadcn/ui
- [ ] Body editor CodeMirror 6 con highlight markdown e preview side-by-side
- [ ] Layout split: tree | form+body | preview placeholder
- [ ] Save con preservazione formato
- [ ] **Criterio**: modifica title/tags/draft + body → diff = solo le modifiche; tags con autocomplete

## M5 — Theme params editor

- [ ] Cascata schema: manifest opt-in → defaults del tema → inferenza dai params correnti
- [ ] Pannello "Theme Settings" raggruppato; badge fonte schema
- [ ] **Criterio**: PaperMod, modifica `params.author`, salvataggio corretto in `hugo.toml`

## M6 — Live preview

- [ ] Hugo bundlato come Tauri sidecar (>= 0.130 extended) + fallback system binary
- [ ] Spawn/kill di `hugo server` con porta libera + `kill_on_drop`
- [ ] WebView panel + auto-navigate
- [ ] Console panel collassabile per i log Hugo
- [ ] **Criterio**: refresh <1s; nessun processo Hugo zombie alla chiusura

## M7 — Asset management

- [ ] Comandi `asset_*`
- [ ] Drag-drop context-aware (bundle vs static vs assets) + dialog "Where to put this?"
- [ ] Image picker per campi front matter con preview thumbnail
- [ ] **Criterio**: drag in un leaf bundle → file copiato + link inserito al cursore

## M8 — Creazione contenuti e archetypes

- [ ] Lettura archetypes (`archetypes/{section}.md` + `default.md`)
- [ ] Wizard "New content" (sezione, slug, archetype, lingua)
- [ ] **Criterio**: nuovo post parte da archetype corretto, appare nel tree

## M9 — Polish e onboarding

- [ ] Empty states curati
- [ ] Settings dell'app (path Hugo binary, lingua UI, telemetria opt-in)
- [ ] Documentazione `.hugoeditor/schema.json` e `.hugoeditor/theme-schema.json`
- [ ] Icone custom + branding
- [ ] **Criterio**: un editor non-tecnico apre un sito esistente e modifica un post senza vedere YAML
