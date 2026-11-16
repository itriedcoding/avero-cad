# ER:LC CAD

A Windows desktop **CAD (Computer-Aided Dispatch) and server management application** for
[Emergency Response: Liberty County](https://policeroleplay.community/) private servers, built with
Rust, Tauri 2, React and SQLite.

It talks to the official ER:LC Private Server API (v2 for live data and commands, v1 for the ban
list) and to the public Roblox Users and Thumbnails APIs for avatars. All network calls,
authentication, permission checks, credential handling and command execution happen in Rust; the
React frontend is a rendering and input layer only.

The interface is deliberately monochrome: near-black surfaces, white accents, and a single red for
destructive actions and errors.

---

## Contents

- [Features](#features)
- [Architecture](#architecture)
- [Requirements](#requirements)
- [Getting started](#getting-started)
- [Building a Windows installer](#building-a-windows-installer)
- [First run and API configuration](#first-run-and-api-configuration)
- [Permissions](#permissions)
- [Security model](#security-model)
- [Known API limitations](#known-api-limitations)
- [Testing](#testing)
- [Releases](#releases)
- [License](#license)

---

## Features

**Operations**

- **Dashboard** — live player counts, police/fire/civilian breakdown, active vehicles, emergency
  calls, moderation activity, API latency, connection state, last sync time and a player-count
  sparkline built from locally recorded metric points (never fabricated history).
- **CAD Dispatch** — create, edit, assign, progress and close incidents; attach players and units;
  priority levels; full call history. Unit roster with callsigns, teams and configurable statuses
  (available, on duty, en route, on scene, transporting, unavailable, off duty).
- **Live Map** — official 5355 × 5355 ER:LC map imagery, live player positions and emergency-call
  coordinates from API v2, zoom and pan, marker selection, postal/street/building details.
- **Players** — live roster with avatars, team, callsign, permission level and location.
- **Player Database** — search by Roblox username, user ID, callsign or current presence; full
  profile with avatar (headshot and full-body), Roblox account details, moderation history,
  internal bans, staff notes and ban status.
- **Moderation** — kick, ban and unban through real ER:LC commands, with reason capture,
  confirmation, per-action result and moderator attribution. Internal CAD bans are clearly separate
  from server bans.
- **Banned Players** — live ER:LC ban list (read-only from the API) and internal CAD bans with lift
  reasons.
- **Server Commands** — permission-checked command terminal with history, API response, HTTP status,
  command ID, latency, owner-managed command policies and confirmation for destructive commands.
- **Vehicles** — spawned vehicles with plate, model, owner, livery and colour.
- **Emergency Calls** — live 911 feed plus incidents mirrored into CAD dispatch.
- **Server Logs** — join, leave, kill, command and moderator-call logs, plus the join queue.

**Administration**

- **Staff Management** — staff directory, role definitions, per-account permission overrides
  (inherit / grant / deny), account enable/disable and password resets.
- **Audit Logs** — append-only record of permission changes, moderation, commands, credential and
  configuration changes, filterable by category and free text.
- **API Configuration** — server key management, connection testing with latency and rate-limit
  headers, key fingerprint, DPAPI storage indicator.
- **Settings** — polling interval, startup page, auto-connect, destructive confirmations, compact
  tables, map image and calibration, session timeout, notification preferences, backups, database
  statistics.

## Architecture

```
Cargo workspace
├── crates/erlc-cad-core/     all business logic; compiles and is tested on any OS
│   ├── api/                  ER:LC HTTP client, rate limiting, error mapping
│   ├── db/                   SQLite schema access, migrations, queries, backup
│   ├── security/             DPAPI storage (Windows) + Argon2id passwords
│   ├── services/             auth, snapshot, command, moderation, players, dispatch, staff, settings
│   ├── permissions.rs        permission catalogue and access checks
│   ├── roblox.rs             Roblox user + thumbnail lookups, disk/memory cache
│   └── models.rs             serialisable view models shared with the frontend
└── src-tauri/                thin Tauri shell: window, state, 63 IPC commands
    ├── src/lib.rs            app setup, auto-backup loop, command registry
    ├── src/commands/         one module per domain, argument marshalling only
    ├── tauri.conf.json       window, CSP, NSIS + MSI bundle configuration
    └── capabilities/         default capability set
src/                          React + TypeScript frontend (Vite, Tailwind CSS v4)
migrations/                   versioned SQL schema with seeds
```

Design rules enforced throughout:

- Every IPC command authenticates its token and re-loads permissions from the database on each call.
- The frontend may hide controls it cannot use, but the backend always re-checks the permission.
- The ER:LC server key never crosses the IPC boundary after being saved; only last four characters
  and a SHA-256 fingerprint are returned.
- Command execution validates the `:` command shape, enforces the API's five-second cadence, records
  the attempt, and never retries a POST.

## Requirements

**To run a build**

- Node.js 22+ and npm
- Rust 1.85+ (stable) with the `x86_64-pc-windows-msvc` target on Windows
- Windows 10/11 with WebView2 (preinstalled on current Windows builds)
- MSVC build tools (Visual Studio Build Tools with "Desktop development with C++") for the Windows
  installer, plus WiX (bundled by the Tauri CLI) for MSI output

**To use the application**

- A Windows 10/11 x64 machine
- An ER:LC private server with API access enabled, and a server key created in the server's API
  settings

## Getting started

```bash
npm install

# Web frontend only (no Rust, no IPC — screens render, data calls fail)
npm run dev

# Full desktop application (Windows or macOS with the matching Rust toolchain)
npm run tauri dev
```

The Tauri dev server expects Vite on `http://localhost:1420`, which `npm run dev` provides.

Backend-only development and tests do not need a desktop toolchain:

```bash
cargo test -p erlc-cad-core
```

## Building a Windows installer

On Windows:

```powershell
npm ci
npm run build          # typecheck + Vite production build
npm run tauri build -- --bundles nsis,msi
```

Outputs:

```
src-tauri/target/release/bundle/nsis/ERLC-CAD_<version>_x64-setup.exe
src-tauri/target/release/bundle/msi/ERLC-CAD_<version>_x64_en-US.msi
```

The same build runs automatically in GitHub Actions on a version tag (see [Releases](#releases)).
The MSI/NSIS installers require the WebView2 runtime, which the Tauri bundler bootstraps when it is
missing.

## First run and API configuration

1. Launch the application. On a fresh database you are asked to create the **Server Owner** account.
   Passwords are hashed with Argon2id; there is no default account and no recovery backdoor.
2. Open **API Configuration**.
3. Paste your ER:LC **server key** (created in the ER:LC private server API settings) and keep the
   base URL `https://api.erlc.gg`.
4. Press **Test entered key**. A successful test shows server name, player counts, latency and the
   API's rate-limit headers. Errors are shown verbatim (invalid key, missing permissions, server
   offline).
5. Save the key. It is encrypted with Windows DPAPI for your Windows user and stored outside the
   database. Only the last four characters are ever displayed again.
6. Create staff accounts in **Staff Management**, choose their roles, and add per-account overrides
   where a role is almost right.

### Command policies

ER:LC commands are disabled until the owner authorizes them. Go to **Server Commands → Policies** and
add each command you want to permit, for example:

| Command  | Minimum permission   | Confirmation |
| -------- | -------------------- | ------------ |
| `:h`     | `commands.execute`   | no           |
| `:kick`  | `moderation.kick`    | yes          |
| `:ban`   | `moderation.ban`     | yes          |
| `:unban` | `moderation.unban`   | yes          |

Any command not present in the policy table is refused by the Rust backend before an HTTP request is
made.

## Permissions

Roles are `owner`, `admin` and `moderator`, and each role holds a set of individual permission keys.
Per-account overrides can grant or deny a single key without changing the role.

| Area       | Keys                                                                                  |
| ---------- | ------------------------------------------------------------------------------------- |
| Live data  | `dashboard.view`, `map.view`, `players.view`, `vehicles.view`, `emergency.view`, `logs.view` |
| Dispatch   | `dispatch.view`, `dispatch.create`, `dispatch.update`, `dispatch.close`, `dispatch.units` |
| Moderation | `moderation.view`, `moderation.kick`, `moderation.ban`, `moderation.unban`, `moderation.internal` |
| Players    | `players.notes.view`, `players.notes.manage`                                            |
| Commands   | `commands.view`, `commands.execute`, `commands.policies.manage`                          |
| Staff      | `staff.view`, `staff.manage`, `roles.manage`                                            |
| System     | `audit.view`, `api.configure`, `settings.manage`, `backup.manage`                        |

The owner role is protected: its permissions cannot be reduced, and owner-only configuration stays
owner-only. Local permissions never bypass ER:LC's own authorization — the server key still decides
what the API will accept.

## Security model

- **Credentials** — Windows DPAPI (`CryptProtectData`, current-user scope) encrypts the server key;
  the file lives in the application data directory. The database stores only metadata (base URL,
  last four characters, fingerprint, verification time). Windows builds never fall back to
  plaintext: if DPAPI fails, the key is not persisted. (Non-Windows development builds use a
  clearly labelled local file, which no installer ships.)
- **No secrets in logs** — request/response logging redacts `server-key` headers and the token that
  follows them.
- **Sessions** — random 256-bit tokens, stored hashed, 12-hour absolute lifetime plus a configurable
  idle timeout; every IPC call re-checks the account is still active.
- **Failed sign-ins** are counted and lock the account temporarily.
- **Input validation** — usernames, reasons, callsigns, command text, paths and URLs are validated
  and length-limited in Rust. SQL uses prepared statements with escaped `LIKE` patterns.
- **Commands** — only `:`-prefixed alphanumeric ER:LC commands are accepted; newlines and shell
  metacharacters are rejected, so no Windows command can be smuggled through the console.
- **CSRF/network surface** — the window loads only bundled assets under a strict CSP and exposes no
  HTTP listener of its own.

See [SECURITY.md](SECURITY.md) for reporting and the full threat model.

## Known API limitations

These are properties of the ER:LC API, not of this application. Nothing is simulated to hide them.

- **No ban-write endpoint.** Ban and unban are executed as the documented `:ban` / `:unban` server
  commands, with the API's response recorded. The live ban list comes from `GET /v1/server/bans` and
  is read-only.
- **No vehicle positions.** The API reports spawned vehicles (model, plate, owner, colour, livery) but
  not coordinates, so vehicles are listed rather than plotted.
- **No explicit leave-log endpoint.** Leave events are derived from join-log entries whose `joined`
  flag is false.
- **Undocumented map scale.** ER:LC publishes the map size (5355 × 5355) and axis directions but not
  the world-unit-to-pixel ratio, so the scale is an operator-calibrated setting (default 3 px per
  unit) with an explanation in the UI. No invented coordinates are ever drawn.
- **Query flags are server-side.** Logs, locations, vehicles and calls only appear when the key
  carries the matching flags and the server reports them. Empty panels mean "the API sent nothing".
- **Rate limit.** One command per five seconds per server; CAD enforces this locally.
- **Roblox thumbnails** may 404 for accounts without a rendered avatar; the UI falls back to initials
  instead of showing another player's image.

## Testing

```bash
cargo test -p erlc-cad-core     # 82 unit tests: API, permissions, DB, services, security
npm run test                    # 17 frontend unit tests (formatting, status mapping)
npm run build                   # TypeScript typecheck + production bundle
```

The Rust crate covers the API client's error mapping and rate limiting, command validation, the
permission catalogue, SQLite migrations and queries (including backup/restore and permission
overrides), moderation and dispatch services, session handling, Argon2id hashing and DPAPI-backed
secret storage. Frontend tests cover the pure formatting and status helpers.

Live integration testing needs a real server key and a running ER:LC server; see
[CONTRIBUTING.md](CONTRIBUTING.md) for a checklist. End-to-end Windows packaging is exercised by the
release workflow, which uploads the installers it produces.

## Releases

`.github/workflows/release.yml` builds on `windows-latest` when a `v*` tag is pushed, runs the tests,
invokes `tauri build` for NSIS and MSI, and attaches both installers to a GitHub Release:

```
ERLC-CAD_<version>_x64-setup.exe
ERLC-CAD_<version>_x64_en-US.msi
```

To cut a release:

```bash
# bump "version" in package.json and src-tauri/tauri.conf.json, then
git tag v1.0.0
git push origin v1.0.0
```

Builds are **unsigned** unless you configure code-signing secrets. Windows SmartScreen will warn
about an unsigned installer; verify the download came from this repository's Releases page.

## License

MIT — see [LICENSE](LICENSE). Not affiliated with Police Roleplay Community or Roblox.
