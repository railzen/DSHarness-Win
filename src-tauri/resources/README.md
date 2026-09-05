# Resources

This directory is bundled into the installer as `resources/**`.

- `pnpm/pnpm-<version>.tgz` — the official pnpm tarball, fetched at build time by
  `scripts/fetch-pnpm.ts` (runs before `pnpm build`; version and SHA-256 come from
  `src/config/constants.rs`). Git-ignored. On first run the app extracts it offline
  instead of downloading pnpm; if the file is missing it falls back to downloading.

At runtime, the app stores shared runtime files in its Tauri app-data directory:

- `runtime/` — bundled Node.js runtime downloaded on first run (skipped when a compatible system Node.js is found)
- `dependencies/pnpm/` — bundled pnpm used for deterministic Harness installs
- `dependencies/dsh/` — the official `@deepseek-ai/dsh` version matching a DeepSeek GitHub Release tag
- `logs/` — application and Harness service logs
- `.store.dat` / `.store.dev.dat` — release/debug desktop settings

The desktop does not bundle, recommend, install, update, remove, or repair third-party Harness plugins. The Harness dependency tree comes from the official DeepSeek release version only.

`$DSH_HOME` follows the official convention: `${DSH_HOME:-$HOME/.dsh}` in release builds and `~/.dsh.dev` in debug builds. Existing legacy data migration remains handled by `service/migrate` for release builds.
