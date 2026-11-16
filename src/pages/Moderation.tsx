import { useState } from "react";
import { Ban, ShieldAlert, ShieldOff, UserX } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, truncate } from "@/lib/format";
import { actionLabel, actionTone, sourceLabel } from "@/lib/status";
import { useSession } from "@/state/session";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input, Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelBody, PanelHeader } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import { ModerationActionDialog, type ModerationAction } from "@/components/ModerationActionDialog";
import type { ModerationRecordView } from "@/types";

export default function ModerationPage() {
  const { token, has } = useSession();
  const toast = useToast();
  const [target, setTarget] = useState("");
  const [action, setAction] = useState<ModerationAction | null>(null);
  const [internalOpen, setInternalOpen] = useState(false);
  const [internalUsername, setInternalUsername] = useState("");
  const [internalReason, setInternalReason] = useState("");
  const [internalSaving, setInternalSaving] = useState(false);

  const records = useAsync<ModerationRecordView[]>(
    () => (token ? api.listModeration(token, 150) : Promise.resolve([])),
    [token],
    Boolean(token),
  );

  async function createInternalBan() {
    if (!token) return;
    setInternalSaving(true);
    try {
      await api.createInternalBan(token, null, internalUsername.trim(), internalReason.trim());
      toast.success("Internal ban recorded", "This is a CAD-only record and does not ban the player in ER:LC.");
      setInternalOpen(false);
      setInternalUsername("");
      setInternalReason("");
      await records.reload();
    } catch (err) {
      toast.error("Could not record internal ban", errorMessage(err));
    } finally {
      setInternalSaving(false);
    }
  }

  const canAct =
    has("moderation.kick") || has("moderation.ban") || has("moderation.unban") || has("moderation.internal");

  return (
    <div>
      <PageHeader
        title="Moderation"
        description="Kicks, bans and unbans run as real ER:LC server commands through the API. Every action is confirmed first, recorded locally and attributed to your staff account."
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,1.35fr)]">
        <div className="space-y-4">
          <Panel>
            <PanelHeader title="Run a moderation action" description="Enter a Roblox username or numeric user ID." />
            <PanelBody className="space-y-3">
              <Input
                label="Target player"
                value={target}
                placeholder="e.g. Builderman or 156"
                onChange={(event) => setTarget(event.target.value)}
                spellCheck={false}
              />
              <div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
                <Button
                  variant="secondary"
                  disabled={!has("moderation.kick") || target.trim().length === 0}
                  onClick={() => setAction("kick")}
                >
                  <UserX size={13} /> Kick
                </Button>
                <Button
                  variant="secondary"
                  disabled={!has("moderation.ban") || target.trim().length === 0}
                  onClick={() => setAction("ban")}
                >
                  <ShieldAlert size={13} /> Ban
                </Button>
                <Button
                  variant="secondary"
                  disabled={!has("moderation.unban") || target.trim().length === 0}
                  onClick={() => setAction("unban")}
                >
                  <Ban size={13} /> Unban
                </Button>
              </div>
              {!canAct ? (
                <p className="text-[11px] text-faint">
                  Your role has no moderation actions. Ask the Server Owner for the matching
                  permission.
                </p>
              ) : null}
              {target.trim().length === 0 ? (
                <p className="text-[11px] text-faint">
                  A target is required before an action can be selected.
                </p>
              ) : null}
            </PanelBody>
          </Panel>

          {has("moderation.internal") ? (
            <Panel>
              <PanelHeader
                title="Internal CAD ban"
                description="A local record only. The ER:LC API has no ban-write endpoint, so this never bans anyone in game."
              />
              <PanelBody className="space-y-3">
                <Input
                  label="Roblox username"
                  value={internalUsername}
                  placeholder="Username"
                  onChange={(event) => setInternalUsername(event.target.value)}
                  spellCheck={false}
                />
                <Textarea
                  label="Reason"
                  rows={2}
                  value={internalReason}
                  placeholder="Why is this internal ban being recorded?"
                  onChange={(event) => setInternalReason(event.target.value)}
                />
                <div className="flex justify-end">
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => setInternalOpen(true)}
                    disabled={internalUsername.trim().length < 1 || internalReason.trim().length < 3}
                    leadingIcon={<ShieldOff size={13} />}
                  >
                    Record internal ban
                  </Button>
                </div>
              </PanelBody>
            </Panel>
          ) : null}

          <Panel>
            <PanelHeader title="Permission model" description="How moderation permissions are enforced." />
            <PanelBody className="space-y-2 text-[11px] leading-5 text-muted">
              <p>
                The Rust backend checks <span className="mono text-ink">moderation.kick</span>,{" "}
                <span className="mono text-ink">moderation.ban</span> and{" "}
                <span className="mono text-ink">moderation.unban</span> before any HTTP request. Each
                command must also be authorized in Command Policies and requires explicit
                confirmation.
              </p>
              <p>
                ER:LC still applies its own authorization: the server key acts as virtual server
                management, so the API decides whether the action is permitted at all.
              </p>
            </PanelBody>
          </Panel>
        </div>

        <Panel>
          <PanelHeader
            title="Moderation history"
            description="Stored in the local CAD database."
            actions={
              <Button size="sm" variant="secondary" onClick={() => void records.reload()} loading={records.loading}>
                Reload
              </Button>
            }
          />
          <PanelBody padded={false}>
            {records.error ? (
              <div className="p-3">
                <InlineError message={records.error} />
              </div>
            ) : records.loading && !records.data ? (
              <TableSkeleton rows={8} columns={4} />
            ) : (
              <DataTable<ModerationRecordView>
                columns={[
                  {
                    key: "action",
                    header: "Action",
                    render: (row) => <Badge tone={actionTone(row.action)}>{actionLabel(row.action)}</Badge>,
                  },
                  {
                    key: "player",
                    header: "Player",
                    render: (row) => (
                      <span className="min-w-0">
                        <span className="block truncate text-[12px] text-ink">{row.username}</span>
                        <span className="block text-[10px] text-faint">{row.robloxUserId ?? "—"}</span>
                      </span>
                    ),
                  },
                  {
                    key: "reason",
                    header: "Reason",
                    render: (row) => (
                      <span className="text-[11px] text-muted">{truncate(row.reason || "—", 40)}</span>
                    ),
                  },
                  {
                    key: "moderator",
                    header: "Staff",
                    render: (row) => <span className="text-[11px] text-muted">{row.moderatorName}</span>,
                  },
                  {
                    key: "result",
                    header: "Result",
                    render: (row) => (
                      <span className="text-[10px]">
                        <Badge tone={row.success ? "positive" : "danger"}>
                          {row.success ? "Success" : "Failed"}
                        </Badge>
                        <span className="ml-1 text-faint">{sourceLabel(row.source)}</span>
                      </span>
                    ),
                  },
                  {
                    key: "when",
                    header: "When",
                    align: "right",
                    render: (row) => (
                      <span className="text-[10px] text-faint">{formatDateTime(row.createdAt)}</span>
                    ),
                  },
                ]}
                rows={records.data ?? []}
                getKey={(row) => row.id}
                emptyTitle="No moderation records"
                emptyDescription="Kicks, bans, unbans and internal bans appear here with their outcome."
                maxHeight="620px"
              />
            )}
          </PanelBody>
        </Panel>
      </div>

      <ModerationActionDialog
        open={action !== null}
        action={action ?? "kick"}
        target={target.trim()}
        robloxUserId={null}
        onClose={() => setAction(null)}
        onDone={() => void records.reload()}
      />

      <Modal
        open={internalOpen}
        onClose={() => setInternalOpen(false)}
        title="Confirm internal CAD ban"
        description="This creates a local ban record only. It does not touch your ER:LC server."
        footer={
          <>
            <Button variant="ghost" onClick={() => setInternalOpen(false)} disabled={internalSaving}>
              Cancel
            </Button>
            <Button variant="danger" onClick={createInternalBan} loading={internalSaving}>
              Record internal ban
            </Button>
          </>
        }
      >
        <div className="space-y-2 text-[13px]">
          <p className="text-muted">
            Player: <span className="text-ink">{internalUsername.trim() || "—"}</span>
          </p>
          <p className="rounded border border-line bg-surface-2 px-3 py-2 text-[11px] leading-5 text-muted">
            To actually ban this player in ER:LC, use the Ban action above, which runs the{" "}
            <span className="mono text-ink">:ban</span> server command.
          </p>
        </div>
      </Modal>
    </div>
  );
}
