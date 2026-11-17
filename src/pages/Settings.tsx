import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { DatabaseBackup, FolderOpen, HardDriveDownload, RotateCcw, Save, ShieldAlert } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatBytes, formatDateTime, formatNumber } from "@/lib/format";
import { MAP_OPTIONS } from "@/lib/map";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input, Select, Toggle } from "@/components/ui/Field";
import { ConfirmDialog } from "@/components/ui/Modal";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { KeyValue, Panel, PanelBody, PanelHeader } from "@/components/ui/Panel";
import type { DatabaseStatsView, SettingsView } from "@/types";

const STARTUP_PAGES = [
  { value: "dashboard", label: "Dashboard" },
  { value: "dispatch", label: "CAD Dispatch" },
  { value: "map", label: "Live Map" },
  { value: "players", label: "Players" },
  { value: "moderation", label: "Moderation" },
  { value: "commands", label: "Server Commands" },
  { value: "logs", label: "Server Logs" },
  { value: "settings", label: "Settings" },
];

export default function SettingsPage() {
  const { token, bootstrap, isOwner, signOut } = useSession();
  const { settings, saveSettings, reloadSettings, refresh } = useSnapshot();
  const toast = useToast();
  const [draft, setDraft] = useState<SettingsView | null>(null);
  const [saving, setSaving] = useState(false);
  const [backingUp, setBackingUp] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [restorePath, setRestorePath] = useState("");
  const [restoreOpen, setRestoreOpen] = useState(false);
  const [lastBackupPath, setLastBackupPath] = useState<string | null>(null);
  const [passwordDraft, setPasswordDraft] = useState({ current: "", next: "", confirm: "" });
  const [changingPassword, setChangingPassword] = useState(false);
  const [passwordError, setPasswordError] = useState<string | null>(null);

  const stats = useAsync<DatabaseStatsView | null>(
    () => (token ? api.databaseStats(token) : Promise.resolve(null)),
    [token],
    Boolean(token),
  );

  useEffect(() => {
    if (settings) setDraft(settings);
  }, [settings]);

  function patch(next: Partial<SettingsView>) {
    setDraft((current) => (current ? { ...current, ...next } : current));
  }

  async function save() {
    if (!draft) return;
    setSaving(true);
    try {
      await saveSettings({
        refreshIntervalSeconds: draft.refreshIntervalSeconds,
        startupPage: draft.startupPage,
        autoConnect: draft.autoConnect,
        confirmDestructive: draft.confirmDestructive,
        compactTables: draft.compactTables,
        mapImage: draft.mapImage,
        mapScale: draft.mapScale,
        sessionIdleMinutes: draft.sessionIdleMinutes,
        backupDirectory: draft.backupDirectory,
        autoBackupHours: draft.autoBackupHours,
        notifyEmergencyCalls: draft.notifyEmergencyCalls,
        notifyModCalls: draft.notifyModCalls,
        notifyConnection: draft.notifyConnection,
      });
      toast.success("Settings saved");
      await reloadSettings();
      await refresh(true);
    } catch (err) {
      toast.error("Could not save settings", errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  async function backupNow() {
    if (!token) return;
    setBackingUp(true);
    try {
      const view = await api.backupDatabase(token, draft?.backupDirectory.trim() ? draft.backupDirectory.trim() : null);
      setLastBackupPath(view.path);
      toast.success("Database backed up", view.path);
      await reloadSettings();
      await stats.reload();
    } catch (err) {
      toast.error("Backup failed", errorMessage(err));
    } finally {
      setBackingUp(false);
    }
  }

  async function changePassword() {
    if (!token) return;
    setPasswordError(null);
    if (passwordDraft.next !== passwordDraft.confirm) {
      setPasswordError("The new passwords do not match.");
      return;
    }
    if (passwordDraft.next.length < 10) {
      setPasswordError("Use at least 10 characters.");
      return;
    }
    setChangingPassword(true);
    try {
      await api.changePassword(token, passwordDraft.current, passwordDraft.next);
      toast.success("Password changed");
      setPasswordDraft({ current: "", next: "", confirm: "" });
    } catch (err) {
      const message = errorMessage(err);
      setPasswordError(message);
      toast.error("Could not change password", message);
    } finally {
      setChangingPassword(false);
    }
  }

  if (!draft) {
    return (
      <div>
        <PageHeader title="Settings" />
        <Panel>
          <PanelBody>
            <TableSkeleton rows={5} columns={2} />
          </PanelBody>
        </Panel>
      </div>
    );
  }

  return (
    <div>
      <PageHeader
        title="Settings"
        description="Local application preferences. Changes apply to this installation only and are recorded in the audit log."
        actions={
          <Button variant="primary" size="sm" onClick={save} loading={saving} leadingIcon={<Save size={13} />}>
            Save settings
          </Button>
        }
      />

      <div className="grid gap-4 xl:grid-cols-2">
        <Panel>
          <PanelHeader title="Refresh and interface" description="Polling cadence and table density." />
          <PanelBody className="space-y-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <Input
                label="Automatic refresh (seconds)"
                type="number"
                min={5}
                max={300}
                hint="5 – 300"
                value={draft.refreshIntervalSeconds}
                onChange={(event) => patch({ refreshIntervalSeconds: Number(event.target.value) })}
              />
              <Select
                label="Startup page"
                value={draft.startupPage}
                onChange={(event) => patch({ startupPage: event.target.value })}
              >
                {STARTUP_PAGES.map((page) => (
                  <option key={page.value} value={page.value}>
                    {page.label}
                  </option>
                ))}
              </Select>
            </div>
            <div className="divide-y divide-line">
              <Toggle
                label="Connect automatically on launch"
                description="Fetch live server data as soon as you sign in."
                checked={draft.autoConnect}
                onChange={(next) => patch({ autoConnect: next })}
              />
              <Toggle
                label="Confirm destructive actions"
                description="Require a confirmation dialog for kicks, bans and destructive commands."
                checked={draft.confirmDestructive}
                onChange={(next) => patch({ confirmDestructive: next })}
              />
              <Toggle
                label="Compact tables"
                description="Tighter row height for high-density dispatch work."
                checked={draft.compactTables}
                onChange={(next) => patch({ compactTables: next })}
              />
            </div>
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader
            title="Map"
            description="Official ER:LC imagery and the operator-calibrated world-to-pixel scale."
          />
          <PanelBody className="space-y-3">
            <Select
              label="Map image"
              value={draft.mapImage}
              onChange={(event) => patch({ mapImage: event.target.value })}
            >
              {MAP_OPTIONS.map((option) => (
                <option key={option.url} value={option.url}>
                  {option.label}
                </option>
              ))}
            </Select>
            <Input
              label="Pixels per world unit"
              type="number"
              step="0.1"
              min="0.2"
              max="20"
              hint="0.2 – 20"
              value={draft.mapScale}
              onChange={(event) => patch({ mapScale: Number(event.target.value) })}
            />
            <p className="rounded border border-line bg-surface-2 px-2.5 py-2 text-[11px] leading-5 text-muted">
              ER:LC publishes a 5355 × 5355 map but not the world-unit scale. Calibrate this value on
              the Live Map so markers line up with postal codes.
            </p>
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader title="Notifications" description="In-app alerts for live activity." />
          <PanelBody className="divide-y divide-line">
            <Toggle
              label="New emergency calls"
              description="Notify when ER:LC reports a new call."
              checked={draft.notifyEmergencyCalls}
              onChange={(next) => patch({ notifyEmergencyCalls: next })}
            />
            <Toggle
              label="Moderator calls"
              description="Notify when a player requests in-game staff help."
              checked={draft.notifyModCalls}
              onChange={(next) => patch({ notifyModCalls: next })}
            />
            <Toggle
              label="Connection changes"
              description="Notify when the API connection drops or recovers."
              checked={draft.notifyConnection}
              onChange={(next) => patch({ notifyConnection: next })}
            />
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader title="Local security" description="Session lifetime and your account password." />
          <PanelBody className="space-y-3">
            <Input
              label="Session idle timeout (minutes)"
              type="number"
              min={5}
              max={480}
              hint="5 – 480"
              value={draft.sessionIdleMinutes}
              onChange={(event) => patch({ sessionIdleMinutes: Number(event.target.value) })}
            />
            <div className="space-y-2 rounded border border-line bg-surface-2/40 p-3">
              <p className="text-[12px] font-medium text-ink">Change password</p>
              <Input
                label="Current password"
                type="password"
                autoComplete="current-password"
                value={passwordDraft.current}
                onChange={(event) => setPasswordDraft({ ...passwordDraft, current: event.target.value })}
              />
              <div className="grid gap-2 sm:grid-cols-2">
                <Input
                  label="New password"
                  type="password"
                  autoComplete="new-password"
                  hint="min 10"
                  value={passwordDraft.next}
                  onChange={(event) => setPasswordDraft({ ...passwordDraft, next: event.target.value })}
                />
                <Input
                  label="Confirm"
                  type="password"
                  autoComplete="new-password"
                  value={passwordDraft.confirm}
                  onChange={(event) => setPasswordDraft({ ...passwordDraft, confirm: event.target.value })}
                />
              </div>
              {passwordError ? <InlineError message={passwordError} /> : null}
              <div className="flex justify-end">
                <Button
                  size="sm"
                  variant="secondary"
                  onClick={changePassword}
                  loading={changingPassword}
                  disabled={
                    passwordDraft.current.length === 0 ||
                    passwordDraft.next.length < 10 ||
                    passwordDraft.confirm.length < 10
                  }
                >
                  Update password
                </Button>
              </div>
            </div>
          </PanelBody>
        </Panel>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-2">
        <Panel>
          <PanelHeader
            title="Database backup"
            description="SQLite online backup. Leaving the folder empty uses the application data directory."
            actions={
              <Badge tone="neutral">
                last {formatDateTime(settings?.lastBackupAt ?? null)}
              </Badge>
            }
          />
          <PanelBody className="space-y-3">
            <div className="flex items-end gap-2">
              <div className="min-w-0 flex-1">
                <Input
                  label="Backup folder (absolute path)"
                  placeholder="C:\\Avero CAD\\backups"
                  value={draft.backupDirectory}
                  onChange={(event) => patch({ backupDirectory: event.target.value })}
                  spellCheck={false}
                  className="mono"
                />
              </div>
              <Button
                size="sm"
                variant="secondary"
                onClick={async () => {
                  try {
                    const selected = await open({
                      directory: true,
                      multiple: false,
                      title: "Select the backup folder",
                    });
                    if (typeof selected === "string") patch({ backupDirectory: selected });
                  } catch (err) {
                    toast.error("Could not open the folder picker", errorMessage(err));
                  }
                }}
                leadingIcon={<FolderOpen size={13} />}
              >
                Browse
              </Button>
            </div>
            <Input
              label="Automatic backup interval (hours)"
              type="number"
              min={0}
              max={720}
              hint="0 disables"
              value={draft.autoBackupHours}
              onChange={(event) => patch({ autoBackupHours: Number(event.target.value) })}
            />
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="secondary"
                onClick={backupNow}
                loading={backingUp}
                leadingIcon={<DatabaseBackup size={13} />}
              >
                Back up now
              </Button>
              <Button
                size="sm"
                variant="secondary"
                onClick={() => setRestoreOpen(true)}
                disabled={restorePath.trim().length === 0}
                leadingIcon={<HardDriveDownload size={13} />}
              >
                Restore from file
              </Button>
            </div>
            <div className="flex items-end gap-2">
              <div className="min-w-0 flex-1">
                <Input
                  label="Backup file to restore"
                  placeholder="C:\\Avero CAD\\backups\\avero-cad-…db"
                  value={restorePath}
                  onChange={(event) => setRestorePath(event.target.value)}
                  spellCheck={false}
                  className="mono"
                />
              </div>
              <Button
                size="sm"
                variant="secondary"
                onClick={async () => {
                  try {
                    const selected = await open({
                      multiple: false,
                      title: "Select a database backup",
                      filters: [{ name: "SQLite database", extensions: ["db", "sqlite", "bak"] }],
                    });
                    if (typeof selected === "string") setRestorePath(selected);
                  } catch (err) {
                    toast.error("Could not open the file picker", errorMessage(err));
                  }
                }}
                leadingIcon={<FolderOpen size={13} />}
              >
                Browse
              </Button>
            </div>
            {lastBackupPath ? (
              <p className="mono break-all rounded border border-line bg-surface-2 px-2.5 py-2 text-[10px] text-muted">
                {lastBackupPath}
              </p>
            ) : null}
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader
            title="Database"
            description="Local SQLite file with staff accounts, records, calls and audit history."
            actions={
              <Button
                size="sm"
                variant="ghost"
                onClick={() => void stats.reload()}
                leadingIcon={<RotateCcw size={12} />}
              >
                Refresh
              </Button>
            }
          />
          {stats.error ? (
            <div className="p-3">
              <InlineError message={stats.error} />
            </div>
          ) : !stats.data ? (
            <PanelBody>
              <TableSkeleton rows={4} columns={2} />
            </PanelBody>
          ) : (
            <PanelBody>
              <div className="space-y-0.5">
                <KeyValue label="File" value={stats.data.path} mono />
                <KeyValue label="Size" value={formatBytes(stats.data.sizeBytes)} />
                <KeyValue label="Players tracked" value={formatNumber(stats.data.players)} />
                <KeyValue label="Moderation records" value={formatNumber(stats.data.moderationRecords)} />
                <KeyValue label="Internal bans" value={formatNumber(stats.data.internalBans)} />
                <KeyValue label="CAD calls" value={formatNumber(stats.data.calls)} />
                <KeyValue label="Units" value={formatNumber(stats.data.units)} />
                <KeyValue label="Staff accounts" value={formatNumber(stats.data.users)} />
                <KeyValue label="Command executions" value={formatNumber(stats.data.commands)} />
                <KeyValue label="Audit entries" value={formatNumber(stats.data.auditLogs)} />
              </div>
            </PanelBody>
          )}
        </Panel>
      </div>

      <div className="mt-4">
        <Panel>
          <PanelHeader title="Application" description="Build and local paths for this installation." />
          <PanelBody>
            <div className="grid gap-x-8 gap-y-0.5 md:grid-cols-2">
              <KeyValue label="Version" value={bootstrap?.appVersion ?? "—"} mono />
              <KeyValue label="Platform" value={bootstrap?.platform ?? "—"} mono />
              <KeyValue label="API key" value={bootstrap?.apiConfigured ? "Configured" : "Not configured"} />
              <KeyValue label="Role" value={isOwner ? "Server Owner" : "Staff"} />
              <KeyValue label="Database" value={bootstrap?.databasePath ?? "—"} mono />
              <KeyValue label="Logs" value={bootstrap?.logPath ?? "—"} mono />
            </div>
          </PanelBody>
        </Panel>
      </div>

      <p className="mt-3 flex items-start gap-2 text-[11px] leading-5 text-faint">
        <ShieldAlert size={13} className="mt-0.5 shrink-0" />
        Restoring a backup replaces staff accounts and operational data. You are signed out afterwards
        and must sign in with an account from the restored file.
      </p>

      <ConfirmDialog
        open={restoreOpen}
        onClose={() => setRestoreOpen(false)}
        title="Restore database"
        description="The current database is replaced by the selected backup. All staff sessions are cleared."
        confirmLabel="Restore and sign out"
        destructive
        loading={restoring}
        onConfirm={async () => {
          if (!token) return;
          setRestoring(true);
          try {
            await api.restoreDatabase(token, restorePath.trim(), true);
            toast.success("Database restored", "Sign in with an account from the backup.");
            setRestoreOpen(false);
            await signOut();
          } catch (err) {
            toast.error("Restore failed", errorMessage(err));
          } finally {
            setRestoring(false);
          }
        }}
      >
        <p className="mono break-all rounded border border-line bg-surface-2 px-2.5 py-2 text-[11px] text-ink">
          {restorePath.trim() || "No file selected"}
        </p>
      </ConfirmDialog>
    </div>
  );
}
