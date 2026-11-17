// Types mirror the Rust view models in crates/avero-cad-core/src/models.rs.


export interface ApiError {
  code: string;
  message: string;
  retryAfter?: number | null;
  status?: number | null;
  commandId?: string | null;
}

export interface BootstrapView {
  needsSetup: boolean;
  appVersion: string;
  apiConfigured: boolean;
  apiKeyLast4: string | null;
  apiBaseUrl: string;
  platform: string;
  databasePath: string;
  logPath: string;
}

export interface PermissionOverrideView {
  permissionKey: string;
  allowed: boolean;
}

export interface StaffUserView {
  id: number;
  username: string;
  displayName: string;
  roleId: number;
  roleKey: string;
  roleName: string;
  robloxUserId: number | null;
  isActive: boolean;
  createdAt: number;
  lastLoginAt: number | null;
  overrides: PermissionOverrideView[];
}

export interface SessionView {
  token: string;
  user: StaffUserView;
  permissions: string[];
  expiresAt: number;
  isOwner: boolean;
}

export interface PermissionInfo {
  key: string;
  label: string;
  category: string;
  description: string;
}

export interface RoleView {
  id: number;
  key: string;
  name: string;
  description: string;
  isSystem: boolean;
  permissions: string[];
  userCount: number;
}

export interface RateLimitView {
  bucket: string | null;
  limit: number | null;
  remaining: number | null;
  reset: number | null;
}

export interface ServerSummary {
  name: string;
  ownerId: number;
  coOwnerIds: number[];
  currentPlayers: number;
  maxPlayers: number;
  joinKey: string;
  accVerifiedReq: string;
  teamBalance: boolean;
}

export interface PlayerLocationView {
  x: number;
  z: number;
  postal: string | null;
  street: string | null;
  building: string | null;
}

export interface PlayerView {
  robloxUserId: number | null;
  username: string;
  team: string;
  permission: string;
  callsign: string | null;
  location: PlayerLocationView | null;
  wantedStars: number;
  service: 'law_enforcement' | 'fire_ems' | 'civilian';
  staffRole: string | null;
}

export interface StaffMember {
  robloxUserId: number;
  username: string;
  role: string;
}

export interface StaffRoster {
  admins: StaffMember[];
  mods: StaffMember[];
  helpers: StaffMember[];
}

export interface VehicleView {
  name: string;
  owner: string;
  plate: string;
  texture: string | null;
  colorHex: string;
  colorName: string;
}

export interface EmergencyCallView {
  callNumber: number;
  team: string;
  callerId: number;
  playerIds: number[];
  description: string;
  positionDescriptor: string;
  x: number | null;
  z: number | null;
  startedAt: number;
}

export interface JoinLogView {
  robloxUserId: number | null;
  username: string;
  joined: boolean;
  timestamp: number;
}

export interface KillLogView {
  killer: string;
  killed: string;
  timestamp: number;
}

export interface CommandLogView {
  player: string;
  command: string;
  timestamp: number;
}

export interface ModCallView {
  caller: string;
  moderator: string | null;
  timestamp: number;
}

export interface QueueEntryView {
  robloxUserId: number;
  position: number;
}

export interface RosterCounts {
  total: number;
  lawEnforcement: number;
  fireEms: number;
  civilian: number;
  staff: number;
  wanted: number;
}

export interface ConnectionView {
  state: 'connected' | 'error' | 'not_configured';
  detail: string | null;
  apiUrl: string;
  latencyMs: number | null;
  rateLimit: RateLimitView | null;
}

export interface AppNotification {
  id: string;
  kind: 'emergency' | 'modcall' | 'connection';
  title: string;
  detail: string;
  at: number;
}

export interface MetricPoint {
  capturedAt: number;
  playerCount: number;
  maxPlayers: number;
  latencyMs: number | null;
}

export interface Snapshot {
  server: ServerSummary | null;
  players: PlayerView[];
  staff: StaffRoster;
  vehicles: VehicleView[];
  emergencyCalls: EmergencyCallView[];
  joinLogs: JoinLogView[];
  killLogs: KillLogView[];
  commandLogs: CommandLogView[];
  modCalls: ModCallView[];
  queue: QueueEntryView[];
  counts: RosterCounts;
  connection: ConnectionView;
  notifications: AppNotification[];
  history: MetricPoint[];
  fetchedAt: number;
}

export interface RobloxUserView {
  id: number;
  name: string;
  displayName: string;
  description: string;
  created: string;
  isBanned: boolean;
  hasVerifiedBadge: boolean;
}

export interface AvatarView {
  robloxUserId: number;
  dataUrl: string;
  cached: boolean;
  fetchedAt: number;
}

export interface SearchResult {
  robloxUserId: number | null;
  username: string;
  online: boolean;
  team: string | null;
  callsign: string | null;
  permission: string | null;
  source: 'live' | 'database';
}

export interface ModerationRecordView {
  id: number;
  robloxUserId: number | null;
  username: string;
  action: 'kick' | 'ban' | 'unban' | 'internal_ban' | 'internal_unban' | 'note';
  source: 'erlc_api' | 'internal_cad';
  reason: string;
  moderatorName: string;
  success: boolean;
  apiMessage: string | null;
  commandId: string | null;
  createdAt: number;
}

export interface InternalBanView {
  id: number;
  robloxUserId: number | null;
  username: string;
  reason: string;
  createdByName: string;
  createdAt: number;
  active: boolean;
  liftedAt: number | null;
  liftedByName: string | null;
  liftReason: string | null;
}

export interface StaffNoteView {
  id: number;
  robloxUserId: number | null;
  username: string;
  authorName: string;
  body: string;
  createdAt: number;
  updatedAt: number;
}

export interface PlayerProfile {
  username: string;
  robloxUserId: number | null;
  roblox: RobloxUserView | null;
  live: PlayerView | null;
  firstSeenAt: number | null;
  lastSeenAt: number | null;
  moderation: ModerationRecordView[];
  notes: StaffNoteView[];
  internalBan: InternalBanView | null;
  erlcBan: boolean;
  erlcBanDetail: string | null;
  internalBanCount: number;
}

export type UnitStatus =
  | 'available'
  | 'on_duty'
  | 'en_route'
  | 'on_scene'
  | 'transporting'
  | 'unavailable'
  | 'off_duty';

export interface UnitView {
  id: number;
  callsign: string;
  label: string;
  team: string;
  status: UnitStatus;
  robloxUserId: number | null;
  assignedCallId: number | null;
  updatedAt: number;
}

export interface CallPlayerView {
  robloxUserId: number;
  username: string;
}

export type CallStatus = 'open' | 'assigned' | 'in_progress' | 'closed';
export type CallPriority = 'low' | 'medium' | 'high' | 'critical';

export interface CallView {
  id: number;
  callNumber: number;
  title: string;
  description: string;
  location: string;
  postal: string;
  priority: CallPriority;
  status: CallStatus;
  source: 'cad' | 'erlc';
  erlcCallNumber: number | null;
  createdByName: string;
  createdAt: number;
  updatedAt: number;
  closedAt: number | null;
  closedByName: string | null;
  units: UnitView[];
  players: CallPlayerView[];
}

export interface CommandResultView {
  status: 'success' | 'failed' | 'rejected';
  message: string;
  commandId: string | null;
  httpStatus: number | null;
  durationMs: number;
  executedAt: number;
}

export interface CommandHistoryView {
  id: number;
  actorName: string;
  command: string;
  status: 'success' | 'failed' | 'rejected';
  response: string;
  httpStatus: number | null;
  commandId: string | null;
  durationMs: number;
  createdAt: number;
}

export interface CommandPolicyView {
  command: string;
  minPermission: string;
  destructive: boolean;
  enabled: boolean;
  description: string;
}

export interface AuditLogView {
  id: number;
  actorName: string;
  category: string;
  action: string;
  target: string;
  details: string;
  createdAt: number;
}

export interface SettingsView {
  refreshIntervalSeconds: number;
  startupPage: string;
  autoConnect: boolean;
  confirmDestructive: boolean;
  compactTables: boolean;
  mapImage: string;
  mapScale: number;
  sessionIdleMinutes: number;
  backupDirectory: string;
  autoBackupHours: number;
  notifyEmergencyCalls: boolean;
  notifyModCalls: boolean;
  notifyConnection: boolean;
  lastBackupAt: number | null;
}

export interface SettingsUpdate {
  refreshIntervalSeconds?: number;
  startupPage?: string;
  autoConnect?: boolean;
  confirmDestructive?: boolean;
  compactTables?: boolean;
  mapImage?: string;
  mapScale?: number;
  sessionIdleMinutes?: number;
  backupDirectory?: string;
  autoBackupHours?: number;
  notifyEmergencyCalls?: boolean;
  notifyModCalls?: boolean;
  notifyConnection?: boolean;
}

export interface ApiConfigView {
  baseUrl: string;
  configured: boolean;
  keyLast4: string | null;
  keyFingerprint: string | null;
  keyStorage: string;
  lastVerifiedAt: number | null;
  serverName: string | null;
  serverId: number | null;
}

export interface ConnectionTestView {
  ok: boolean;
  message: string;
  latencyMs: number;
  serverName: string | null;
  currentPlayers: number | null;
  maxPlayers: number | null;
  rateLimit: RateLimitView | null;
}

export interface DatabaseStatsView {
  path: string;
  sizeBytes: number;
  players: number;
  moderationRecords: number;
  internalBans: number;
  calls: number;
  units: number;
  users: number;
  auditLogs: number;
  commands: number;
}

export interface BackupView {
  path: string;
  sizeBytes: number;
  createdAt: number;
}

export interface LiveBanView {
  username: string | null;
  robloxUserId: number | null;
  raw: string;
}

export type BoloKind = 'person' | 'vehicle';
export type BoloStatus = 'active' | 'cleared';
export type BoloPriority = 'low' | 'medium' | 'high' | 'critical';

export interface BoloView {
  id: number;
  kind: BoloKind;
  subjectName: string;
  plate: string;
  description: string;
  lastSeen: string;
  priority: BoloPriority;
  status: BoloStatus;
  createdByName: string;
  createdAt: number;
  clearedByName: string | null;
  clearedAt: number | null;
  clearanceNote: string | null;
  /** Where the subject was last matched in the live snapshot, if anywhere. */
  liveMatch: string | null;
}

export interface DutyShiftView {
  id: number;
  userId: number;
  displayName: string;
  startedAt: number;
  endedAt: number | null;
  durationMs: number;
  onDuty: boolean;
  endReason: string | null;
}

export interface DutyStatusView {
  current: DutyShiftView | null;
  todayMs: number;
  weekMs: number;
  totalMs: number;
  shiftCount: number;
  roster: DutyShiftView[];
  recent: DutyShiftView[];
  canViewRoster: boolean;
}

export interface DailyStatView {
  date: string;
  callsOpened: number;
  callsClosed: number;
  moderation: number;
  commands: number;
  peakPlayers: number;
}

export interface HourStatView {
  hour: number;
  calls: number;
  moderation: number;
  commands: number;
}

export interface TopModeratorView {
  name: string;
  actions: number;
  failed: number;
}

export type InstallKind = "nsis" | "msi" | "unknown";

export interface PendingUpdateView {
  version: string;
  startedAt: number;
  installerPath: string;
  sha256: string;
  stalled: boolean;
}

export interface UpdateCheckView {
  currentVersion: string;
  latestVersion: string | null;
  available: boolean;
  releaseUrl: string | null;
  publishedAt: string | null;
  notes: string | null;
  assetName: string | null;
  assetSize: number | null;
  canInstall: boolean;
  installKind: InstallKind;
  installSupported: boolean;
  manualUrl: string | null;
  pending: PendingUpdateView | null;
  detail: string;
}

export interface UpdateInstallView {
  version: string;
  installerPath: string;
  sha256: string;
  sizeBytes: number;
  installKind: InstallKind;
}

export interface StatsOverviewView {
  rangeDays: number;
  since: number;
  callsOpened: number;
  callsClosed: number;
  callsOpenNow: number;
  avgCloseMs: number | null;
  moderationTotal: number;
  moderationFailed: number;
  kicks: number;
  bans: number;
  unbans: number;
  internalBans: number;
  commandsTotal: number;
  commandsFailed: number;
  bolosActive: number;
  bolosCleared: number;
  playersTracked: number;
  playersNew: number;
  dutyMs: number;
  dutyShifts: number;
  onDutyNow: number;
  daily: DailyStatView[];
  hours: HourStatView[];
  topModerators: TopModeratorView[];
}
