import { invoke } from "@tauri-apps/api/core";
import type {
  ApiConfigView,
  ApiError,
  AuditLogView,
  AvatarView,
  BackupView,
  BoloView,
  BootstrapView,
  CallView,
  CommandHistoryView,
  CommandPolicyView,
  CommandResultView,
  ConnectionTestView,
  DatabaseStatsView,
  DutyShiftView,
  DutyStatusView,
  InternalBanView,
  LiveBanView,
  ModerationRecordView,
  PermissionInfo,
  PlayerProfile,
  RobloxUserView,
  RoleView,
  SearchResult,
  SessionView,
  SettingsUpdate,
  SettingsView,
  Snapshot,
  StaffNoteView,
  StaffUserView,
  StatsOverviewView,
  UnitView,
  UpdateCheckView,
  UpdateInstallView,
} from "@/types";

export function isApiError(value: unknown): value is ApiError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof (value as ApiError).message === "string"
  );
}

export function errorMessage(value: unknown): string {
  if (isApiError(value)) return value.message;
  if (value instanceof Error) return value.message;
  if (typeof value === "string") return value;
  return "An unexpected error occurred.";
}

/** Normalise any rejected IPC call into an ApiError. */
export async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isApiError(error)) throw error;
    throw { code: "internal", message: errorMessage(error) } satisfies ApiError;
  }
}

export const api = {
  bootstrap: () => call<BootstrapView>("app_bootstrap"),
  setupOwner: (username: string, displayName: string, password: string) =>
    call<SessionView>("setup_owner", { username, displayName, password }),
  login: (username: string, password: string) =>
    call<SessionView>("login", { username, password }),
  logout: (token: string) => call<null>("logout", { token }),
  sessionInfo: (token: string) => call<SessionView>("session_info", { token }),
  changePassword: (token: string, currentPassword: string, newPassword: string) =>
    call<null>("change_password", { token, currentPassword, newPassword }),

  snapshot: (token: string, force = false) =>
    call<Snapshot>("get_server_snapshot", { token, force }),

  searchPlayers: (token: string, query: string) =>
    call<SearchResult[]>("search_players", { token, query }),
  listKnownPlayers: (token: string, limit = 200) =>
    call<SearchResult[]>("list_known_players", { token, limit }),
  playerProfile: (token: string, robloxUserId: number | null, username: string | null) =>
    call<PlayerProfile>("get_player_profile", { token, robloxUserId, username }),
  avatar: (token: string, robloxUserId: number, fullBody = false) =>
    call<AvatarView>("get_player_avatar", { token, robloxUserId, fullBody }),
  lookupRoblox: (token: string, identifier: string) =>
    call<RobloxUserView>("lookup_roblox_user", { token, identifier }),
  addStaffNote: (token: string, robloxUserId: number | null, username: string, body: string) =>
    call<StaffNoteView>("add_staff_note", { token, robloxUserId, username, body }),
  listStaffNotes: (token: string, robloxUserId: number | null, username: string) =>
    call<StaffNoteView[]>("list_staff_notes", { token, robloxUserId, username }),
  deleteStaffNote: (token: string, noteId: number) =>
    call<null>("delete_staff_note", { token, noteId }),

  kickPlayer: (
    token: string,
    target: string,
    reason: string,
    confirmed: boolean,
    robloxUserId: number | null,
  ) => call<CommandResultView>("kick_player", { token, target, reason, confirmed, robloxUserId }),
  banPlayer: (
    token: string,
    target: string,
    reason: string,
    confirmed: boolean,
    robloxUserId: number | null,
  ) => call<CommandResultView>("ban_player", { token, target, reason, confirmed, robloxUserId }),
  unbanPlayer: (
    token: string,
    target: string,
    reason: string,
    confirmed: boolean,
    robloxUserId: number | null,
  ) => call<CommandResultView>("unban_player", { token, target, reason, confirmed, robloxUserId }),
  createInternalBan: (
    token: string,
    robloxUserId: number | null,
    username: string,
    reason: string,
  ) => call<InternalBanView>("create_internal_ban", { token, robloxUserId, username, reason }),
  liftInternalBan: (token: string, banId: number, reason: string) =>
    call<null>("lift_internal_ban", { token, banId, reason }),
  listModeration: (token: string, limit = 100) =>
    call<ModerationRecordView[]>("list_moderation", { token, limit }),
  listPlayerModeration: (
    token: string,
    robloxUserId: number | null,
    username: string,
    limit = 100,
  ) => call<ModerationRecordView[]>("list_player_moderation", { token, robloxUserId, username, limit }),
  listLiveBans: (token: string) => call<LiveBanView[]>("list_live_bans", { token }),
  listInternalBans: (token: string, activeOnly = false, limit = 200) =>
    call<InternalBanView[]>("list_internal_bans", { token, activeOnly, limit }),

  listBolos: (token: string, status: string | null) =>
    call<BoloView[]>("list_bolos", { token, status }),
  createBolo: (
    token: string,
    payload: {
      kind: string;
      subjectName: string;
      plate: string;
      description: string;
      lastSeen: string;
      priority: string;
    },
  ) => call<BoloView>("create_bolo", { token, ...payload }),
  clearBolo: (token: string, boloId: number, note: string) =>
    call<BoloView>("clear_bolo", { token, boloId, note }),
  reopenBolo: (token: string, boloId: number) =>
    call<BoloView>("reopen_bolo", { token, boloId }),

  dutyStatus: (token: string) => call<DutyStatusView>("duty_status", { token }),
  clockIn: (token: string) => call<DutyStatusView>("clock_in", { token }),
  clockOut: (token: string) => call<DutyStatusView>("clock_out", { token }),
  listShifts: (token: string, mineOnly = false) =>
    call<DutyShiftView[]>("list_shifts", { token, mineOnly }),
  endShift: (token: string, userId: number, reason: string) =>
    call<DutyShiftView>("end_shift", { token, userId, reason }),

  statsOverview: (token: string, days: number) =>
    call<StatsOverviewView>("stats_overview", { token, days }),

  checkForUpdate: (token: string) => call<UpdateCheckView>("check_for_update", { token }),
  installUpdate: (token: string, confirmed: boolean) =>
    call<UpdateInstallView>("install_update", { token, confirmed }),

  listCalls: (token: string, status: string | null, limit = 100) =>
    call<CallView[]>("list_calls", { token, status, limit }),
  createCall: (
    token: string,
    payload: {
      title: string;
      description: string;
      location: string;
      postal: string;
      priority: string;
    },
  ) => call<CallView>("create_call", { token, ...payload }),
  updateCall: (
    token: string,
    callId: number,
    payload: {
      title: string;
      description: string;
      location: string;
      postal: string;
      priority: string;
    },
  ) => call<CallView>("update_call", { token, callId, ...payload }),
  setCallStatus: (token: string, callId: number, status: string) =>
    call<CallView>("set_call_status", { token, callId, status }),
  attachPlayerToCall: (
    token: string,
    callId: number,
    robloxUserId: number,
    username: string,
  ) => call<CallView>("attach_player_to_call", { token, callId, robloxUserId, username }),
  detachPlayerFromCall: (token: string, callId: number, robloxUserId: number) =>
    call<CallView>("detach_player_from_call", { token, callId, robloxUserId }),
  listUnits: (token: string) => call<UnitView[]>("list_units", { token }),
  createUnit: (token: string, callsign: string, label: string, team: string, status: string) =>
    call<UnitView>("create_unit", { token, callsign, label, team, status }),
  updateUnit: (token: string, unitId: number, label: string, team: string, status: string) =>
    call<UnitView>("update_unit", { token, unitId, label, team, status }),
  assignUnit: (token: string, callId: number, unitId: number) =>
    call<CallView>("assign_unit_to_call", { token, callId, unitId }),
  unassignUnit: (token: string, unitId: number) =>
    call<null>("unassign_unit", { token, unitId }),
  deleteUnit: (token: string, unitId: number) => call<null>("delete_unit", { token, unitId }),

  executeCommand: (token: string, commandText: string, confirmed: boolean) =>
    call<CommandResultView>("execute_server_command", { token, commandText, confirmed }),
  commandHistory: (token: string, limit = 200) =>
    call<CommandHistoryView[]>("list_command_history", { token, limit }),
  commandPolicies: (token: string) =>
    call<CommandPolicyView[]>("list_command_policies", { token }),
  upsertCommandPolicy: (
    token: string,
    commandText: string,
    minPermission: string,
    destructive: boolean,
    enabled: boolean,
    description: string,
  ) =>
    call<null>("upsert_command_policy", {
      token,
      commandText,
      minPermission,
      destructive,
      enabled,
      description,
    }),
  deleteCommandPolicy: (token: string, commandText: string) =>
    call<null>("delete_command_policy", { token, commandText }),

  listStaff: (token: string) => call<StaffUserView[]>("list_staff", { token }),
  listRoles: (token: string) => call<RoleView[]>("list_roles", { token }),
  listPermissions: (token: string) =>
    call<PermissionInfo[]>("list_permission_catalogue", { token }),
  createStaff: (
    token: string,
    username: string,
    displayName: string,
    password: string,
    roleKey: string,
    robloxUserId: number | null,
  ) => call<StaffUserView>("create_staff", { token, username, displayName, password, roleKey, robloxUserId }),
  updateStaffRole: (token: string, userId: number, roleKey: string) =>
    call<null>("update_staff_role", { token, userId, roleKey }),
  setStaffActive: (token: string, userId: number, active: boolean) =>
    call<null>("set_staff_active", { token, userId, active }),
  resetStaffPassword: (token: string, userId: number, newPassword: string) =>
    call<null>("reset_staff_password", { token, userId, newPassword }),
  setPermissionOverride: (
    token: string,
    userId: number,
    permissionKey: string,
    allowed: boolean | null,
  ) => call<null>("set_permission_override", { token, userId, permissionKey, allowed }),
  createRole: (
    token: string,
    key: string,
    name: string,
    description: string,
    permissions: string[],
  ) => call<number>("create_role", { token, key, name, description, permissions }),
  updateRoleDefinition: (
    token: string,
    roleId: number,
    name: string,
    description: string,
    permissions: string[],
  ) => call<null>("update_role_definition", { token, roleId, name, description, permissions }),
  deleteRole: (token: string, roleId: number) => call<null>("delete_role", { token, roleId }),
  auditLogs: (token: string, category: string | null, query: string | null, limit = 200) =>
    call<AuditLogView[]>("list_audit_logs", { token, category, query, limit }),
  auditCategories: (token: string) => call<string[]>("list_audit_categories", { token }),

  getSettings: (token: string) => call<SettingsView>("get_settings", { token }),
  updateSettings: (token: string, update: SettingsUpdate) =>
    call<SettingsView>("update_settings", { token, update }),
  getApiConfig: (token: string) => call<ApiConfigView>("get_api_config", { token }),
  saveApiKey: (token: string, baseUrl: string, apiKey: string) =>
    call<ApiConfigView>("save_api_key", { token, baseUrl, apiKey }),
  clearApiKey: (token: string) => call<null>("clear_api_key", { token }),
  testApiConnection: (token: string, candidateKey: string | null) =>
    call<ConnectionTestView>("test_api_connection", { token, candidateKey }),
  backupDatabase: (token: string, directory: string | null) =>
    call<BackupView>("backup_database", { token, directory }),
  restoreDatabase: (token: string, source: string, confirmed: boolean) =>
    call<BackupView>("restore_database", { token, source, confirmed }),
  databaseStats: (token: string) => call<DatabaseStatsView>("database_stats", { token }),
};
