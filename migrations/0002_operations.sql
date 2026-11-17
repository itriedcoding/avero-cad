-- Avero CAD operations schema: BOLO reports, duty shifts and their permissions.

CREATE TABLE bolo_reports (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    kind               TEXT    NOT NULL CHECK (kind IN ('person', 'vehicle')),
    subject_name       TEXT    NOT NULL,
    plate              TEXT    NOT NULL DEFAULT '',
    description        TEXT    NOT NULL DEFAULT '',
    last_seen          TEXT    NOT NULL DEFAULT '',
    priority           TEXT    NOT NULL DEFAULT 'medium'
                       CHECK (priority IN ('low', 'medium', 'high', 'critical')),
    status             TEXT    NOT NULL DEFAULT 'active'
                       CHECK (status IN ('active', 'cleared')),
    created_by_user_id INTEGER REFERENCES app_users(id) ON DELETE SET NULL,
    created_by_name    TEXT    NOT NULL,
    created_at         INTEGER NOT NULL,
    cleared_by_name    TEXT,
    cleared_at         INTEGER,
    clearance_note     TEXT
);

CREATE INDEX idx_bolo_status ON bolo_reports(status, created_at DESC);
CREATE INDEX idx_bolo_plate ON bolo_reports(plate);

-- One row per completed or in-progress duty shift. ended_at NULL means on duty.
CREATE TABLE duty_shifts (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id      INTEGER NOT NULL REFERENCES app_users(id) ON DELETE CASCADE,
    display_name TEXT    NOT NULL,
    started_at   INTEGER NOT NULL,
    ended_at     INTEGER,
    end_reason   TEXT
);

CREATE UNIQUE INDEX idx_duty_open_user ON duty_shifts(user_id) WHERE ended_at IS NULL;
CREATE INDEX idx_duty_started ON duty_shifts(started_at DESC);
CREATE INDEX idx_duty_user ON duty_shifts(user_id, started_at DESC);

INSERT INTO permissions (key, label, category, description) VALUES
    ('bolos.view',   'View BOLO board',   'Operations', 'Read active and cleared BOLO reports'),
    ('bolos.manage', 'Manage BOLOs',      'Operations', 'Create, clear and reopen BOLO reports'),
    ('shifts.view',  'View duty roster',  'Staff',      'See who is on duty and review shift history'),
    ('shifts.manage','Manage duty roster','Staff',      'End another staff member''s duty shift'),
    ('stats.view',   'View statistics',   'Operations', 'Read aggregated activity statistics');

INSERT INTO role_permissions (role_id, permission_key)
SELECT r.id, p.key FROM roles r CROSS JOIN permissions p
WHERE r.key = 'owner'
  AND p.key IN ('bolos.view', 'bolos.manage', 'shifts.view', 'shifts.manage', 'stats.view');

INSERT INTO role_permissions (role_id, permission_key)
SELECT r.id, p.key FROM roles r CROSS JOIN permissions p
WHERE r.key = 'administrator'
  AND p.key IN ('bolos.view', 'bolos.manage', 'shifts.view', 'shifts.manage', 'stats.view');

INSERT INTO role_permissions (role_id, permission_key)
SELECT r.id, p.key FROM roles r CROSS JOIN permissions p
WHERE r.key = 'moderator'
  AND p.key IN ('bolos.view', 'shifts.view', 'stats.view');
