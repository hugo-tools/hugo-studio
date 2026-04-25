# Hugo Studio — Decisioni tecniche

Le righe seguono la regola "una decisione, un razionale di una riga". Aggiornare quando una scelta cambia.

## Bootstrap (M0 — 2026-04-25)

- **Tauri 2 (latest stable, react-ts template)** — vincolato dal prompt; usato `create-tauri-app` come baseline e poi modificato in-place.
- **Rust toolchain `rust:1-bookworm` (1.95.x)** — l'edizione 2024 è richiesta da deps transitive di Tauri 2 (`zbus_names`); 1.83 non basta, 1.85 è il minimo, prendiamo l'ultima stable per non doverlo bumpare a ogni rilascio.
- **Node 20 LTS** — minimo richiesto dai tooling moderni (Vite 7, ESLint 9), LTS evita soprese in CI.
- **Docker per dev/build** — l'host (Ubuntu 18.04, Node 16, no Rust) non può buildare Tauri 2; Docker dà parità riproducibile. CI invece resta su runner nativi mac/linux/win per validare i target reali.
- **React 19** — tirato dal template; nessun motivo di downgrade, shadcn/ui e tutte le altre lib che useremo lo supportano.
- **Tailwind v3 (3.4)** — v4 è uscita ma molti adapter (incluse alcune ricette shadcn/ui) sono ancora in transizione; v3 ha tooling più maturo per Vite e per `prettier-plugin-tailwindcss`. Da rivalutare in M9.
- **shadcn/ui (style "new-york", baseColor "slate")** — preferito da prompt; "new-york" rende meglio in app dense come la nostra (editor + form). Componenti aggiunti on-demand: per M0 solo `Button`.
- **Zustand per stato globale** — vincolato dal prompt; nessuna alternativa valutata.
- **TanStack Query per orchestrare i comandi Tauri** — vincolato dal prompt; pattern: ogni `commands.foo()` viene chiamato dentro `useQuery` / `useMutation` per cache, invalidation e optimistic updates gratuiti.
- **react-hook-form + zod** — vincolato dal prompt; useremo `@hookform/resolvers/zod` per validazione.
- **CodeMirror 6 invece di Monaco** — Monaco pesa ~5 MB minified e va inserito tramite worker; CodeMirror 6 è modulare (~150-250 KB nel nostro caso), tree-shakeable, ESM-native, e ha bindings React maturi (`@uiw/react-codemirror`). Per un editor il cui focus è il body markdown (non IDE-feel) CodeMirror è la scelta giusta.
- **Specta + tauri-specta per codegen Rust↔TS** — preferito da prompt; rispetto a `ts-rs` espone anche un client tipato per i comandi (`commands.healthCheck()`), non solo i tipi. La generazione gira a `cargo build` in debug; il file `src/lib/tauri/bindings.ts` è committato come stub per permettere il typecheck del frontend prima della prima `tauri dev`.
- **Versioni RC pinnate per specta + tauri-specta** — la 2.0 è ancora rc; pin esatti su `Cargo.toml` per evitare bump silenziosi che cambiano l'API del builder.
- **Path alias `@/` → `src/`** — convenzione shadcn/ui; configurato in `tsconfig.json` e in `vite.config.ts`.
- **ESLint 9 flat config + Prettier 3** — flat config è il futuro; tenuti separati (eslint = correctness, prettier = style) per non litigare. `prettier-plugin-tailwindcss` ordina le classi.
- **`bindings.ts` versionato, escluso da prettier/eslint** — è auto-generato; ignorarlo evita che i tooling protestino su uno spaziamento "non canonico" del generator.
- **Volumi nominati per `target/`, `node_modules/`, cache cargo** — tengono build pesanti fuori dal bind mount per non saturare il filesystem dell'host e per accelerare le run successive.
- **`run() -> ()` con `try_run() -> Result<…>`** — il prompt vieta `unwrap`/`expect` in production. Usiamo il classico pattern "wrap fallible logic in `try_run`, log + exit non-zero alla radice".
- **Hugo binary non bundlato in M0** — è un task di M6. Per ora l'app non spawna `hugo`; basta che apra e parli col backend.
- **Nessuna icona custom in M0** — restano gli asset Tauri di default; le sostituiremo in M9 (polish).
