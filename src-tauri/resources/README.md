# Resources

This directory is bundled into the installer as `resources/**`.

At runtime, the app stores shared runtime files in its Tauri app-data directory:

- `runtime/` — bundled Node.js runtime downloaded on first run
- `dependencies/pnpm/` — bundled pnpm used for deterministic Harness installs
- `dependencies/dsh/` — the official `@deepseek-ai/dsh` version matching a DeepSeek GitHub Release tag
- `logs/` — application and Harness service logs
- `.store.dat` / `.store.dev.dat` — release/debug desktop settings

The desktop does not bundle, recommend, install, update, remove, or repair third-party Harness plugins. The Harness dependency tree comes from the official DeepSeek release version only.

`$DSH_HOME` follows the official convention: `${DSH_HOME:-$HOME/.dsh}` in release builds and `~/.dsh.dev` in debug builds. Existing legacy data migration remains handled by `service/migrate` for release builds.
