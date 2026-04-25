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

- [x] Cascata schema: `themes/<n>/.hugoeditor/theme-schema.json` (Manifest) → `themes/<n>/{config.{toml,yaml,json},theme.toml}` con `[params]` (Defaults) → introspezione dei `params` correnti del sito (Inferred)
- [x] Comandi `theme_get` / `theme_save_params` che riusano `cascade::save` (M2) — i params editati atterrano nel file giusto sia in single-file (`hugo.toml`) sia in `_default/params.toml`
- [x] Effective params: nel modo Defaults, i valori del sito vincono sui default del tema (form si apre con quello che l'utente ha impostato)
- [x] ThemeInfo gestisce theme array (component themes layered → primary preso, secondari TODO)
- [x] Pannello "Theme Settings" riusa `FrontMatterForm` (stessa shape `FieldDef` di M4); badge fonte schema con icona + tooltip esplicativo (Manifest / Theme defaults / Inferred)
- [x] SiteShell con Tabs `[Site | Theme]` quando non c'è una selezione editor
- [x] Documentato il formato `theme-schema.json` nel README per gli autori di temi
- [x] Test unit: 74 totali (era 68) — Manifest wins, Defaults inferred, Inferred fallback, missing theme dir graceful, save su hugo.toml, save su `_default/params.toml`
- [ ] **Criterio**: PaperMod, modifica `params.author`, salvataggio corretto in `hugo.toml` (validato dai test cargo; verifica visiva sull'app reale ancora da fare)

## M6 — Live preview

- [x] Locate Hugo via `HUGO_STUDIO_HUGO_PATH` → `which::which("hugo")` (sidecar bundlato vero rimandato a M9; documentato nel placeholder dell'UI)
- [x] Spawn `hugo server -D --bind 127.0.0.1 --port <free> --navigateToChanged --disableFastRender --source <root>` con `tokio::process` e `Child::kill_on_drop(true)` — niente zombi anche se l'app crasha
- [x] Pump stdout/stderr line-by-line; tail in memoria 50 righe per arricchire `preview:error`
- [x] Eventi Tauri `preview:log` / `preview:ready` / `preview:error` / `preview:exited`
- [x] Lifecycle agganciato a `workspace_set_active` / `workspace_clear_active` (e in M3 dropping `replace_preview` ferma il vecchio prima di partire il nuovo)
- [x] Comandi `preview_start` (async) / `preview_stop` / `preview_status`
- [x] WebView pane con iframe sull'URL ricevuto + reload button + status dot (idle / starting / running / error)
- [x] Console collassabile (cap 500 righe, auto-scroll bottom-aware, clear button)
- [x] SiteShell layout 3-colonne `[280px tree | 1fr center | minmax(380px,1fr) preview]` quando preview aperta
- [x] Hugo extended in dev Docker image (build/test in container)
- [x] Test unit: 79 totali (era 74) — parse "Web Server is available at" (3 forme), pick_free_port, locate_hugo via env var, EnvGuard helper
- [ ] **Criterio**: refresh <1s; nessun processo Hugo zombie alla chiusura (validato dai test e da `kill_on_drop`; verifica visiva richiede l'app reale + Hugo sul host)

## M7 — Asset management

- [x] `assets/` module: `AssetKind` (Image/Script/Style/Document/Other), `AssetContext { Bundle{contentId} | Static{subpath} | Assets{subpath} }`, `AssetRef` with `relativeLink` already shaped for the markdown editor
- [x] Backend operations: `import` (collision-safe `-1`/`-2`/… suffix), `list` (bundle siblings only, drops dotfiles), `delete` (refuses index files)
- [x] Sandbox check on every IO touch (reuses `PathTraversal` AppError variant from M3)
- [x] Tauri commands `asset_import`, `asset_list`, `asset_delete`
- [x] Frontend `AssetImportDialog` with smart default per active content kind, custom subpath inputs for static / assets
- [x] OS file drag-drop captured via `getCurrentWebview().onDragDropEvent`; routes through the dialog → `assetImport` → `editor.insertAtCursor` for each file
- [x] `BodyEditor` exposes an imperative `insertAtCursor` via `forwardRef` so multiple call sites (drop, sidebar click) can inject markdown without remounting CodeMirror
- [x] `BundleAssetsPanel` sidebar (only when editing a bundle) — list, click-to-insert, delete with hover-revealed trash button
- [x] Specta `bigint(BigIntExportBehavior::Number)` so `u64` file sizes serialise to TS `number`
- [x] Image thumbnails via `asset://` scheme: deferred to M9 (needs scoped `app.security.assetProtocol`)
- [x] Test unit: 87 totali (era 79) — import to bundle / static / assets, collision suffix, traversal rejection, list filters, delete refuses index
- [ ] **Criterio**: drag in un leaf bundle → file copiato + link inserito al cursore (validato dai test e dal flow UI; verifica visiva sull'app reale ancora da fare)

## M8 — Creazione contenuti e archetypes

- [x] `content/archetype.rs` enumerates `archetypes/<name>.md` + `archetypes/<name>/index.md` (single page + leaf-bundle archetypes), sorts `default` first
- [x] `resolve_template` walks `<requested>` → `<section>` → `default` → built-in
- [x] Minimal Go-template substitution (`.Name`, `.Title`, `.Slug`, `.Section`, `.Date`, plus the canonical `{{ replace .Name "-" " " | title }}` and `_` variant) — anything else stays in place for the user to edit
- [x] `content/create.rs` — slug + section sanitisation, language-aware target path (filename `.lang` suffix vs directory `<lang>/` prefix), atomic write, refuses to overwrite, copies sibling files of bundle archetypes verbatim
- [x] Commands `content_archetypes` + `content_create`
- [x] `NewContentDialog` from the ContentTree header — title → slug live derivation, section autocomplete, archetype dropdown (`(auto)` default), language dropdown only when site is multilingual; on success opens the new content in the editor
- [x] Test unit: 107 totali (era 93)
- [x] **Criterio**: nuovo post parte da archetype corretto, appare nel tree (validato dai test, verifica visiva ancora da fare sull'app reale)

## v0.9.0 UX & git extras (shipped together)

- [x] CodeMirror body editor reads its colors from the `--background`/`--foreground` Tailwind variables → dark mode no longer renders white-on-white
- [x] Content tree sort selector (Name / Newest first / Oldest first); folders still grouped first
- [x] Git: `PullStrategy { FastForward, ForceReset }` — backend `git_pull` takes a strategy; backend `git_stash_save` + `git_stash_pop`
- [x] GitPanel: `Stash`, `Pop stash`, `Force pull` (auto-stashes dirty working tree first, confirms before discarding local commits)

## M9 — Polish e onboarding

- [ ] Empty states curati
- [ ] Settings dell'app (path Hugo binary, lingua UI, telemetria opt-in)
- [ ] Documentazione `.hugoeditor/schema.json` e `.hugoeditor/theme-schema.json`
- [ ] Icone custom + branding
- [ ] **Criterio**: un editor non-tecnico apre un sito esistente e modifica un post senza vedere YAML
