import { useMemo, useState } from "react";
import { RefreshCw, ScrollText, Search } from "lucide-react";
import { formatDateTime, formatRelative } from "@/lib/format";
import { useSnapshot } from "@/state/snapshot";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Field";
import { EmptyState, PageHeader, Skeleton } from "@/components/ui/Feedback";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Tabs } from "@/components/ui/Tabs";
import { DataTable } from "@/components/ui/Table";
import type { CommandLogView, JoinLogView, KillLogView, ModCallView, QueueEntryView } from "@/types";

type Tab = "joins" | "leaves" | "kills" | "commands" | "modcalls" | "queue";

export default function ServerLogsPage() {
  const { snapshot, refreshing, refresh, loading } = useSnapshot();
  const [tab, setTab] = useState<Tab>("joins");
  const [query, setQuery] = useState("");

  const needle = query.trim().toLowerCase();
  const joins = (snapshot?.joinLogs ?? []).filter((entry) => entry.joined);
  const leaves = (snapshot?.joinLogs ?? []).filter((entry) => !entry.joined);
  const kills = snapshot?.killLogs ?? [];
  const commands = snapshot?.commandLogs ?? [];
  const modcalls = snapshot?.modCalls ?? [];
  const queue = snapshot?.queue ?? [];

  const visibleJoins = useMemo(
    () =>
      joins.filter(
        (entry) =>
          !needle ||
          entry.username.toLowerCase().includes(needle) ||
          String(entry.robloxUserId ?? "").includes(needle),
      ),
    [joins, needle],
  );
  const visibleLeaves = useMemo(
    () =>
      leaves.filter(
        (entry) =>
          !needle ||
          entry.username.toLowerCase().includes(needle) ||
          String(entry.robloxUserId ?? "").includes(needle),
      ),
    [leaves, needle],
  );
  const visibleKills = useMemo(
    () =>
      kills.filter(
        (entry) =>
          !needle ||
          entry.killer.toLowerCase().includes(needle) ||
          entry.killed.toLowerCase().includes(needle),
      ),
    [kills, needle],
  );
  const visibleCommands = useMemo(
    () =>
      commands.filter(
        (entry) =>
          !needle ||
          entry.player.toLowerCase().includes(needle) ||
          entry.command.toLowerCase().includes(needle),
      ),
    [commands, needle],
  );
  const visibleModcalls = useMemo(
    () =>
      modcalls.filter(
        (entry) =>
          !needle ||
          entry.caller.toLowerCase().includes(needle) ||
          (entry.moderator ?? "").toLowerCase().includes(needle),
      ),
    [modcalls, needle],
  );

  const tabs: { value: Tab; label: string; count: number }[] = [
    { value: "joins", label: "Join logs", count: joins.length },
    { value: "leaves", label: "Leave logs", count: leaves.length },
    { value: "kills", label: "Kill logs", count: kills.length },
    { value: "commands", label: "Command logs", count: commands.length },
    { value: "modcalls", label: "Moderator calls", count: modcalls.length },
    { value: "queue", label: "Player queue", count: queue.length },
  ];

  const empty = (title: string, description: string) => (
    <EmptyState compact title={title} description={description} icon={<ScrollText size={15} />} />
  );

  return (
    <div>
      <PageHeader
        title="Server Logs"
        description="Join, leave, kill, command and moderator-call history reported by ER:LC API v2, plus the current join queue. Requires the matching query flags on the API key."
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

      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <Tabs<Tab> value={tab} onChange={setTab} items={tabs} />
        <div className="relative w-full sm:w-64">
          <Search size={14} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint" />
          <Input
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="Filter entries"
            className="pl-8"
            spellCheck={false}
          />
        </div>
      </div>

      <Panel>
        <PanelHeader
          title={tabs.find((entry) => entry.value === tab)?.label ?? "Logs"}
          description={
            snapshot?.fetchedAt
              ? `Last API poll ${formatRelative(snapshot.fetchedAt)} · timestamps reported by ER:LC`
              : "Live data unavailable"
          }
        />
        {loading && !snapshot ? (
          <div className="space-y-2 p-3">
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
            <Skeleton className="h-8 w-full" />
          </div>
        ) : tab === "joins" ? (
          <DataTable<JoinLogView>
            dense
            columns={[
              { key: "player", header: "Player", render: (row) => <span className="text-[12px] text-ink">{row.username}</span> },
              {
                key: "id",
                header: "Roblox ID",
                render: (row) => <span className="mono text-[11px] text-muted">{row.robloxUserId ?? "—"}</span>,
              },
              {
                key: "when",
                header: "Joined",
                align: "right",
                render: (row) => <span className="text-[10px] text-faint">{formatDateTime(row.timestamp)}</span>,
              },
            ]}
            rows={visibleJoins}
            getKey={(row) => `${row.robloxUserId ?? row.username}-${row.timestamp}`}
            emptyTitle="No join records"
            emptyDescription="ER:LC reports joins for the current session when the JoinLogs flag is enabled."
            maxHeight="620px"
            footer={`${visibleJoins.length} join record(s)`}
          />
        ) : tab === "leaves" ? (
          <DataTable<JoinLogView>
            dense
            columns={[
              { key: "player", header: "Player", render: (row) => <span className="text-[12px] text-ink">{row.username}</span> },
              {
                key: "id",
                header: "Roblox ID",
                render: (row) => <span className="mono text-[11px] text-muted">{row.robloxUserId ?? "—"}</span>,
              },
              {
                key: "when",
                header: "Left",
                align: "right",
                render: (row) => <span className="text-[10px] text-faint">{formatDateTime(row.timestamp)}</span>,
              },
            ]}
            rows={visibleLeaves}
            getKey={(row) => `${row.robloxUserId ?? row.username}-${row.timestamp}`}
            emptyTitle="No leave records"
            emptyDescription="Leave events are derived from the same join-log feed ER:LC exposes."
            maxHeight="620px"
            footer={`${visibleLeaves.length} leave record(s)`}
          />
        ) : tab === "kills" ? (
          <DataTable<KillLogView>
            dense
            columns={[
              { key: "killer", header: "Killer", render: (row) => <span className="text-[12px] text-ink">{row.killer}</span> },
              { key: "killed", header: "Killed", render: (row) => <span className="text-[12px] text-muted">{row.killed}</span> },
              {
                key: "when",
                header: "When",
                align: "right",
                render: (row) => <span className="text-[10px] text-faint">{formatDateTime(row.timestamp)}</span>,
              },
            ]}
            rows={visibleKills}
            getKey={(row) => `${row.killer}-${row.killed}-${row.timestamp}`}
            emptyTitle="No kill records"
            emptyDescription="Kill logs require the KillLogs query flag on the ER:LC API key."
            maxHeight="620px"
            footer={`${visibleKills.length} kill record(s)`}
          />
        ) : tab === "commands" ? (
          <DataTable<CommandLogView>
            dense
            columns={[
              { key: "player", header: "Player", render: (row) => <span className="text-[12px] text-ink">{row.player}</span> },
              {
                key: "command",
                header: "Command executed in game",
                render: (row) => <span className="mono text-[11px] text-muted">{row.command}</span>,
              },
              {
                key: "when",
                header: "When",
                align: "right",
                render: (row) => <span className="text-[10px] text-faint">{formatDateTime(row.timestamp)}</span>,
              },
            ]}
            rows={visibleCommands}
            getKey={(row) => `${row.player}-${row.timestamp}-${row.command}`}
            emptyTitle="No command log entries"
            emptyDescription="CommandLogs reports commands run inside the game, not commands sent from CAD."
            maxHeight="620px"
            footer={`${visibleCommands.length} entry(ies)`}
          />
        ) : tab === "modcalls" ? (
          <DataTable<ModCallView>
            dense
            columns={[
              { key: "caller", header: "Caller", render: (row) => <span className="text-[12px] text-ink">{row.caller}</span> },
              {
                key: "moderator",
                header: "Handled by",
                render: (row) =>
                  row.moderator ? (
                    <span className="text-[12px] text-muted">{row.moderator}</span>
                  ) : (
                    <span className="text-[11px] text-faint">Unclaimed</span>
                  ),
              },
              {
                key: "when",
                header: "When",
                align: "right",
                render: (row) => <span className="text-[10px] text-faint">{formatDateTime(row.timestamp)}</span>,
              },
            ]}
            rows={visibleModcalls}
            getKey={(row) => `${row.caller}-${row.timestamp}`}
            emptyTitle="No moderator calls"
            emptyDescription="ModCalls lists in-game help requests; the API reports the caller and answering moderator."
            maxHeight="620px"
            footer={`${visibleModcalls.length} call(s)`}
          />
        ) : queue.length === 0 ? (
          empty("Queue is empty", "No players are waiting to join the server right now.")
        ) : (
          <DataTable<QueueEntryView>
            dense
            columns={[
              { key: "position", header: "Position", render: (row) => <span className="mono text-[12px] text-ink">#{row.position}</span> },
              {
                key: "id",
                header: "Roblox user ID",
                render: (row) => <span className="mono text-[11px] text-muted">{row.robloxUserId}</span>,
              },
            ]}
            rows={queue}
            getKey={(row) => `${row.robloxUserId}-${row.position}`}
            emptyTitle="Queue is empty"
            emptyDescription="Players waiting for a slot appear here."
            maxHeight="620px"
            footer={`${queue.length} player(s) queued`}
          />
        )}
      </Panel>

      <p className="mt-3 text-[11px] leading-5 text-faint">
        ER:LC only returns logs when the API key has the corresponding query flags and the server
        reports them. Empty tables mean the API sent no records — nothing is generated locally.
      </p>
    </div>
  );
}
