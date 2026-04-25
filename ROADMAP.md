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

- [x] Tipi `Workspace`, `SiteRef`, `Site`, `SiteId` (UUID) in Rust con codegen TS via specta
- [x] Comandi: `workspace_list_sites`, `workspace_active_site_id`, `workspace_add_site`, `workspace_remove_site`, `workspace_rename_site`, `workspace_set_active`, `workspace_clear_active`, `site_detect`
- [x] Persistenza workspace in `app_data_dir/workspace.json` (write atomico via tmp+rename)
- [x] UI: schermata iniziale con elenco siti, "Add site" (file picker via `tauri-plugin-dialog`), "Open" e "Remove" con AlertDialog di conferma
- [x] Detection `hugo.{toml,yaml,yml,json}` / `config.{toml,yaml,yml,json}` / `config/_default/` (priorità nell'ordine; root single-file vince sempre)
- [x] Plugin dialog registrato sia su Cargo che su capabilities (`dialog:default`)
- [x] Specta-typescript con header `@ts-nocheck` + `eslint-disable` sui `bindings.ts` autogenerati
- [x] AppError tipizzato con `thiserror` e discriminator JSON, wrapper TS `tauri.*` che fa `unwrap` del `Result<T, AppError>`
- [x] Bin `gen-bindings` per rigenerare i tipi TS senza display (`make gen-bindings`)
- [x] Test unit: 7 detect + 3 persistence + 1 health = 11 passed
- [ ] **Criterio**: aggiungo 2 siti, switch tra loro, l'app ricorda la lista al riavvio (validabile solo eseguendo il binary su un host con display)

## M2 — Lettura della config del sito

- [x] Parser multi-formato con preservazione: `toml_edit` (full preservation) per TOML, `serde_json` con `preserve_order` per JSON, line-patching regex-based per YAML scalari top-level (con fallback a re-serialize per nested)
- [x] Cascata `config/_default/`: file `hugo.*` mappato sulla root; gli altri file (`params.*`, `menus.*`, ecc.) mountati sotto la chiave omonima allo stem
- [x] Comandi `config_get` / `config_save` con FS sandbox via lookup nel workspace
- [x] UI: pannello "Site Settings" con form react-hook-form + zod per i campi noti (title, baseURL, languageCode, defaultContentLanguage, theme, paginate, enableEmoji, enableRobotsTXT) + sezione "Advanced" read-only (JSON viewer)
- [x] Save di un campo invariato non riscrive il file su disco (early-return per byte-identical)
- [x] Test unit: 25 totali (era 11) — round-trip byte-identico per i 3 formati, modifica baseURL = 1 sola riga di diff, comments survivability, save su `_default/` tocca solo il file giusto
- [x] Environment override (config/development/, config/production/): rimandato a quando arriverà come richiesta utente — Hugo lo permette ma in M2 non è documentato come bloccante; aprirò una M2.x se serve
- [ ] **Criterio**: aprire la config, modificare baseURL, salvare → diff = 1 sola riga (validato dai test, da provare end-to-end sull'app reale)

## M3 — Content tree e lettura contenuti

- [x] Scansione `content/` con classificazione `Section` / `BranchBundle` / `LeafBundle` / `SinglePage` (estensioni: `.md`, `.markdown`, `.html`, `.htm`)
- [x] Detection multilingua per §6.6: `Mono` / `Filename` / `Directory` (driven da `defaultContentLanguageInSubdir` + presenza di subdir per i lang code)
- [x] Front-matter peek minimale per popolare title/draft/date senza full deserialize (TOML/YAML/JSON con tolleranza BOM)
- [x] Comando `content_list` con cache TanStack Query (queryKey scoped per `site.id`)
- [x] UI: file tree gerarchico costruito client-side dalla flat list, icone per kind (Layers/Package/Folder/FileText), badge draft, badge "+ lang" / "⚠ missing"
- [x] LanguageSwitcher: select compatto in toolbar tree, filtra per lingua attiva
- [x] File watcher `notify` debounced 200ms (poll loop a 50ms + finestra di silenzio 200ms), ignora `public/`, `resources/`, `node_modules/`, `.git/`, `.hugo_build.lock`, `.DS_Store`
- [x] Watcher gestito dal lifecycle di `workspace_set_active` / `workspace_clear_active`; emette `site:changed` Tauri event; frontend invalida `["content", id]` + `["config", id]`
- [x] Test unit: 53 totali (era 25) — classify (6), language (7), FM peek (6), scan (4), watcher ignore (4)
- [ ] **Criterio**: tree popolato per Hugo Coder / PaperMod, modifica esterna riflessa in <1s (validabile solo runtime su un sito reale)

## M4 — Editor di contenuto (cuore del prodotto)

- [x] Document codec: split FM/body con preservazione layout (CRLF/LF, BOM, blank lines), riusa i `ConfigCodec` di M2 sul blocco FM (TOML full preservation, YAML line-patching scalari, JSON preserve_order)
- [x] Save atomico (tmp + rename), early-return byte-identical quando il file non è davvero cambiato
- [x] Schema inference: 21 campi standard Hugo curati con tipi nativi (Basic / Taxonomy / Routing / Schedule / Order / Rendering) + inferenza dei campi custom dalla section + autocomplete enumValues per Tags/StringArray
- [x] Comandi `content_get` / `content_save` con sandbox `resolve_under_root` (PathTraversal vivo) e detection automatica della section anche per strategia Directory (xx / xx-yy)
- [x] FrontMatterForm schema-driven: string/text/number/boolean/date/dateTime/tags/stringArray/json — raggruppato per `group`
- [x] TagsInput chip-style con suggestion list, navigabile da tastiera (↑/↓/Enter/Tab), backspace toglie l'ultima
- [x] BodyEditor CodeMirror 6 con `@codemirror/lang-markdown` + line-wrapping, riempie il pane
- [x] EditorView: split form|body, dirty tracking, Save flash, X chiude la selezione (torna alla settings)
- [x] Click sul tree apre l'editor; il riquadro destro alterna Settings ↔ Editor in base a `selection`
- [x] Test unit: 68 totali (era 53) — 9 round-trip nuovi su `document.rs` (Yaml/Toml/Json + body-only + CRLF + BOM) e 4 su `schema.rs` (standard fields, classify, humanise, inference + tag autocomplete) + sandbox top_section
- [ ] Override `.hugoeditor/schema.json`: rimandato (M9 polish — scelta architetturale: l'inferenza copre il 90% dei casi senza chiedere lavoro agli autori dei temi)
- [ ] Side-by-side preview del Markdown renderizzato: rimandato a M6 quando arriva `hugo server` (sarà la stessa cosa, gratis)
- [ ] **Criterio**: modifica title/tags/draft + body → diff = solo le modifiche; tags con autocomplete (validato dai test sul codec; verifica visiva sull'app reale ancora da fare)

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
