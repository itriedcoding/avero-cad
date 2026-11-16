import { useState } from "react";
import { MapPin, RefreshCw, Siren } from "lucide-react";
import { api } from "@/lib/api";
import { formatDateTime, formatRelative, formatTime } from "@/lib/format";
import { callStatusLabel, callStatusTone } from "@/lib/status";
import { navigate } from "@/lib/router";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useAsync } from "@/hooks/useAsync";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { EmptyState, InlineError, PageHeader } from "@/components/ui/Feedback";
import { Panel, PanelBody, PanelHeader } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import type { CallView } from "@/types";

export default function EmergencyCallsPage() {
  const { snapshot, refresh, refreshing } = useSnapshot();
  const { token, has } = useSession();
  const [tab, setTab] = useState<"live" | "cad">("live");

  const mirror = useAsync<CallView[]>(
    () => (token && has("dispatch.view") ? api.listCalls(token, null, 200) : Promise.resolve([])),
    [token, snapshot?.fetchedAt],
    Boolean(token) && has("dispatch.view"),
  );

  const liveCalls = snapshot?.emergencyCalls ?? [];
  const erlcCalls = (mirror.data ?? []).filter((call) => call.source === "erlc");

  return (
    <div>
      <PageHeader
        title="Emergency Calls"
        description="Live 911 calls reported by ER:LC, plus the incidents CAD has mirrored from that feed. CAD-authored calls are separate and never appear in game."
        actions={
          <Button
            size="sm"
            variant="secondary"
            onClick={() => void refresh(true)}
            loading={refreshing}
            leadingIcon={<RefreshCw size={13} />}
          >
            Refresh
          </Button>
        }
      />

      <div className="mb-3 flex items-center gap-1.5">
        <Button
          size="sm"
          variant={tab === "live" ? "primary" : "ghost"}
          onClick={() => setTab("live")}
        >
          Live ER:LC calls ({liveCalls.length})
        </Button>
        <Button
          size="sm"
          variant={tab === "cad" ? "primary" : "ghost"}
          onClick={() => setTab("cad")}
        >
          Mirrored in CAD ({erlcCalls.length})
        </Button>
      </div>

      {tab === "live" ? (
        <Panel>
          <PanelHeader
            title="Live emergency calls"
            description="Directly from the API v2 EmergencyCalls block. Positions come from the game."
          />
          <PanelBody>
            {liveCalls.length === 0 ? (
              <EmptyState
                icon={<Siren size={16} />}
                title="No active emergency calls"
                description="ER:LC reports no open calls right now. Calls appear here automatically as players create them."
              />
            ) : (
              <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
                {liveCalls.map((call) => (
                  <div key={`${call.callNumber}-${call.startedAt}`} className="rounded-md border border-line bg-surface-2 p-3">
                    <div className="flex items-center justify-between gap-2">
                      <span className="mono text-[12px] font-semibold text-ink">#{call.callNumber}</span>
                      <Badge tone="neutral">{call.team || "Unknown team"}</Badge>
                    </div>
                    <p className="mt-1.5 text-[12px] leading-5 text-ink">{call.description || "No description"}</p>
                    <p className="mt-1 flex items-start gap-1 text-[11px] leading-4 text-muted">
                      <MapPin size={11} className="mt-0.5 shrink-0" />
                      {call.positionDescriptor || "Location unavailable"}
                    </p>
                    <div className="mt-2 flex flex-wrap items-center justify-between gap-2 border-t border-line pt-2 text-[10px] text-faint">
                      <span>Caller ID {call.callerId}</span>
                      <span>{formatTime(call.startedAt)}</span>
                    </div>
                    {call.playerIds.length > 0 ? (
                      <p className="mt-1 text-[10px] text-faint">
                        Attached player IDs: {call.playerIds.join(", ")}
                      </p>
                    ) : null}
                    {call.x !== null && call.z !== null ? (
                      <button
                        type="button"
                        onClick={() => navigate("map")}
                        className="mono mt-1.5 text-[10px] text-muted underline decoration-line underline-offset-2 hover:text-ink"
                      >
                        {call.x.toFixed(1)}, {call.z.toFixed(1)} · view on map
                      </button>
                    ) : null}
                  </div>
                ))}
              </div>
            )}
          </PanelBody>
        </Panel>
      ) : (
        <Panel>
          <PanelHeader
            title="Calls mirrored into CAD dispatch"
            description="Each live ER:LC call is imported once by its call number so dispatchers can track it alongside CAD incidents."
          />
          {mirror.error ? (
            <div className="p-3">
              <InlineError message={mirror.error} />
            </div>
          ) : (
            <DataTable<CallView>
              dense
              columns={[
                {
                  key: "number",
                  header: "CAD #",
                  render: (row) => <span className="mono text-[12px] text-ink">#{row.callNumber}</span>,
                },
                {
                  key: "erlc",
                  header: "ER:LC #",
                  render: (row) => <span className="mono text-[11px] text-muted">{row.erlcCallNumber ?? "—"}</span>,
                },
                {
                  key: "title",
                  header: "Incident",
                  render: (row) => (
                    <span className="min-w-0">
                      <span className="block truncate text-[12px] text-ink">{row.title}</span>
                      <span className="block truncate text-[10px] text-faint">{row.location || "—"}</span>
                    </span>
                  ),
                },
                {
                  key: "status",
                  header: "Status",
                  render: (row) => <Badge tone={callStatusTone(row.status)}>{callStatusLabel(row.status)}</Badge>,
                },
                {
                  key: "units",
                  header: "Units",
                  render: (row) => <span className="text-[11px] text-muted">{row.units.length}</span>,
                },
                {
                  key: "created",
                  header: "Imported",
                  align: "right",
                  render: (row) => (
                    <span className="text-[10px] text-faint">
                      {formatRelative(row.createdAt)} · {formatDateTime(row.createdAt)}
                    </span>
                  ),
                },
              ]}
              rows={erlcCalls}
              getKey={(row) => row.id}
              emptyTitle="No mirrored calls yet"
              emptyDescription="When ER:LC reports an emergency call it is imported here automatically on the next refresh."
              maxHeight="600px"
            />
          )}
        </Panel>
      )}
    </div>
  );
}
