//! ER:LC API payloads (v2, plus the v1 bans endpoint) and the view models
//! exchanged with the React frontend.
//!
//! The API uses PascalCase keys; the frontend consumes camelCase. Model fields
//! mirror the published OpenAPI schemas exactly, with optional blocks modelled
//! as `Option` because they are only returned when the matching query flag is
//! set to `true`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// ER:LC API v2 response models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServerInfo {
    pub name: String,
    pub owner_id: u64,
    #[serde(default)]
    pub co_owner_ids: Vec<u64>,
    pub current_players: i64,
    pub max_players: i64,
    pub join_key: String,
    pub acc_verified_req: String,
    pub team_balance: bool,
    #[serde(default)]
    pub players: Option<Vec<ApiPlayer>>,
    #[serde(default)]
    pub staff: Option<ApiStaff>,
    #[serde(default)]
    pub join_logs: Option<Vec<JoinLog>>,
    #[serde(default)]
    pub queue: Option<Vec<u64>>,
    #[serde(default)]
    pub kill_logs: Option<Vec<KillLog>>,
    #[serde(default)]
    pub command_logs: Option<Vec<CommandLog>>,
    #[serde(default)]
    pub mod_calls: Option<Vec<ModCall>>,
    #[serde(default)]
    pub emergency_calls: Option<Vec<EmergencyCall>>,
    #[serde(default)]
    pub vehicles: Option<Vec<ApiVehicle>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ApiPlayer {
    pub team: String,
    /// "Username:UserId" in v2 and v1 alike.
    pub player: String,
    #[serde(default)]
    pub callsign: Option<String>,
    #[serde(default)]
    pub location: Option<ApiLocation>,
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub wanted_stars: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ApiLocation {
    #[serde(default)]
    pub location_x: f64,
    #[serde(default)]
    pub location_z: f64,
    #[serde(default)]
    pub postal_code: Option<String>,
    #[serde(default)]
    pub street_name: Option<String>,
    #[serde(default)]
    pub building_number: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ApiStaff {
    #[serde(default)]
    pub admins: BTreeMap<String, String>,
    #[serde(default)]
    pub mods: BTreeMap<String, String>,
    #[serde(default)]
    pub helpers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct JoinLog {
    #[serde(default)]
    pub join: bool,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub player: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct KillLog {
    #[serde(default)]
    pub killed: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub killer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommandLog {
    #[serde(default)]
    pub player: String,
    #[serde(default)]
    pub timestamp: i64,
    #[serde(default)]
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ModCall {
    #[serde(default)]
    pub caller: String,
    #[serde(default)]
    pub moderator: Option<String>,
    #[serde(default)]
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmergencyCall {
    #[serde(default)]
    pub team: String,
    #[serde(default)]
    pub caller: i64,
    #[serde(default)]
    pub players: Vec<i64>,
    #[serde(default)]
    pub position: Vec<f64>,
    #[serde(default)]
    pub started_at: i64,
    #[serde(default)]
    pub call_number: i64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub position_descriptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ApiVehicle {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub plate: String,
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default)]
    pub color_hex: String,
    #[serde(default)]
    pub color_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CommandApiResponse {
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub command_id: Option<String>,
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Split the API's "Username:UserId" composite field.
pub fn split_player(raw: &str) -> (String, Option<u64>) {
    match raw.rsplit_once(':') {
        Some((name, id)) if !name.trim().is_empty() && id.trim().parse::<u64>().is_ok() => {
            (name.trim().to_string(), id.trim().parse::<u64>().ok())
        }
        _ => (raw.trim().to_string(), None),
    }
}

/// Classify an ER:LC team name into a coarse service bucket for dashboard counts.
pub fn service_bucket(team: &str) -> &'static str {
    let lower = team.to_ascii_lowercase();
    if lower.contains("police")
        || lower.contains("sheriff")
        || lower.contains("patrol")
        || lower.contains("swat")
        || lower.contains("law")
    {
        "law_enforcement"
    } else if lower.contains("fire")
        || lower.contains("ems")
        || lower.contains("medical")
        || lower.contains("ambulance")
        || lower.contains("dot")
    {
        "fire_ems"
    } else {
        "civilian"
    }
}

// ---------------------------------------------------------------------------
// Frontend view models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitView {
    pub bucket: Option<String>,
    pub limit: Option<i64>,
    pub remaining: Option<i64>,
    pub reset: Option<i64>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerSummary {
    pub name: String,
    pub owner_id: u64,
    pub co_owner_ids: Vec<u64>,
    pub current_players: i64,
    pub max_players: i64,
    pub join_key: String,
    pub acc_verified_req: String,
    pub team_balance: bool,
}

impl From<&ServerInfo> for ServerSummary {
    fn from(info: &ServerInfo) -> Self {
        Self {
            name: info.name.clone(),
            owner_id: info.owner_id,
            co_owner_ids: info.co_owner_ids.clone(),
            current_players: info.current_players,
            max_players: info.max_players,
            join_key: info.join_key.clone(),
            acc_verified_req: info.acc_verified_req.clone(),
            team_balance: info.team_balance,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerLocationView {
    pub x: f64,
    pub z: f64,
    pub postal: Option<String>,
    pub street: Option<String>,
    pub building: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerView {
    pub roblox_user_id: Option<u64>,
    pub username: String,
    pub team: String,
    pub permission: String,
    pub callsign: Option<String>,
    pub location: Option<PlayerLocationView>,
    pub wanted_stars: i64,
    pub service: String,
    pub staff_role: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffMember {
    pub roblox_user_id: u64,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffRoster {
    pub admins: Vec<StaffMember>,
    pub mods: Vec<StaffMember>,
    pub helpers: Vec<StaffMember>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleView {
    pub name: String,
    pub owner: String,
    pub plate: String,
    pub texture: Option<String>,
    pub color_hex: String,
    pub color_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyCallView {
    pub call_number: i64,
    pub team: String,
    pub caller_id: i64,
    pub player_ids: Vec<i64>,
    pub description: String,
    pub position_descriptor: String,
    pub x: Option<f64>,
    pub z: Option<f64>,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinLogView {
    pub roblox_user_id: Option<u64>,
    pub username: String,
    pub joined: bool,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KillLogView {
    pub killer: String,
    pub killed: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandLogView {
    pub player: String,
    pub command: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModCallView {
    pub caller: String,
    pub moderator: Option<String>,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEntryView {
    pub roblox_user_id: u64,
    pub position: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterCounts {
    pub total: usize,
    pub law_enforcement: usize,
    pub fire_ems: usize,
    pub civilian: usize,
    pub staff: usize,
    pub wanted: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionView {
    /// "connected" | "error" | "not_configured"
    pub state: String,
    pub detail: Option<String>,
    pub api_url: String,
    pub latency_ms: Option<u64>,
    pub rate_limit: Option<RateLimitView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppNotification {
    pub id: String,
    /// "emergency" | "modcall" | "connection"
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricPoint {
    pub captured_at: i64,
    pub player_count: i64,
    pub max_players: i64,
    pub latency_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub server: Option<ServerSummary>,
    pub players: Vec<PlayerView>,
    pub staff: StaffRoster,
    pub vehicles: Vec<VehicleView>,
    pub emergency_calls: Vec<EmergencyCallView>,
    pub join_logs: Vec<JoinLogView>,
    pub kill_logs: Vec<KillLogView>,
    pub command_logs: Vec<CommandLogView>,
    pub mod_calls: Vec<ModCallView>,
    pub queue: Vec<QueueEntryView>,
    pub counts: RosterCounts,
    pub connection: ConnectionView,
    pub notifications: Vec<AppNotification>,
    pub history: Vec<MetricPoint>,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxUserView {
    pub id: u64,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub created: String,
    pub is_banned: bool,
    pub has_verified_badge: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarView {
    pub roblox_user_id: u64,
    pub data_url: String,
    pub cached: bool,
    pub fetched_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub roblox_user_id: Option<u64>,
    pub username: String,
    pub online: bool,
    pub team: Option<String>,
    pub callsign: Option<String>,
    pub permission: Option<String>,
    /// "live" when the ER:LC server reported the player, "database" for stored records.
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModerationRecordView {
    pub id: i64,
    pub roblox_user_id: Option<u64>,
    pub username: String,
    pub action: String,
    pub source: String,
    pub reason: String,
    pub moderator_name: String,
    pub success: bool,
    pub api_message: Option<String>,
    pub command_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InternalBanView {
    pub id: i64,
    pub roblox_user_id: Option<u64>,
    pub username: String,
    pub reason: String,
    pub created_by_name: String,
    pub created_at: i64,
    pub active: bool,
    pub lifted_at: Option<i64>,
    pub lifted_by_name: Option<String>,
    pub lift_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffNoteView {
    pub id: i64,
    pub roblox_user_id: Option<u64>,
    pub username: String,
    pub author_name: String,
    pub body: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub username: String,
    pub roblox_user_id: Option<u64>,
    pub roblox: Option<RobloxUserView>,
    pub live: Option<PlayerView>,
    pub first_seen_at: Option<i64>,
    pub last_seen_at: Option<i64>,
    pub moderation: Vec<ModerationRecordView>,
    pub notes: Vec<StaffNoteView>,
    pub internal_ban: Option<InternalBanView>,
    pub erlc_ban: bool,
    pub erlc_ban_detail: Option<String>,
    pub internal_ban_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitView {
    pub id: i64,
    pub callsign: String,
    pub label: String,
    pub team: String,
    pub status: String,
    pub roblox_user_id: Option<u64>,
    pub assigned_call_id: Option<i64>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallPlayerView {
    pub roblox_user_id: u64,
    pub username: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallView {
    pub id: i64,
    pub call_number: i64,
    pub title: String,
    pub description: String,
    pub location: String,
    pub postal: String,
    pub priority: String,
    pub status: String,
    pub source: String,
    pub erlc_call_number: Option<i64>,
    pub created_by_name: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub closed_at: Option<i64>,
    pub closed_by_name: Option<String>,
    pub units: Vec<UnitView>,
    pub players: Vec<CallPlayerView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffUserView {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role_id: i64,
    pub role_key: String,
    pub role_name: String,
    pub roblox_user_id: Option<u64>,
    pub is_active: bool,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
    pub overrides: Vec<PermissionOverrideView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOverrideView {
    pub permission_key: String,
    pub allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleView {
    pub id: i64,
    pub key: String,
    pub name: String,
    pub description: String,
    pub is_system: bool,
    pub permissions: Vec<String>,
    pub user_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub token: String,
    pub user: StaffUserView,
    pub permissions: Vec<String>,
    pub expires_at: i64,
    pub is_owner: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapView {
    pub needs_setup: bool,
    pub app_version: String,
    pub api_configured: bool,
    pub api_key_last4: Option<String>,
    pub api_base_url: String,
    pub platform: String,
    pub database_path: String,
    pub log_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfigView {
    pub base_url: String,
    pub configured: bool,
    pub key_last4: Option<String>,
    pub key_fingerprint: Option<String>,
    pub key_storage: String,
    pub last_verified_at: Option<i64>,
    pub server_name: Option<String>,
    pub server_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestView {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
    pub server_name: Option<String>,
    pub current_players: Option<i64>,
    pub max_players: Option<i64>,
    pub rate_limit: Option<RateLimitView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResultView {
    pub status: String,
    pub message: String,
    pub command_id: Option<String>,
    pub http_status: Option<u16>,
    pub duration_ms: u64,
    pub executed_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandHistoryView {
    pub id: i64,
    pub actor_name: String,
    pub command: String,
    pub status: String,
    pub response: String,
    pub http_status: Option<u16>,
    pub command_id: Option<String>,
    pub duration_ms: i64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandPolicyView {
    pub command: String,
    pub min_permission: String,
    pub destructive: bool,
    pub enabled: bool,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogView {
    pub id: i64,
    pub actor_name: String,
    pub category: String,
    pub action: String,
    pub target: String,
    pub details: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    pub refresh_interval_seconds: u64,
    pub startup_page: String,
    pub auto_connect: bool,
    pub confirm_destructive: bool,
    pub compact_tables: bool,
    pub map_image: String,
    pub map_scale: f64,
    pub session_idle_minutes: u64,
    pub backup_directory: String,
    pub auto_backup_hours: u64,
    pub notify_emergency_calls: bool,
    pub notify_mod_calls: bool,
    pub notify_connection: bool,
    pub last_backup_at: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub refresh_interval_seconds: Option<u64>,
    pub startup_page: Option<String>,
    pub auto_connect: Option<bool>,
    pub confirm_destructive: Option<bool>,
    pub compact_tables: Option<bool>,
    pub map_image: Option<String>,
    pub map_scale: Option<f64>,
    pub session_idle_minutes: Option<u64>,
    pub backup_directory: Option<String>,
    pub auto_backup_hours: Option<u64>,
    pub notify_emergency_calls: Option<bool>,
    pub notify_mod_calls: Option<bool>,
    pub notify_connection: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseStatsView {
    pub path: String,
    pub size_bytes: u64,
    pub players: i64,
    pub moderation_records: i64,
    pub internal_bans: i64,
    pub calls: i64,
    pub units: i64,
    pub users: i64,
    pub audit_logs: i64,
    pub commands: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupView {
    pub path: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveBanView {
    pub username: Option<String>,
    pub roblox_user_id: Option<u64>,
    pub raw: String,
}

// ---------------------------------------------------------------------------
// Operations: BOLO board, duty roster and statistics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BoloView {
    pub id: i64,
    /// "person" | "vehicle"
    pub kind: String,
    pub subject_name: String,
    pub plate: String,
    pub description: String,
    pub last_seen: String,
    pub priority: String,
    /// "active" | "cleared"
    pub status: String,
    pub created_by_name: String,
    pub created_at: i64,
    pub cleared_by_name: Option<String>,
    pub cleared_at: Option<i64>,
    pub clearance_note: Option<String>,
    /// Where the subject was last matched in the live snapshot, if anywhere.
    pub live_match: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyShiftView {
    pub id: i64,
    pub user_id: i64,
    pub display_name: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub duration_ms: i64,
    pub on_duty: bool,
    pub end_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DutyStatusView {
    pub current: Option<DutyShiftView>,
    pub today_ms: i64,
    pub week_ms: i64,
    pub total_ms: i64,
    pub shift_count: i64,
    pub roster: Vec<DutyShiftView>,
    pub recent: Vec<DutyShiftView>,
    pub can_view_roster: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStatView {
    pub date: String,
    pub calls_opened: i64,
    pub calls_closed: i64,
    pub moderation: i64,
    pub commands: i64,
    pub peak_players: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HourStatView {
    pub hour: i64,
    pub calls: i64,
    pub moderation: i64,
    pub commands: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TopModeratorView {
    pub name: String,
    pub actions: i64,
    pub failed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckView {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub available: bool,
    pub release_url: Option<String>,
    pub published_at: Option<String>,
    pub notes: Option<String>,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
    pub can_install: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInstallView {
    pub version: String,
    pub installer_path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsOverviewView {
    pub range_days: i64,
    pub since: i64,
    pub calls_opened: i64,
    pub calls_closed: i64,
    pub calls_open_now: i64,
    pub avg_close_ms: Option<i64>,
    pub moderation_total: i64,
    pub moderation_failed: i64,
    pub kicks: i64,
    pub bans: i64,
    pub unbans: i64,
    pub internal_bans: i64,
    pub commands_total: i64,
    pub commands_failed: i64,
    pub bolos_active: i64,
    pub bolos_cleared: i64,
    pub players_tracked: i64,
    pub players_new: i64,
    pub duty_ms: i64,
    pub duty_shifts: i64,
    pub on_duty_now: i64,
    pub daily: Vec<DailyStatView>,
    pub hours: Vec<HourStatView>,
    pub top_moderators: Vec<TopModeratorView>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_composite_player_field() {
        let (name, id) = split_player("Builderman:156");
        assert_eq!(name, "Builderman");
        assert_eq!(id, Some(156));
    }

    #[test]
    fn handles_usernames_containing_colons_gracefully() {
        let (name, id) = split_player("Weird:Name");
        assert_eq!(name, "Weird:Name");
        assert_eq!(id, None);
    }

    #[test]
    fn classifies_teams() {
        assert_eq!(service_bucket("Police"), "law_enforcement");
        assert_eq!(service_bucket("State Patrol"), "law_enforcement");
        assert_eq!(service_bucket("Fire"), "fire_ems");
        assert_eq!(service_bucket("DOT"), "fire_ems");
        assert_eq!(service_bucket("Civilian"), "civilian");
    }

    #[test]
    fn parses_v2_server_payload_shape() {
        // Field names taken from the published v2 OpenAPI schema.
        let json = serde_json::json!({
            "Name": "API Test",
            "OwnerId": 1,
            "CoOwnerIds": [2, 3],
            "CurrentPlayers": 2,
            "MaxPlayers": 40,
            "JoinKey": "APIServer",
            "AccVerifiedReq": "Phone/ID",
            "TeamBalance": true,
            "Players": [{
                "Team": "Police",
                "Player": "Builderman:156",
                "Callsign": "1A-1",
                "Permission": "Normal",
                "WantedStars": 0,
                "Location": {
                    "LocationX": 12.5,
                    "LocationZ": -40.0,
                    "PostalCode": "302",
                    "StreetName": "Park Avenue",
                    "BuildingNumber": "12"
                }
            }],
            "Staff": { "Admins": { "1": "Builderman" }, "Mods": {}, "Helpers": {} },
            "Vehicles": [{ "Name": "Falcon", "Owner": "Builderman", "Plate": "ABC-123", "Texture": null, "ColorHex": "#ffffff", "ColorName": "White" }],
            "EmergencyCalls": [{ "Team": "Police", "Caller": 156, "Players": [156], "Position": [1.0, 2.0], "StartedAt": 1704614400, "CallNumber": 1, "Description": "Test", "PositionDescriptor": "302 Park Avenue" }]
        });
        let info: ServerInfo = serde_json::from_value(json).unwrap();
        assert_eq!(info.name, "API Test");
        assert_eq!(info.players.as_ref().unwrap().len(), 1);
        assert_eq!(info.vehicles.as_ref().unwrap()[0].plate, "ABC-123");
        assert_eq!(info.emergency_calls.as_ref().unwrap()[0].call_number, 1);
        assert!(info.staff.as_ref().unwrap().admins.contains_key("1"));
    }

    #[test]
    fn parses_minimal_server_payload() {
        let json = serde_json::json!({
            "Name": "S",
            "OwnerId": 1,
            "CoOwnerIds": [],
            "CurrentPlayers": 0,
            "MaxPlayers": 10,
            "JoinKey": "x",
            "AccVerifiedReq": "Disabled",
            "TeamBalance": false
        });
        let info: ServerInfo = serde_json::from_value(json).unwrap();
        assert!(info.players.is_none());
        assert_eq!(info.max_players, 10);
    }
}
