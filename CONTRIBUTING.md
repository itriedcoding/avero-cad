# Contributing

Thanks for taking the time to improve ER:LC CAD. This project integrates with a live game API and
executes real server commands, so the bar for correctness and safety is high.

## Ground rules

- **No fabricated data.** Do not add mock API responses, placeholder statistics, fake releases or
  buttons that do nothing. If the ER:LC API does not expose something, document the limitation and
  implement the closest legitimate behaviour.
- **Business logic belongs in Rust.** The Tauri shell (`src-tauri`) only marshals arguments; the
  frontend only renders and collects input. Anything touching the API, database, permissions or
  credentials goes in `crates/erlc-cad-core`.
- **Permissions are enforced server-side.** Every new IPC command must authenticate the token and
  require a permission key. Never trust a frontend check.
- **Never commit secrets.** No server keys, tokens or captured request headers.

## Development setup

```bash
npm install
cargo test -p erlc-cad-core     # backend: works on Linux/macOS/Windows
npm run test                    # frontend unit tests
npm run build                   # typecheck + production bundle
```

Running the desktop app requires Windows (or macOS) with the Tauri prerequisites:

```bash
npm run tauri dev
```

## Repository layout

| Path                     | Purpose                                                        |
| ------------------------ | -------------------------------------------------------------- |
| `crates/erlc-cad-core`   | API client, database, permissions, services, security          |
| `src-tauri`              | Tauri window, app state, IPC command registry                  |
| `src`                    | React frontend: pages, components, hooks, state, typed IPC     |
| `migrations`             | SQL schema, indexes, seed roles/permissions/command policies   |
| `.github/workflows`      | CI (tests on Linux + Windows compile check) and tagged releases |

## Adding a feature

1. Add or extend the model in `crates/erlc-cad-core/src/models.rs`.
2. Implement the behaviour in the relevant `services/` module, returning `AppResult<_>`.
3. Add a thin `#[tauri::command]` in `src-tauri/src/commands/` and register it in
   `src-tauri/src/lib.rs`; keep argument names snake_case in Rust (the frontend sends camelCase).
4. Add the typed wrapper to `src/lib/api.ts` and mirror any new view type in `src/types.ts`.
5. Build the UI from the existing primitives (`Panel`, `DataTable`, `Modal`, `Field`, `Badge`) so the
   monochrome design language stays consistent.
6. Cover the logic with a unit test in the same module (`#[cfg(test)]`), or with a Vitest test for
   pure frontend helpers.

## Database changes

Never edit an applied migration. Add `migrations/000N_description.sql`, register it in the migration
list inside `crates/erlc-cad-core/src/db/mod.rs`, and add a test that opens an in-memory database and
asserts the new shape. Keep indexes and foreign keys explicit.

## Code style

- Rust: `cargo fmt` and `cargo clippy` before opening a pull request; prefer explicit error mapping
  over `unwrap` outside tests.
- TypeScript: strict mode is on (`noUnusedLocals`, `noUncheckedIndexedAccess`); avoid `any` and
  non-null assertions unless genuinely provable.
- CSS: Tailwind utilities with the existing design tokens (`surface`, `line`, `ink`, `muted`, `faint`,
  `canvas`). No new colours beyond the single destructive red.

## Testing a change against a real server

Live integration testing needs your own ER:LC private server and server key. A useful checklist:

1. Save the key in **API Configuration** and run the connection test.
2. Confirm dashboard counts match the in-game player list.
3. Open **Players**, verify an avatar loads and matches the username.
4. Open **Live Map** and check that markers align with postal codes (adjust the scale if not).
5. Create a CAD call, assign a unit, attach a player, change status, close it.
6. As a moderator account, try a command that is not in **Command Policies** — it must be refused
   before any HTTP request.
7. Run an allowed `:h` command and confirm the response and audit entry.
8. Disable the moderator account from another session and confirm its next call is rejected.

Never test with a production community server; use a test server and test accounts.

## Commits and pull requests

- Keep commits focused and describe the reason for the change, not only the diff.
- Include the checks you ran in the pull request description.
- Note any behaviour that could not be verified locally (for example Windows-only packaging).
