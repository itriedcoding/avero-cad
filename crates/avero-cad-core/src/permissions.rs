//! Granular permission catalogue.
//!
//! Roles are only a convenient bundle: every backend operation checks an
//! individual permission key, and per-user overrides can grant or deny any key.
//! The server owner role always holds the complete catalogue so an owner can
//! never lock themselves out of their own installation.

use crate::error::{AppError, AppResult};
use serde::Serialize;
use std::collections::BTreeSet;

pub const ROLE_OWNER: &str = "owner";
pub const ROLE_ADMINISTRATOR: &str = "administrator";
pub const ROLE_MODERATOR: &str = "moderator";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
    DashboardView,
    MapView,
    PlayersSearch,
    PlayersView,
    PlayersNotesView,
    PlayersNotesManage,
    ModerationView,
    ModerationKick,
    ModerationBan,
    ModerationUnban,
    ModerationInternal,
    ModerationNotes,
    CommandsView,
    CommandsExecute,
    CommandsPoliciesManage,
    DispatchView,
    DispatchCreate,
    DispatchUpdate,
    DispatchClose,
    DispatchUnits,
    VehiclesView,
    EmergencyView,
    LogsView,
    StaffView,
    StaffManage,
    RolesManage,
    AuditView,
    ApiConfigure,
    SettingsManage,
    DatabaseBackup,
    DatabaseRestore,
    BolosView,
    BolosManage,
    ShiftsView,
    ShiftsManage,
    StatsView,
}

impl Permission {
    pub const ALL: &'static [Permission] = &[
        Permission::DashboardView,
        Permission::MapView,
        Permission::PlayersSearch,
        Permission::PlayersView,
        Permission::PlayersNotesView,
        Permission::PlayersNotesManage,
        Permission::ModerationView,
        Permission::ModerationKick,
        Permission::ModerationBan,
        Permission::ModerationUnban,
        Permission::ModerationInternal,
        Permission::ModerationNotes,
        Permission::CommandsView,
        Permission::CommandsExecute,
        Permission::CommandsPoliciesManage,
        Permission::DispatchView,
        Permission::DispatchCreate,
        Permission::DispatchUpdate,
        Permission::DispatchClose,
        Permission::DispatchUnits,
        Permission::VehiclesView,
        Permission::EmergencyView,
        Permission::LogsView,
        Permission::StaffView,
        Permission::StaffManage,
        Permission::RolesManage,
        Permission::AuditView,
        Permission::ApiConfigure,
        Permission::SettingsManage,
        Permission::DatabaseBackup,
        Permission::DatabaseRestore,
        Permission::BolosView,
        Permission::BolosManage,
        Permission::ShiftsView,
        Permission::ShiftsManage,
        Permission::StatsView,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Permission::DashboardView => "dashboard.view",
            Permission::MapView => "map.view",
            Permission::PlayersSearch => "players.search",
            Permission::PlayersView => "players.view",
            Permission::PlayersNotesView => "players.notes.view",
            Permission::PlayersNotesManage => "players.notes.manage",
            Permission::ModerationView => "moderation.view",
            Permission::ModerationKick => "moderation.kick",
            Permission::ModerationBan => "moderation.ban",
            Permission::ModerationUnban => "moderation.unban",
            Permission::ModerationInternal => "moderation.internal",
            Permission::ModerationNotes => "moderation.notes",
            Permission::CommandsView => "commands.view",
            Permission::CommandsExecute => "commands.execute",
            Permission::CommandsPoliciesManage => "commands.policies.manage",
            Permission::DispatchView => "dispatch.view",
            Permission::DispatchCreate => "dispatch.create",
            Permission::DispatchUpdate => "dispatch.update",
            Permission::DispatchClose => "dispatch.close",
            Permission::DispatchUnits => "dispatch.units",
            Permission::VehiclesView => "vehicles.view",
            Permission::EmergencyView => "emergency.view",
            Permission::LogsView => "logs.view",
            Permission::StaffView => "staff.view",
            Permission::StaffManage => "staff.manage",
            Permission::RolesManage => "roles.manage",
            Permission::AuditView => "audit.view",
            Permission::ApiConfigure => "api.configure",
            Permission::SettingsManage => "settings.manage",
            Permission::DatabaseBackup => "database.backup",
            Permission::DatabaseRestore => "database.restore",
            Permission::BolosView => "bolos.view",
            Permission::BolosManage => "bolos.manage",
            Permission::ShiftsView => "shifts.view",
            Permission::ShiftsManage => "shifts.manage",
            Permission::StatsView => "stats.view",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Permission::DashboardView => "View dashboard",
            Permission::MapView => "View live map",
            Permission::PlayersSearch => "Search players",
            Permission::PlayersView => "View player profiles",
            Permission::PlayersNotesView => "View staff notes",
            Permission::PlayersNotesManage => "Manage staff notes",
            Permission::ModerationView => "View moderation records",
            Permission::ModerationKick => "Kick players",
            Permission::ModerationBan => "Ban players",
            Permission::ModerationUnban => "Unban players",
            Permission::ModerationInternal => "Manage internal bans",
            Permission::ModerationNotes => "Record moderation notes",
            Permission::CommandsView => "View command console",
            Permission::CommandsExecute => "Execute server commands",
            Permission::CommandsPoliciesManage => "Manage command policies",
            Permission::DispatchView => "View CAD dispatch",
            Permission::DispatchCreate => "Create calls",
            Permission::DispatchUpdate => "Update calls",
            Permission::DispatchClose => "Close calls",
            Permission::DispatchUnits => "Manage units",
            Permission::VehiclesView => "View vehicles",
            Permission::EmergencyView => "View emergency calls",
            Permission::LogsView => "View server logs",
            Permission::StaffView => "View staff",
            Permission::StaffManage => "Manage staff",
            Permission::RolesManage => "Manage roles",
            Permission::AuditView => "View audit logs",
            Permission::ApiConfigure => "Configure API access",
            Permission::SettingsManage => "Manage settings",
            Permission::DatabaseBackup => "Back up database",
            Permission::DatabaseRestore => "Restore database",
            Permission::BolosView => "View BOLO board",
            Permission::BolosManage => "Manage BOLOs",
            Permission::ShiftsView => "View duty roster",
            Permission::ShiftsManage => "Manage duty roster",
            Permission::StatsView => "View statistics",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Permission::DashboardView => "Dashboard",
            Permission::MapView => "Live Map",
            Permission::PlayersSearch
            | Permission::PlayersView
            | Permission::PlayersNotesView
            | Permission::PlayersNotesManage => "Players",
            Permission::ModerationView
            | Permission::ModerationKick
            | Permission::ModerationBan
            | Permission::ModerationUnban
            | Permission::ModerationInternal
            | Permission::ModerationNotes => "Moderation",
            Permission::CommandsView
            | Permission::CommandsExecute
            | Permission::CommandsPoliciesManage => "Commands",
            Permission::DispatchView
            | Permission::DispatchCreate
            | Permission::DispatchUpdate
            | Permission::DispatchClose
            | Permission::DispatchUnits => "Dispatch",
            Permission::VehiclesView | Permission::EmergencyView | Permission::LogsView => "Server",
            Permission::BolosView | Permission::BolosManage | Permission::StatsView => "Operations",
            Permission::StaffView
            | Permission::StaffManage
            | Permission::RolesManage
            | Permission::ShiftsView
            | Permission::ShiftsManage
            | Permission::AuditView => "Staff",
            Permission::ApiConfigure
            | Permission::SettingsManage
            | Permission::DatabaseBackup
            | Permission::DatabaseRestore => "Settings",
        }
    }

    pub fn from_key(key: &str) -> Option<Permission> {
        Permission::ALL.iter().copied().find(|p| p.key() == key)
    }
}

/// Serializable description of a single permission, used by the role editor.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionInfo {
    pub key: String,
    pub label: String,
    pub category: String,
    pub description: String,
}

pub fn catalogue() -> Vec<PermissionInfo> {
    Permission::ALL
        .iter()
        .map(|p| PermissionInfo {
            key: p.key().to_string(),
            label: p.label().to_string(),
            category: p.category().to_string(),
            description: String::new(),
        })
        .collect()
}

/// Authenticated actor together with the permissions resolved from the database.
#[derive(Debug, Clone)]
pub struct Access {
    pub user_id: i64,
    pub username: String,
    pub display_name: String,
    pub role_id: i64,
    pub role_key: String,
    pub role_name: String,
    pub permissions: BTreeSet<String>,
}

impl Access {
    pub fn can(&self, permission: Permission) -> bool {
        self.is_owner() || self.permissions.contains(permission.key())
    }

    pub fn is_owner(&self) -> bool {
        self.role_key == ROLE_OWNER
    }

    pub fn require(&self, permission: Permission) -> AppResult<()> {
        if self.can(permission) {
            Ok(())
        } else {
            Err(AppError::permission(format!(
                "Your role does not include the \"{}\" permission.",
                permission.label()
            )))
        }
    }

    pub fn effective_permissions(&self) -> Vec<String> {
        if self.is_owner() {
            Permission::ALL
                .iter()
                .map(|p| p.key().to_string())
                .collect()
        } else {
            self.permissions.iter().cloned().collect()
        }
    }
}

/// Resolve effective permissions from a role key, role grants and user overrides.
pub fn resolve_permissions(
    role_key: &str,
    role_grants: impl IntoIterator<Item = String>,
    overrides: impl IntoIterator<Item = (String, bool)>,
) -> BTreeSet<String> {
    if role_key == ROLE_OWNER {
        return Permission::ALL
            .iter()
            .map(|p| p.key().to_string())
            .collect();
    }
    let mut resolved: BTreeSet<String> = role_grants
        .into_iter()
        .filter(|key| Permission::from_key(key).is_some())
        .collect();
    for (key, allowed) in overrides {
        if Permission::from_key(&key).is_none() {
            continue;
        }
        if allowed {
            resolved.insert(key);
        } else {
            resolved.remove(&key);
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_is_unique_and_parses_back() {
        let mut keys = BTreeSet::new();
        for permission in Permission::ALL {
            assert!(
                keys.insert(permission.key()),
                "duplicate key {}",
                permission.key()
            );
            assert_eq!(Permission::from_key(permission.key()), Some(*permission));
            assert!(!permission.label().is_empty());
        }
        assert_eq!(keys.len(), Permission::ALL.len());
    }

    #[test]
    fn owner_always_has_every_permission() {
        let access = Access {
            user_id: 1,
            username: "owner".into(),
            display_name: "Owner".into(),
            role_id: 1,
            role_key: ROLE_OWNER.into(),
            role_name: "Server Owner".into(),
            permissions: BTreeSet::new(),
        };
        assert!(access.can(Permission::ApiConfigure));
        assert!(access.can(Permission::DatabaseRestore));
    }

    #[test]
    fn moderator_cannot_manage_staff() {
        let access = Access {
            user_id: 2,
            username: "mod".into(),
            display_name: "Mod".into(),
            role_id: 3,
            role_key: ROLE_MODERATOR.into(),
            role_name: "Moderator".into(),
            permissions: resolve_permissions(
                ROLE_MODERATOR,
                vec!["players.view".into(), "commands.execute".into()],
                vec![],
            ),
        };
        assert!(access.can(Permission::PlayersView));
        assert!(!access.can(Permission::StaffManage));
        assert!(access.require(Permission::StaffManage).is_err());
    }

    #[test]
    fn user_override_can_grant_and_revoke() {
        let resolved = resolve_permissions(
            ROLE_MODERATOR,
            vec!["players.view".into(), "moderation.kick".into()],
            vec![("players.view".into(), false), ("logs.view".into(), true)],
        );
        assert!(!resolved.contains("players.view"));
        assert!(resolved.contains("moderation.kick"));
        assert!(resolved.contains("logs.view"));
    }

    #[test]
    fn unknown_permission_keys_are_ignored() {
        let resolved = resolve_permissions(
            ROLE_ADMINISTRATOR,
            vec!["totally.made.up".into(), "players.view".into()],
            vec![("also.fake".into(), true)],
        );
        assert_eq!(resolved.len(), 1);
        assert!(resolved.contains("players.view"));
    }
}
