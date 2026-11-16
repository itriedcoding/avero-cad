import { useEffect, useState } from "react";
import {
  Activity,
  Car,
  Check,
  Copy,
  CarFront,
  Radio,
  Server,
  ShieldCheck,
  Siren,
  Users,
  Wifi,
} from "lucide-react";
import { api } from "@/lib/api";
import { formatNumber, formatRelative, formatTime } from "@/lib/format";
import { navigate } from "@/lib/router";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useToast } from "@/state/toast";
import { Badge, StatusDot } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { KeyValue, Panel, PanelBody, PanelHeader, SectionTitle, StatTile } from "@/components/ui/Panel";
import { PageHeader, InlineError, RefreshButton, TableSkeleton } from "@/components/ui/Feedback";
import { Sparkline } from "@/components/ui/Sparkline";
import type { CallView } from "@/types";

export default function DashboardPage() {
  const { snapshot, loading, error, refresh, refreshing, lastSync } = useSnapshot();
  const { token, has, session } = useSession();
  const toast = useToast();
  const [calls, setCalls] = useState<CallView[]>([]);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!token || !has("dispatch.view")) {
      setCalls([]);
      return;
    }
    api
      .listCalls(token, null, 100)
      .then(setCalls)
      .catch(() => setCalls([]));
  }, [token, has, snapshot?.fetchedAt]);

  const server = snapshot?.server ?? null;
  const connection = snapshot?.connection;
  const counts = snapshot?.counts;
  const openCalls = calls.filter((call) => call.status !== "closed");
  const emergencyCalls = snapshot?.emergencyCalls ?? [];
  const joinLogs = (snapshot?.joinLogs ?? []).slice(0, 7);
  const modCalls = (snapshot?.modCalls ?? []).slice(0, 5);
  const history = (snapshot?.history ?? []).map((point) => point.playerCount);
  const capacity = server && server.maxPlayers > 0 ? server.currentPlayers / server.maxPlayers : 0;

  function copyJoinKey() {
    if (!server) return;
    navigator.clipboard
      .writeText(server.joinKey)
      .then(() => {
        setCopied(true);
        toast.success("Join code copied");
        window.setTimeout(() => setCopied(false), 1600);
      })
      .catch(() => toast.error("Clipboard unavailable in this environment"));
  }

  return (
    <div>
      <PageHeader
        title="Dashboard"
        description={
          server
            ? `Live overview of ${server.name}. Data comes directly from the ER:LC Private Server API.`
            : "Connect your ER:LC server key to load live server data."
        }
        actions={<RefreshButton onClick={() => void refresh(true)} refreshing={refreshing} lastSync={lastSync} />}
      />

      {connection?.state === "not_configured" ? (
        <Panel className="mb-4">
          <PanelBody className="flex flex-wrap items-center justify-between gap-4">
            <div className="flex items-start gap-3">
              <span className="mt-0.5 flex h-8 w-8 items-center justify-center rounded border border-line bg-surface-2 text-faint">
                <Server size={15} />
              </span>
              <div>
                <p className="text-[13px] font-medium text-ink">No ER:LC server key configured</p>
                <p className="mt-0.5 max-w-xl text-xs leading-5 text-muted">
                  Add your private server key to load the live roster, vehicles, logs and emergency
                  calls. The key is encrypted with Windows DPAPI and never shown again after saving.
                </p>
              </div>
            </div>
            {has("api.configure") ? (
              <Button variant="primary" onClick={() => navigate("api")}>
                Configure API access
              </Button>
            ) : (
              <p className="text-[11px] text-faint">Ask the Server Owner to configure the API key.</p>
            )}
          </PanelBody>
        </Panel>
      ) : null}

      {connection?.state === "error" ? (
        <div className="mb-4">
          <InlineError
            message={connection.detail ?? "The last synchronization failed."}
            action={
              <Button size="sm" variant="secondary" onClick={() => void refresh(true)}>
                Retry
              </Button>
            }
          />
        </div>
      ) : null}
      {error ? (
        <div className="mb-4">
          <InlineError message={error} />
        </div>
      ) : null}

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        <StatTile
          label="Players online"
          value={counts ? formatNumber(counts.total) : "—"}
          detail={server ? `of ${formatNumber(server.maxPlayers)} slots · ${Math.round(capacity * 100)}% full` : "No data"}
          icon={<Users size={15} />}
          accent
        />
        <StatTile
          label="Law enforcement"
          value={counts ? formatNumber(counts.lawEnforcement) : "—"}
          detail="Police, sheriff and patrol units on duty"
          icon={<ShieldCheck size={15} />}
        />
        <StatTile
          label="Fire / EMS"
          value={counts ? formatNumber(counts.fireEms) : "—"}
          detail="Fire, EMS and DOT personnel"
          icon={<Siren size={15} />}
        />
        <StatTile
          label="Staff in server"
          value={counts ? formatNumber(counts.staff) : "—"}
          detail="Admins, moderators and helpers"
          icon={<Activity size={15} />}
        />
        <StatTile
          label="Active CAD calls"
          value={has("dispatch.view") ? formatNumber(openCalls.length) : "—"}
          detail={
            has("dispatch.view")
              ? `${openCalls.filter((call) => call.source === "erlc").length} mirrored from ER:LC`
              : "Dispatch access required"
          }
          icon={<Radio size={15} />}
        />
        <StatTile
          label="Vehicles spawned"
          value={snapshot ? formatNumber(snapshot.vehicles.length) : "—"}
          detail="Spawned vehicles reported by the API"
          icon={<Car size={15} />}
        />
        <StatTile
          label="Emergency calls"
          value={snapshot ? formatNumber(emergencyCalls.length) : "—"}
          detail="Live 911 calls in ER:LC"
          icon={<Siren size={15} />}
        />
        <StatTile
          label="Wanted players"
          value={counts ? formatNumber(counts.wanted) : "—"}
          detail="Players with at least one wanted star"
          icon={<CarFront size={15} />}
        />
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-3">
        <Panel className="xl:col-span-2">
          <PanelHeader
            title="Server"
            description="Identity and configuration reported by the ER:LC API."
            actions={
              <Badge tone={connection?.state === "connected" ? "positive" : connection?.state === "error" ? "danger" : "muted"} dot>
                {connection?.state === "connected"
                  ? "Live"
                  : connection?.state === "error"
                    ? "Unavailable"
                    : "Not configured"}
              </Badge>
            }
          />
          <PanelBody>
            {loading && !server ? (
              <TableSkeleton rows={4} columns={2} />
            ) : server ? (
              <div className="grid gap-x-8 gap-y-1 sm:grid-cols-2">
                <KeyValue label="Server name" value={server.name} />
                <KeyValue label="Owner user ID" value={server.ownerId} mono />
                <KeyValue
                  label="Join code"
                  value={
                    <button
                      type="button"
                      onClick={copyJoinKey}
                      className="mono inline-flex items-center gap-1.5 rounded border border-line bg-surface-2 px-1.5 py-0.5 text-[11px] text-ink transition-colors hover:border-line-strong"
                      title="Copy join code"
                    >
                      {server.joinKey || "—"}
                      {copied ? <Check size={11} /> : <Copy size={11} />}
                    </button>
                  }
                />
                <KeyValue label="Capacity" value={`${server.currentPlayers} / ${server.maxPlayers}`} />
                <KeyValue label="Account verification" value={server.accVerifiedReq || "Disabled"} />
                <KeyValue label="Team balance" value={server.teamBalance ? "Enforced" : "Off"} />
                <KeyValue label="API endpoint" value={connection?.apiUrl ?? "—"} mono />
                <KeyValue
                  label="API latency"
                  value={connection?.latencyMs ? `${connection.latencyMs} ms` : "—"}
                />
                <KeyValue
                  label="Rate limit remaining"
                  value={
                    connection?.rateLimit?.remaining !== null && connection?.rateLimit?.remaining !== undefined
                      ? `${connection.rateLimit.remaining}${
                          connection.rateLimit.limit ? ` / ${connection.rateLimit.limit}` : ""
                        }`
                      : "—"
                  }
                />
                <KeyValue
                  label="Last synchronized"
                  value={lastSync ? formatRelative(lastSync) : "—"}
                />
              </div>
            ) : (
              <p className="text-xs text-faint">
                No server data yet. {session?.user.roleName === "Server Owner" ? "Configure your API key to begin." : "Ask the Server Owner to configure the API key."}
              </p>
            )}
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader title="Player count trend" description="Recorded from live API polls." />
          <PanelBody>
            <Sparkline values={history} height={64} label="Player count history" />
            <div className="mt-3 flex items-center justify-between border-t border-line pt-2.5 text-[11px] text-faint">
              <span>{history.length} samples</span>
              <span className="flex items-center gap-1.5">
                <Wifi size={11} /> auto-refresh
              </span>
            </div>
          </PanelBody>
        </Panel>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-3">
        <Panel>
          <PanelHeader
            title="Emergency calls"
            description="Live 911 calls reported by ER:LC."
            actions={<Button size="sm" variant="ghost" onClick={() => navigate("emergency")}>Open</Button>}
          />
          <PanelBody padded={false}>
            {emergencyCalls.length === 0 ? (
              <p className="px-4 py-6 text-center text-xs text-faint">No active emergency calls.</p>
            ) : (
              <ul className="divide-y divide-line/70">
                {emergencyCalls.slice(0, 6).map((call) => (
                  <li key={`${call.callNumber}-${call.startedAt}`} className="px-4 py-2.5">
                    <div className="flex items-center justify-between gap-3">
                      <p className="text-[12px] font-medium text-ink">
                        #{call.callNumber} · {call.team || "Unknown team"}
                      </p>
                      <span className="text-[10px] text-faint">{formatTime(call.startedAt)}</span>
                    </div>
                    <p className="mt-0.5 truncate text-[11px] text-muted">
                      {call.description || "No description"}
                    </p>
                    <p className="text-[10px] text-faint">
                      {call.positionDescriptor || "Location unavailable"}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader
            title="Recent joins"
            description="Join and leave activity from the API."
            actions={<Button size="sm" variant="ghost" onClick={() => navigate("logs")}>Logs</Button>}
          />
          <PanelBody padded={false}>
            {joinLogs.length === 0 ? (
              <p className="px-4 py-6 text-center text-xs text-faint">No join activity reported.</p>
            ) : (
              <ul className="divide-y divide-line/70">
                {joinLogs.map((entry, index) => (
                  <li
                    key={`${entry.username}-${entry.timestamp}-${index}`}
                    className="flex items-center justify-between gap-3 px-4 py-2"
                  >
                    <span className="flex min-w-0 items-center gap-2">
                      <StatusDot tone={entry.joined ? "online" : "neutral"} />
                      <span className="truncate text-[12px] text-ink">{entry.username}</span>
                    </span>
                    <span className="shrink-0 text-[10px] text-faint">
                      {entry.joined ? "joined" : "left"} · {formatTime(entry.timestamp)}
                    </span>
                  </li>
                ))}
              </ul>
            )}
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader
            title="Moderator calls"
            description="Calls for staff assistance in game."
            actions={<Button size="sm" variant="ghost" onClick={() => navigate("moderation")}>Moderation</Button>}
          />
          <PanelBody padded={false}>
            {modCalls.length === 0 ? (
              <p className="px-4 py-6 text-center text-xs text-faint">No moderator calls.</p>
            ) : (
              <ul className="divide-y divide-line/70">
                {modCalls.map((call, index) => (
                  <li key={`${call.caller}-${call.timestamp}-${index}`} className="px-4 py-2">
                    <div className="flex items-center justify-between gap-3">
                      <span className="truncate text-[12px] text-ink">{call.caller}</span>
                      <span className="shrink-0 text-[10px] text-faint">{formatTime(call.timestamp)}</span>
                    </div>
                    <p className="text-[10px] text-faint">
                      {call.moderator ? `Answered by ${call.moderator}` : "Unanswered"}
                    </p>
                  </li>
                ))}
              </ul>
            )}
          </PanelBody>
        </Panel>
      </div>

      <div className="mt-4 grid gap-4 xl:grid-cols-2">
        <Panel>
          <PanelHeader title="Live roster" description="Players currently in your ER:LC server." />
          <PanelBody padded={false}>
            {(snapshot?.players ?? []).length === 0 ? (
              <p className="px-4 py-6 text-center text-xs text-faint">
                {server ? "No players are in the server." : "Waiting for server data."}
              </p>
            ) : (
              <ul className="grid max-h-64 grid-cols-1 gap-px overflow-y-auto bg-line/40 sm:grid-cols-2">
                {(snapshot?.players ?? []).map((player) => (
                  <li
                    key={`${player.username}-${player.robloxUserId ?? "x"}`}
                    className="flex items-center justify-between gap-2 bg-surface px-3 py-2"
                  >
                    <div className="min-w-0">
                      <p className="truncate text-[12px] text-ink">{player.username}</p>
                      <p className="truncate text-[10px] text-faint">
                        {player.team}
                        {player.callsign ? ` · ${player.callsign}` : ""}
                      </p>
                    </div>
                    {player.staffRole ? <Badge tone="neutral">Staff</Badge> : null}
                  </li>
                ))}
              </ul>
            )}
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader
            title="Notifications"
            description="Live events detected from API polls during this session."
          />
          <PanelBody padded={false}>
            {(snapshot?.notifications ?? []).length === 0 ? (
              <p className="px-4 py-6 text-center text-xs text-faint">
                No new emergency or moderator calls detected yet.
              </p>
            ) : (
              <ul className="max-h-64 divide-y divide-line/70 overflow-y-auto">
                {(snapshot?.notifications ?? []).slice(0, 12).map((note) => (
                  <li key={note.id} className="flex items-start gap-2.5 px-4 py-2">
                    <StatusDot tone={note.kind === "emergency" ? "warn" : "online"} className="mt-1.5" />
                    <div className="min-w-0 flex-1">
                      <p className="truncate text-[12px] text-ink">{note.title}</p>
                      <p className="truncate text-[10px] text-faint">{note.detail}</p>
                    </div>
                    <span className="shrink-0 text-[10px] text-faint">{formatTime(note.at)}</span>
                  </li>
                ))}
              </ul>
            )}
          </PanelBody>
        </Panel>
      </div>

      <div className="mt-4">
        <SectionTitle>Shortcuts</SectionTitle>
        <div className="flex flex-wrap gap-2">
          {has("dispatch.view") ? (
            <Button variant="secondary" size="sm" onClick={() => navigate("dispatch")}>
              <Radio size={13} /> CAD dispatch
            </Button>
          ) : null}
          {has("map.view") ? (
            <Button variant="secondary" size="sm" onClick={() => navigate("map")}>
              Live map
            </Button>
          ) : null}
          {has("commands.view") ? (
            <Button variant="secondary" size="sm" onClick={() => navigate("commands")}>
              Command console
            </Button>
          ) : null}
          {has("moderation.view") ? (
            <Button variant="secondary" size="sm" onClick={() => navigate("moderation")}>
              Moderation
            </Button>
          ) : null}
          {has("players.view") ? (
            <Button variant="secondary" size="sm" onClick={() => navigate("players")}>
              Player lookup
            </Button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
