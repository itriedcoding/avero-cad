import { useState } from "react";
import { Ban, RefreshCw, ShieldOff, Unlock } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime } from "@/lib/format";
import { useSession } from "@/state/session";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Avatar } from "@/components/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Textarea } from "@/components/ui/Field";
import { Modal } from "@/components/ui/Modal";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Tabs } from "@/components/ui/Tabs";
import { DataTable } from "@/components/ui/Table";
import { ModerationActionDialog } from "@/components/ModerationActionDialog";
import type { InternalBanView, LiveBanView } from "@/types";

type Tab = "live" | "internal";

export default function BannedPlayersPage() {
  const { token, has } = useSession();
  const toast = useToast();
  const [tab, setTab] = useState<Tab>("live");
  const [unbanTarget, setUnbanTarget] = useState<string | null>(null);
  const [liftTarget, setLiftTarget] = useState<InternalBanView | null>(null);
  const [liftReason, setLiftReason] = useState("");
  const [lifting, setLifting] = useState(false);

  const live = useAsync<LiveBanView[]>(
    () => (token ? api.listLiveBans(token) : Promise.resolve([])),
    [token, tab],
    Boolean(token) && tab === "live",
  );
  const internal = useAsync<InternalBanView[]>(
    () => (token ? api.listInternalBans(token, false, 300) : Promise.resolve([])),
    [token, tab],
    Boolean(token) && tab === "internal",
  );

  async function liftBan() {
    if (!token || !liftTarget) return;
    setLifting(true);
    try {
      await api.liftInternalBan(token, liftTarget.id, liftReason.trim());
      toast.success("Internal ban lifted");
      setLiftTarget(null);
      setLiftReason("");
      await internal.reload();
    } catch (err) {
      toast.error("Could not lift ban", errorMessage(err));
    } finally {
      setLifting(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="Banned Players"
        description="Two distinct ban sources: the live ER:LC ban list (read-only from the API) and internal CAD records that never affect the Roblox server."
      />

      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <Tabs<Tab>
          value={tab}
          onChange={setTab}
          items={[
            { value: "live", label: "ER:LC server bans", count: live.data?.length },
            { value: "internal", label: "Internal CAD bans", count: internal.data?.length },
          ]}
        />
        <Button
          size="sm"
          variant="secondary"
          loading={tab === "live" ? live.loading : internal.loading}
          onClick={() => void (tab === "live" ? live.reload() : internal.reload())}
          leadingIcon={<RefreshCw size={13} />}
        >
          Refresh
        </Button>
      </div>

      {tab === "live" ? (
        <Panel>
          <PanelHeader
            title="ER:LC server bans"
            description="Fetched from GET /v1/server/bans. ER:LC has no ban-write endpoint; unbans run the documented :unban server command."
            actions={<Badge tone="neutral">Read-only API</Badge>}
          />
          {live.error ? (
            <div className="p-3">
              <InlineError
                message={live.error}
                action={
                  <Button size="sm" variant="secondary" onClick={() => void live.reload()}>
                    Retry
                  </Button>
                }
              />
            </div>
          ) : live.loading && !live.data ? (
            <TableSkeleton rows={8} columns={3} />
          ) : (
            <DataTable<LiveBanView>
              columns={[
                {
                  key: "player",
                  header: "Banned player",
                  render: (row) => (
                    <span className="flex items-center gap-2">
                      <Avatar userId={row.robloxUserId} name={row.username ?? "Unknown"} size={22} />
                      <span className="min-w-0">
                        <span className="block truncate text-[12px] text-ink">
                          {row.username ?? "Unknown player"}
                        </span>
                        <span className="block text-[10px] text-faint">{row.robloxUserId ?? "ID unavailable"}</span>
                      </span>
                    </span>
                  ),
                },
                {
                  key: "raw",
                  header: "API record",
                  render: (row) => (
                    <span className="mono text-[10px] text-faint">{row.raw.slice(0, 80)}</span>
                  ),
                },
                {
                  key: "action",
                  header: "",
                  align: "right",
                  render: (row) =>
                    has("moderation.unban") && row.username ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => setUnbanTarget(row.username as string)}
                        leadingIcon={<Unlock size={12} />}
                      >
                        Unban
                      </Button>
                    ) : (
                      <span className="text-[10px] text-faint">—</span>
                    ),
                },
              ]}
              rows={live.data ?? []}
              getKey={(row) => row.raw}
              emptyTitle="No bans reported"
              emptyDescription="The ER:LC API returned an empty ban list for this server."
              maxHeight="560px"
            />
          )}
        </Panel>
      ) : (
        <Panel>
          <PanelHeader
            title="Internal CAD bans"
            description="Local records created in this application. They are visible to staff and never change the ER:LC server."
            actions={<Badge tone="neutral">CAD only</Badge>}
          />
          {internal.error ? (
            <div className="p-3">
              <InlineError message={internal.error} />
            </div>
          ) : internal.loading && !internal.data ? (
            <TableSkeleton rows={6} columns={4} />
          ) : (
            <DataTable<InternalBanView>
              columns={[
                {
                  key: "player",
                  header: "Player",
                  render: (row) => (
                    <span className="min-w-0">
                      <span className="block truncate text-[12px] text-ink">{row.username}</span>
                      <span className="block text-[10px] text-faint">{row.robloxUserId ?? "ID unknown"}</span>
                    </span>
                  ),
                },
                {
                  key: "reason",
                  header: "Reason",
                  render: (row) => (
                    <span className="text-[11px] text-muted">{row.reason || "—"}</span>
                  ),
                },
                {
                  key: "staff",
                  header: "Recorded by",
                  render: (row) => <span className="text-[11px] text-muted">{row.createdByName}</span>,
                },
                {
                  key: "status",
                  header: "Status",
                  render: (row) =>
                    row.active ? (
                      <Badge tone="danger" dot>Active</Badge>
                    ) : (
                      <span className="text-[10px] text-faint">
                        Lifted {formatDateTime(row.liftedAt)}
                        {row.liftedByName ? ` by ${row.liftedByName}` : ""}
                      </span>
                    ),
                },
                {
                  key: "action",
                  header: "",
                  align: "right",
                  render: (row) =>
                    row.active && has("moderation.internal") ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => {
                          setLiftTarget(row);
                          setLiftReason("");
                        }}
                        leadingIcon={<ShieldOff size={12} />}
                      >
                        Lift
                      </Button>
                    ) : (
                      <span className="text-[10px] text-faint">—</span>
                    ),
                },
              ]}
              rows={internal.data ?? []}
              getKey={(row) => row.id}
              emptyTitle="No internal bans"
              emptyDescription="Internal bans created from the Moderation page appear here."
              maxHeight="560px"
            />
          )}
        </Panel>
      )}

      <div className="mt-3 rounded-md border border-line bg-surface px-3.5 py-3">
        <p className="flex items-start gap-2 text-[11px] leading-5 text-muted">
          <Ban size={13} className="mt-0.5 shrink-0 text-faint" />
          <span>
            An internal CAD ban is not a server ban. To enforce a ban in ER:LC, run the{" "}
            <span className="mono text-ink">:ban</span> command from Moderation or the command
            console; the API then reports it in the live ban list above.
          </span>
        </p>
      </div>

      <ModerationActionDialog
        open={unbanTarget !== null}
        action="unban"
        target={unbanTarget ?? ""}
        robloxUserId={null}
        onClose={() => setUnbanTarget(null)}
        onDone={() => void live.reload()}
      />

      <Modal
        open={liftTarget !== null}
        onClose={() => setLiftTarget(null)}
        title={`Lift internal ban — ${liftTarget?.username ?? ""}`}
        description="This removes the CAD-only record. It does not unban anyone on the ER:LC server."
        footer={
          <>
            <Button variant="ghost" onClick={() => setLiftTarget(null)} disabled={lifting}>
              Cancel
            </Button>
            <Button variant="primary" onClick={liftBan} loading={lifting} disabled={liftReason.trim().length < 3}>
              Lift ban
            </Button>
          </>
        }
      >
        <Textarea
          label="Reason for lifting"
          rows={3}
          value={liftReason}
          onChange={(event) => setLiftReason(event.target.value)}
          placeholder="e.g. Appeal accepted"
        />
      </Modal>
    </div>
  );
}
