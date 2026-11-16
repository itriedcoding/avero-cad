import { useState } from "react";
import { Ban, Check, Copy, RefreshCw, ShieldAlert, UserX } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, formatRelative, truncate } from "@/lib/format";
import { actionLabel, actionTone, sourceLabel } from "@/lib/status";
import { useSession } from "@/state/session";
import { useToast } from "@/state/toast";
import { Avatar } from "@/components/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { KeyValue, Panel, PanelBody, PanelHeader, SectionTitle } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import { FieldLabel, Textarea } from "@/components/ui/Field";
import { EmptyState, InlineError, Skeleton } from "@/components/ui/Feedback";
import { ModerationActionDialog, type ModerationAction } from "@/components/ModerationActionDialog";
import type { ModerationRecordView, PlayerProfile } from "@/types";

export function PlayerProfilePanel({
  profile,
  loading,
  error,
  onReload,
  onModerated,
}: {
  profile: PlayerProfile | null;
  loading: boolean;
  error: string | null;
  onReload: () => void;
  onModerated?: () => void;
}) {
  const { token, has } = useSession();
  const toast = useToast();
  const [note, setNote] = useState("");
  const [savingNote, setSavingNote] = useState(false);
  const [moderation, setModeration] = useState<ModerationAction | null>(null);

  if (loading) {
    return (
      <Panel>
        <PanelHeader title="Player profile" />
        <PanelBody className="space-y-3">
          <div className="flex items-center gap-3">
            <Skeleton className="h-14 w-14" />
            <div className="space-y-2">
              <Skeleton className="h-3 w-40" />
              <Skeleton className="h-3 w-24" />
            </div>
          </div>
          <Skeleton className="h-3 w-full" />
          <Skeleton className="h-3 w-2/3" />
        </PanelBody>
      </Panel>
    );
  }

  if (error) {
    return (
      <Panel>
        <PanelHeader title="Player profile" />
        <PanelBody>
          <InlineError message={error} />
        </PanelBody>
      </Panel>
    );
  }

  if (!profile) {
    return (
      <Panel>
        <PanelHeader title="Player profile" />
        <EmptyState
          title="No player selected"
          description="Search for a Roblox username, user ID or callsign to open a profile."
        />
      </Panel>
    );
  }

  const live = profile.live;
  const identifier = profile.robloxUserId ?? profile.username;

  async function addNote() {
    if (!token || !profile) return;
    setSavingNote(true);
    try {
      await api.addStaffNote(token, profile.robloxUserId, profile.username, note);
      setNote("");
      toast.success("Staff note saved");
      onReload();
    } catch (err) {
      toast.error("Could not save note", errorMessage(err));
    } finally {
      setSavingNote(false);
    }
  }

  return (
    <Panel>
      <PanelHeader
        title="Player profile"
        actions={
          <IconButton label="Reload profile" onClick={onReload}>
            <RefreshCw size={14} />
          </IconButton>
        }
      />
      <PanelBody className="space-y-4">
        <div className="flex items-start gap-3">
          <Avatar userId={profile.robloxUserId} name={profile.username} size={56} />
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="truncate text-[15px] font-semibold text-ink">{profile.username}</h3>
              {profile.roblox?.hasVerifiedBadge ? <Badge tone="positive">Verified</Badge> : null}
              {live ? <Badge tone="positive" dot>In server</Badge> : <Badge tone="muted">Offline</Badge>}
              {profile.erlcBan ? <Badge tone="danger">ER:LC ban</Badge> : null}
              {profile.internalBan ? <Badge tone="danger">Internal ban</Badge> : null}
            </div>
            <p className="mt-1 text-xs text-muted">
              {profile.roblox?.displayName ? `${profile.roblox.displayName} · ` : ""}
              ID {identifier}
            </p>
            <button
              type="button"
              onClick={() => {
                navigator.clipboard
                  .writeText(String(identifier))
                  .then(() => toast.info("Roblox ID copied"))
                  .catch(() => toast.error("Clipboard unavailable"));
              }}
              className="mt-1.5 inline-flex items-center gap-1 text-[11px] text-faint transition-colors hover:text-ink"
            >
              <Copy size={11} /> Copy identifier
            </button>
          </div>
          <Avatar userId={profile.robloxUserId} name={profile.username} size={56} fullBody />
        </div>

        <div className="grid gap-x-6 gap-y-1 border-t border-line pt-3 sm:grid-cols-2">
          <KeyValue label="Team" value={live?.team ?? "Not in server"} />
          <KeyValue label="Callsign" value={live?.callsign ?? "—"} mono />
          <KeyValue label="Server permission" value={live?.permission ?? "—"} />
          <KeyValue
            label="Location"
            value={
              live?.location
                ? [live.location.building, live.location.street, live.location.postal]
                    .filter(Boolean)
                    .join(" ") || `${live.location.x.toFixed(0)}, ${live.location.z.toFixed(0)}`
                : "Unavailable"
            }
          />
          <KeyValue label="First seen" value={formatDateTime(profile.firstSeenAt)} />
          <KeyValue label="Last seen" value={formatRelative(profile.lastSeenAt)} />
        </div>

        {has("moderation.view") && profile.erlcBanDetail ? (
          <p className="rounded border border-line bg-surface-2 px-3 py-2 text-[11px] leading-5 text-muted">
            {profile.erlcBanDetail}
          </p>
        ) : null}

        {has("moderation.kick") || has("moderation.ban") || has("moderation.unban") ? (
          <div className="flex flex-wrap gap-2 border-t border-line pt-3">
            {has("moderation.kick") ? (
              <Button size="sm" variant="secondary" onClick={() => setModeration("kick")}>
                <UserX size={13} /> Kick
              </Button>
            ) : null}
            {has("moderation.ban") ? (
              <Button size="sm" variant="secondary" onClick={() => setModeration("ban")}>
                <ShieldAlert size={13} /> Ban
              </Button>
            ) : null}
            {has("moderation.unban") ? (
              <Button size="sm" variant="secondary" onClick={() => setModeration("unban")}>
                <Ban size={13} /> Unban
              </Button>
            ) : null}
          </div>
        ) : null}

        <div className="border-t border-line pt-3">
          <SectionTitle>Moderation history</SectionTitle>
          <div className="panel overflow-hidden">
            <DataTable<ModerationRecordView>
              columns={[
                {
                  key: "action",
                  header: "Action",
                  render: (row) => (
                    <Badge tone={actionTone(row.action)}>{actionLabel(row.action)}</Badge>
                  ),
                },
                { key: "reason", header: "Reason", render: (row) => truncate(row.reason || "—", 48) },
                {
                  key: "moderator",
                  header: "By",
                  render: (row) => <span className="text-muted">{row.moderatorName}</span>,
                },
                {
                  key: "source",
                  header: "Source",
                  render: (row) => (
                    <span className="text-[11px] text-muted">{sourceLabel(row.source)}</span>
                  ),
                },
                {
                  key: "when",
                  header: "When",
                  align: "right",
                  render: (row) => (
                    <span className="text-[11px] text-muted">{formatDateTime(row.createdAt)}</span>
                  ),
                },
              ]}
              rows={profile.moderation}
              getKey={(row) => row.id}
              emptyTitle="No moderation records"
              emptyDescription="This player has no kicks, bans or notes recorded in CAD."
            />
          </div>
        </div>

        {has("players.notes.view") ? (
          <div className="border-t border-line pt-3">
            <SectionTitle>Staff notes</SectionTitle>
            <div className="space-y-2">
              {profile.notes.length === 0 ? (
                <p className="text-xs text-faint">No staff notes for this player.</p>
              ) : (
                profile.notes.map((entry) => (
                  <div key={entry.id} className="rounded border border-line bg-surface-2 px-3 py-2">
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-[11px] font-medium text-ink">{entry.authorName}</p>
                      <p className="text-[10px] text-faint">{formatDateTime(entry.createdAt)}</p>
                    </div>
                    <p className="mt-1 whitespace-pre-wrap break-words text-xs leading-5 text-muted">
                      {entry.body}
                    </p>
                  </div>
                ))
              )}
            </div>
            {has("players.notes.manage") ? (
              <div className="mt-3">
                <FieldLabel htmlFor="staff-note">Add a note</FieldLabel>
                <Textarea
                  id="staff-note"
                  rows={2}
                  value={note}
                  onChange={(event) => setNote(event.target.value)}
                  placeholder="Internal note visible to authorized staff only"
                />
                <div className="mt-2 flex justify-end">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={addNote}
                    loading={savingNote}
                    disabled={note.trim().length < 3}
                    leadingIcon={<Check size={13} />}
                  >
                    Save note
                  </Button>
                </div>
              </div>
            ) : null}
          </div>
        ) : null}
      </PanelBody>
      <ModerationActionDialog
        open={moderation !== null}
        action={moderation ?? "kick"}
        target={profile.username}
        robloxUserId={profile.robloxUserId}
        onClose={() => setModeration(null)}
        onDone={() => {
          onReload();
          onModerated?.();
        }}
      />
    </Panel>
  );
}
