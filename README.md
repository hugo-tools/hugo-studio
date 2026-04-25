# Hugo Studio

Editor desktop multi-site per [Hugo](https://gohugo.io/) — Tauri 2 + React.
Stato: **M0 (bootstrap)**. Le milestone successive sono in [`ROADMAP.md`](./ROADMAP.md).

## Sviluppo

Tutto il tooling gira **dentro Docker** (vedi `.docker/Dockerfile.dev` e `docker-compose.yml`). L'host non ha bisogno di Rust o Node.

### Comandi rapidi

```bash
make image        # builda l'immagine dev (una volta sola, poi solo se cambia il Dockerfile)
make shell        # entra in una shell nel container
make dev          # Vite dev server (frontend solo, http://localhost:1420)
make tauri-dev    # Tauri dev (richiede X server / Xvfb sull'host)
make fe-build     # build di produzione del frontend
make cargo-check  # type-check del backend Rust
make fmt          # rustfmt + prettier
make lint         # clippy + eslint
make clean        # giù i container e i volumi nominati (cache)
```

### Layout

```
hugo-studio/
├── .docker/Dockerfile.dev    # immagine dev/build
├── docker-compose.yml
├── Makefile                  # wrapper su `docker compose`
├── package.json              # frontend (React 19 + Vite + TS)
├── src/                      # frontend
│   ├── app/                  # entry: providers, routing
│   ├── components/ui/        # shadcn/ui primitives
│   ├── lib/tauri/            # client tipato generato da specta
│   └── lib/utils.ts          # helpers (cn, ecc.)
└── src-tauri/                # backend Rust
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs
    │   ├── lib.rs            # entry: Builder + commands registry
    │   └── commands/         # un file per comando (M1+ ne aggiungerà molti)
    └── tauri.conf.json
```

## Decisioni tecniche

Vedi [`DECISIONS.md`](./DECISIONS.md). Ogni scelta è una riga con razionale.

## Release

I binari sono prodotti dalla GitHub Action `release` quando si pusha un tag `v*` (es. `v0.1.0`).
Vedi [`.github/workflows/release.yml`](./.github/workflows/release.yml) — produce installer DMG (macOS), AppImage + .deb (Linux) e NSIS .exe (Windows), allegandoli alla GitHub Release in draft.

### macOS — primo avvio

Le build macOS sono _ad-hoc signed_ ma **non notarizzate** (la notarizzazione richiede un Apple Developer ID a pagamento). Quando scarichi il `.dmg` dalla Release, macOS aggiunge il flag `com.apple.quarantine` e al primo lancio mostra:

> "Hugo Studio" non può essere aperto perché Apple non può verificare l'assenza di software malevolo.

Due modi per autorizzarla una sola volta:

1. **GUI**: tasto destro sull'app in Finder → `Apri` → conferma nel dialog. Le aperture successive non chiedono più nulla.
2. **Terminale**, dopo aver trascinato l'app in `/Applications`:
   ```sh
   xattr -dr com.apple.quarantine "/Applications/Hugo Studio.app"
   ```

Se compare invece **"è danneggiato"** (anziché il messaggio sopra) significa che la firma ad-hoc è andata persa: usa il comando `xattr` qui sopra.

## Contribuire

- Conventional Commits (`feat:`, `fix:`, `chore:`, `refactor:`, `test:`, `docs:`)
- `main` protetto, lavorare su feature branch
- PR: titolo che richiama la milestone (`feat(M2): config_get command`)
- `make lint` deve passare prima di aprire la PR
