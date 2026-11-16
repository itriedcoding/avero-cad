# Security Policy

ER:LC CAD can execute real commands against a live ER:LC private server, so security issues are taken
seriously.

## Reporting a vulnerability

Do **not** open a public issue for a security problem. Use GitHub's private vulnerability reporting
for this repository (Security → Report a vulnerability) and include:

- the affected version or commit,
- reproduction steps,
- the impact you believe it has,
- whether the issue requires local access, a staff account, or only the network.

You can expect an acknowledgement within a few days. Please give us reasonable time to ship a fix
before public disclosure.

## Scope

In scope:

- authentication, session handling and permission bypasses,
- command injection into ER:LC or into Windows,
- exposure of the ER:LC server key or staff credentials,
- SQL injection or database corruption,
- path traversal in backup/restore or avatar caching,
- privilege escalation between roles or per-account overrides.

Out of scope:

- issues that require an already-compromised Windows user session with access to the DPAPI blob,
- ER:LC API behaviour or permission decisions made by the API itself,
- unsigned-build SmartScreen warnings (documented, not a vulnerability),
- denial of service caused by the ER:LC API's own rate limits.

## Security model

### Credentials

- The ER:LC server key is encrypted with Windows DPAPI (`CryptProtectData`, current-user scope) and
  written to the application data directory. The database stores only the base URL, the last four
  characters, a SHA-256 fingerprint and the last verification time.
- Keys are never returned over IPC after saving, never written to logs (request logging redacts the
  `server-key` header and the token that follows it), and never committed to the repository.
- If DPAPI protection fails, the key is not persisted at all. No Windows build has a plaintext
  fallback; development builds on other platforms use a clearly labelled local file that no
  installer ships.

### Staff authentication

- Passwords are hashed with Argon2id.
- Sessions use 256-bit random tokens, stored hashed, with a 12-hour absolute lifetime and a
  configurable idle timeout (5–480 minutes, default 60).
- Repeated failed sign-ins lock the account for a cooling-off period.
- Every IPC command loads the account and its current permissions from the database; disabling an
  account or changing a role takes effect on the next call, and disabling an account revokes its
  sessions.

### Authorization

- Permissions are checked in Rust for every command. The React UI hides controls a user cannot use,
  but the UI is never the enforcement point.
- The owner role cannot be limited and owner-only configuration (API key, roles, backup restore)
  stays owner-only.
- Local permissions never expand what ER:LC allows: the API still authorizes the server key.

### Command execution

- Only `:`-prefixed ER:LC commands with an alphanumeric name are accepted. Newlines, control
  characters and shell metacharacters are rejected, so the console cannot reach a Windows shell.
- Commands must exist in the owner-managed policy table with a minimum permission.
- Destructive commands require an explicit confirmation and are recorded with actor, target, outcome
  and API command ID.
- The API's five-second command cadence is enforced locally; failed commands are never retried, so a
  command cannot execute twice.

### Data

- SQLite access uses prepared statements, foreign keys and transactions; `LIKE` patterns are escaped.
- Audit records are append-only from the application's perspective.
- Backup and restore accept absolute paths only, validate the source file, and replace the database
  through SQLite's backup API.
- Avatar images are cached per Roblox user ID and verified by signature and size before being served
  back to the UI as data URLs.

## Hardening recommendations for operators

1. Use a dedicated Windows account for dispatch machines.
2. Give each dispatcher and moderator their own staff account; never share the owner login.
3. Keep only the commands you actually use in **Command Policies**.
4. Rotate the ER:LC server key if a machine is lost or a staff member leaves with access.
5. Keep automatic backups enabled and store the backup folder on a protected volume.
6. Treat the installer as unsigned: verify checksums and download only from this repository's
   Releases page.
