-- ER:LC CAD initial schema.
-- All timestamps are Unix epoch milliseconds in UTC.
-- The API server key is NEVER stored here; only non-secret metadata is persisted.

CREATE TABLE roles (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    key         TEXT    NOT NULL UNIQUE,
    name        TEXT    NOT NULL,
    description TEXT    NOT NULL DEFAULT '',
    is_system   INTEGER NOT NULL DEFAULT 0 CHECK (is_system IN (0, 1)),
    created_at  INTEGER NOT NULL
);

CREATE TABLE permissions (
    key      TEXT PRIMARY KEY,
    label    TEXT NOT NULL,
    category TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT ''
);

CREATE TABLE role_permissions (
    role_id        INTEGER NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    permission_key TEXT    NOT NULL REFERENCES permissions(key) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_key)
);

CREATE TABLE app_users (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    username       TEXT    NOT NULL UNIQUE COLLATE NOCASE,
    display_name   TEXT    NOT NULL,
    password_hash  TEXT    NOT NULL,
    role_id        INTEGER NOT NULL REFERENCES roles(id) ON DELETE RESTRICT,
    roblox_user_id INTEGER,
    is_active      INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    failed_attempts INTEGER NOT NULL DEFAULT 0,
    locked_until   INTEGER,
    created_at     INTEGER NOT NULL,
    last_login_at  INTEGER
);

CREATE INDEX idx_app_users_role ON app_users(role_id);

-- Per-user grants or denials layered on top of the role.
CREATE TABLE user_permissions (
    user_id        INTEGER NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    permission_key TEXT    NOT NULL REFERENCES permissions(key) ON DELETE CASCADE,
    allowed        INTEGER NOT NULL CHECK (allowed IN (0, 1)),
    PRIMARY KEY (user_id, permission_key)
);

CREATE TABLE players (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    roblox_user_id INTEGER NOT NULL UNIQUE,
    username       TEXT    NOT NULL,
    display_name   TEXT,
    first_seen_at  INTEGER NOT NULL,
    last_seen_at   INTEGER NOT NULL,
    last_team      TEXT,
    last_callsign  TEXT,
    avatar_updated_at INTEGER
);

CREATE INDEX idx_players_username ON players(username COLLATE NOCASE);
CREATE INDEX idx_players_last_seen ON players(last_seen_at DESC);

CREATE TABLE moderation_records (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    roblox_user_id    INTEGER,
    username          TEXT NOT NULL,
    action            TEXT NOT NULL CHECK (action IN ('kick', 'ban', 'unban', 'internal_ban', 'internal_unban', 'note')),
    source            TEXT NOT NULL CHECK (source IN ('erlc_api', 'internal_cad')),
    reason            TEXT NOT NULL DEFAULT '',
    moderator_user_id INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    moderator_name    TEXT NOT NULL,
    success           INTEGER NOT NULL DEFAULT 0 CHECK (success IN (0, 1)),
    command_id        TEXT,
    api_message       TEXT,
    created_at        INTEGER NOT NULL
);

CREATE INDEX idx_moderation_player ON moderation_records(roblox_user_id, created_at DESC);
CREATE INDEX idx_moderation_created ON moderation_records(created_at DESC);

-- Internal CAD bans. These do NOT ban a player inside ER:LC; the API has no ban-write endpoint.
CREATE TABLE internal_bans (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    roblox_user_id    INTEGER,
    username          TEXT NOT NULL,
    reason            TEXT NOT NULL DEFAULT '',
    created_by_user_id INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    created_by_name   TEXT NOT NULL,
    created_at        INTEGER NOT NULL,
    active            INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    lifted_at         INTEGER,
    lifted_by_name    TEXT,
    lift_reason       TEXT
);

CREATE INDEX idx_internal_bans_player ON internal_bans(roblox_user_id, active);
CREATE UNIQUE INDEX idx_internal_bans_active_player
    ON internal_bans(roblox_user_id) WHERE active = 1 AND roblox_user_id IS NOT NULL;

CREATE TABLE staff_notes (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    roblox_user_id INTEGER,
    username       TEXT NOT NULL,
    author_user_id INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    author_name    TEXT NOT NULL,
    body           TEXT NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE INDEX idx_staff_notes_player ON staff_notes(roblox_user_id, created_at DESC);

CREATE TABLE dispatch_units (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    callsign         TEXT NOT NULL UNIQUE COLLATE NOCASE,
    label            TEXT NOT NULL DEFAULT '',
    team             TEXT NOT NULL DEFAULT '',
    status           TEXT NOT NULL DEFAULT 'available'
                     CHECK (status IN ('available', 'on_duty', 'en_route', 'on_scene', 'transporting', 'unavailable', 'off_duty')),
    roblox_user_id   INTEGER,
    assigned_call_id INTEGER REFERENCES dispatch_calls(id) ON DELETE SET NULL,
    created_by_user_id INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);

CREATE INDEX idx_dispatch_units_status ON dispatch_units(status);

CREATE TABLE dispatch_calls (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    call_number      INTEGER NOT NULL UNIQUE,
    title            TEXT NOT NULL,
    description      TEXT NOT NULL DEFAULT '',
    location         TEXT NOT NULL DEFAULT '',
    postal           TEXT NOT NULL DEFAULT '',
    priority         TEXT NOT NULL DEFAULT 'medium'
                     CHECK (priority IN ('low', 'medium', 'high', 'critical')),
    status           TEXT NOT NULL DEFAULT 'open'
                     CHECK (status IN ('open', 'assigned', 'in_progress', 'closed')),
    source           TEXT NOT NULL DEFAULT 'cad' CHECK (source IN ('cad', 'erlc')),
    erlc_call_number INTEGER,
    created_by_user_id INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    created_by_name  TEXT NOT NULL,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    closed_at        INTEGER,
    closed_by_name   TEXT
);

CREATE INDEX idx_dispatch_calls_status ON dispatch_calls(status, created_at DESC);
CREATE UNIQUE INDEX idx_dispatch_calls_erlc ON dispatch_calls(erlc_call_number) WHERE erlc_call_number IS NOT NULL;

CREATE TABLE call_units (
    call_id     INTEGER NOT NULL REFERENCES dispatch_calls(id) ON DELETE CASCADE,
    unit_id     INTEGER NOT NULL REFERENCES dispatch_units(id) ON DELETE CASCADE,
    assigned_at INTEGER NOT NULL,
    PRIMARY KEY (call_id, unit_id)
);

CREATE TABLE call_players (
    call_id        INTEGER NOT NULL REFERENCES dispatch_calls(id) ON DELETE CASCADE,
    roblox_user_id INTEGER NOT NULL,
    username       TEXT NOT NULL,
    PRIMARY KEY (call_id, roblox_user_id)
);

CREATE TABLE audit_logs (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_user_id  INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    actor_name     TEXT NOT NULL,
    category       TEXT NOT NULL,
    action         TEXT NOT NULL,
    target         TEXT NOT NULL DEFAULT '',
    details        TEXT NOT NULL DEFAULT '',
    created_at     INTEGER NOT NULL
);

CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);
CREATE INDEX idx_audit_category ON audit_logs(category, created_at DESC);

CREATE TABLE command_history (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    actor_user_id  INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    actor_name     TEXT NOT NULL,
    command        TEXT NOT NULL,
    status         TEXT NOT NULL CHECK (status IN ('success', 'failed', 'rejected')),
    response       TEXT NOT NULL DEFAULT '',
    http_status    INTEGER,
    command_id     TEXT,
    duration_ms    INTEGER NOT NULL DEFAULT 0,
    created_at     INTEGER NOT NULL
);

CREATE INDEX idx_command_history_created ON command_history(created_at DESC);

CREATE TABLE command_policies (
    command        TEXT PRIMARY KEY,
    min_permission TEXT NOT NULL REFERENCES permissions(key) ON DELETE RESTRICT,
    destructive    INTEGER NOT NULL DEFAULT 0 CHECK (destructive IN (0, 1)),
    enabled        INTEGER NOT NULL DEFAULT 1 CHECK (enabled IN (0, 1)),
    description    TEXT NOT NULL DEFAULT '',
    updated_at     INTEGER NOT NULL
);

CREATE TABLE settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE api_config (
    id               INTEGER PRIMARY KEY CHECK (id = 1),
    base_url         TEXT NOT NULL,
    key_storage      TEXT NOT NULL DEFAULT 'os_keychain',
    key_fingerprint  TEXT,
    key_last4        TEXT,
    last_verified_at INTEGER,
    server_name      TEXT,
    server_id        INTEGER,
    updated_at       INTEGER NOT NULL
);

CREATE TABLE metric_samples (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    captured_at     INTEGER NOT NULL,
    player_count    INTEGER NOT NULL,
    max_players     INTEGER NOT NULL,
    staff_count     INTEGER NOT NULL DEFAULT 0,
    latency_ms      INTEGER
);

CREATE INDEX idx_metric_samples_time ON metric_samples(captured_at DESC);

INSERT INTO permissions (key, label, category, description) VALUES
    ('dashboard.view',      'View dashboard',          'Dashboard',  'Access the live server overview'),
    ('map.view',            'View live map',           'Live Map',   'Access live map positions and calls'),
    ('players.search',      'Search players',          'Players',    'Search the player database and live roster'),
    ('players.view',        'View player profiles',    'Players',    'Open player profiles and Roblox identities'),
    ('players.notes.view',  'View staff notes',        'Players',    'Read internal staff notes on players'),
    ('players.notes.manage','Manage staff notes',      'Players',    'Create and edit internal staff notes'),
    ('moderation.view',     'View moderation records', 'Moderation', 'Read moderation history and live bans'),
    ('moderation.kick',     'Kick players',            'Moderation', 'Kick players from the ER:LC server'),
    ('moderation.ban',      'Ban players',             'Moderation', 'Ban players on the ER:LC server'),
    ('moderation.unban',    'Unban players',           'Moderation', 'Lift ER:LC server bans'),
    ('moderation.internal', 'Manage internal bans',    'Moderation', 'Create and lift CAD-only ban records'),
    ('moderation.notes',    'Record moderation notes', 'Moderation', 'Attach notes to moderation records'),
    ('commands.view',       'View command console',    'Commands',   'See the remote command console and history'),
    ('commands.execute',    'Execute server commands', 'Commands',   'Run authorized commands against the server'),
    ('commands.policies.manage', 'Manage command policies', 'Commands', 'Authorize which commands may run at which permission level'),
    ('dispatch.view',       'View CAD dispatch',       'Dispatch',   'See calls and unit status'),
    ('dispatch.create',     'Create calls',            'Dispatch',   'Open new CAD calls'),
    ('dispatch.update',     'Update calls',            'Dispatch',   'Edit calls, notes and associations'),
    ('dispatch.close',      'Close calls',             'Dispatch',   'Close and archive CAD calls'),
    ('dispatch.units',      'Manage units',            'Dispatch',   'Create units and change unit status'),
    ('vehicles.view',       'View vehicles',           'Server',     'See vehicles spawned in the server'),
    ('emergency.view',      'View emergency calls',    'Server',     'See live ER:LC emergency calls'),
    ('logs.view',           'View server logs',        'Server',     'See join, kill, command and moderator logs'),
    ('staff.view',          'View staff',              'Staff',      'See the staff directory and activity'),
    ('staff.manage',        'Manage staff',            'Staff',      'Create staff accounts and assign roles'),
    ('roles.manage',        'Manage roles',            'Staff',      'Create roles and change their permissions'),
    ('audit.view',          'View audit logs',         'Staff',      'Read the application audit trail'),
    ('api.configure',       'Configure API access',    'Settings',   'Set and test the ER:LC server key'),
    ('settings.manage',     'Manage settings',         'Settings',   'Change application preferences'),
    ('database.backup',     'Back up database',        'Settings',   'Create database backups'),
    ('database.restore',    'Restore database',        'Settings',   'Restore the database from a backup');

INSERT INTO roles (key, name, description, is_system, created_at) VALUES
    ('owner',         'Server Owner',  'Full access to every module, including permissions and API credentials.', 1, 0),
    ('administrator', 'Administrator', 'Server management, moderation, dispatch and audit access.',             1, 0),
    ('moderator',     'Moderator',     'Player lookup, assigned moderation tools and authorized commands.',      1, 0);

-- Server Owner: every permission.
INSERT INTO role_permissions (role_id, permission_key)
SELECT r.id, p.key FROM roles r CROSS JOIN permissions p WHERE r.key = 'owner';

-- Administrator: everything except destructive account/role/API ownership controls.
INSERT INTO role_permissions (role_id, permission_key)
SELECT r.id, p.key FROM roles r CROSS JOIN permissions p
WHERE r.key = 'administrator'
  AND p.key NOT IN ('roles.manage', 'api.configure', 'database.restore', 'commands.policies.manage');

-- Moderator: front-line tools only.
INSERT INTO role_permissions (role_id, permission_key)
SELECT r.id, p.key FROM roles r CROSS JOIN permissions p
WHERE r.key = 'moderator'
  AND p.key IN (
    'dashboard.view', 'map.view', 'players.search', 'players.view', 'players.notes.view',
    'moderation.view', 'moderation.kick', 'moderation.ban', 'moderation.notes',
    'commands.view', 'commands.execute',
    'dispatch.view', 'dispatch.create', 'dispatch.update', 'dispatch.units',
    'vehicles.view', 'emergency.view'
  );

INSERT INTO command_policies (command, min_permission, destructive, enabled, description, updated_at) VALUES
    (':h',      'commands.execute',  0, 1, 'Broadcast a hint / server-wide message (documented in the ER:LC API reference).', 0),
    (':kick',   'moderation.kick',   1, 1, 'Remove a player from the server. Requires confirmation.', 0),
    (':ban',    'moderation.ban',    1, 1, 'Ban a player from the ER:LC server. Requires confirmation.', 0),
    (':unban',  'moderation.unban',  1, 1, 'Lift an ER:LC server ban. Requires confirmation and the moderation.unban permission.', 0);

INSERT INTO settings (key, value, updated_at) VALUES
    ('refresh_interval_seconds', '15', 0),
    ('startup_page', 'dashboard', 0),
    ('auto_connect', 'true', 0),
    ('confirm_destructive', 'true', 0),
    ('compact_tables', 'false', 0),
    ('map_image', 'https://api.erlc.gg/maps/summer_postals.png', 0),
    ('map_scale', '3', 0),
    ('session_idle_minutes', '60', 0),
    ('backup_directory', '', 0),
    ('auto_backup_hours', '24', 0),
    ('notify_emergency_calls', 'true', 0),
    ('notify_mod_calls', 'true', 0),
    ('notify_connection', 'true', 0),
    ('last_backup_at', '', 0);

INSERT INTO api_config (id, base_url, key_storage, updated_at)
VALUES (1, 'https://api.erlc.gg', 'windows_dpapi', 0);
