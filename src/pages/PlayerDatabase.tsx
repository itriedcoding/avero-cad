import { useEffect, useMemo, useState } from "react";
import { RefreshCw, Search } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, formatRelative } from "@/lib/format";
import { useSession } from "@/state/session";
import { Avatar } from "@/components/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Field";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import { PlayerProfilePanel } from "@/components/PlayerProfilePanel";
import type { PlayerProfile, SearchResult } from "@/types";

export default function PlayerDatabasePage() {
  const { token } = useSession();
  const [rows, setRows] = useState<SearchResult[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");
  const [selection, setSelection] = useState<{ id: number | null; username: string } | null>(null);
  const [profile, setProfile] = useState<PlayerProfile | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [nonce, setNonce] = useState(0);

  async function load() {
    if (!token) return;
    setLoading(true);
    try {
      const next = await api.listKnownPlayers(token, 300);
      setRows(next);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token]);

  useEffect(() => {
    if (!token || !selection) {
      setProfile(null);
      return;
    }
    let cancelled = false;
    setProfileLoading(true);
    setProfileError(null);
    api
      .playerProfile(token, selection.id, selection.username)
      .then((next) => {
        if (!cancelled) setProfile(next);
      })
      .catch((err) => {
        if (!cancelled) {
          setProfile(null);
          setProfileError(errorMessage(err));
        }
      })
      .finally(() => {
        if (!cancelled) setProfileLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [token, selection, nonce]);

  const filtered = useMemo(() => {
    const needle = filter.trim().toLowerCase();
    if (!needle) return rows;
    return rows.filter(
      (row) =>
        row.username.toLowerCase().includes(needle) ||
        String(row.robloxUserId ?? "").includes(needle) ||
        (row.team ?? "").toLowerCase().includes(needle) ||
        (row.callsign ?? "").toLowerCase().includes(needle),
    );
  }, [rows, filter]);

  return (
    <div>
      <PageHeader
        title="Player Database"
        description="Every player the CAD has seen is stored locally with first/last seen timestamps, last team and callsign. This database is independent of the ER:LC server."
        actions={
          <Button
            size="sm"
            variant="secondary"
            onClick={() => void load()}
            loading={loading}
            leadingIcon={<RefreshCw size={13} />}
          >
            Reload
          </Button>
        }
      />

      {error ? <div className="mb-3"><InlineError message={error} /></div> : null}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]">
        <Panel>
          <PanelHeader
            title="Stored players"
            description={`${rows.length} record(s) stored · ${rows.filter((row) => row.online).length} currently online`}
          />
          <div className="border-b border-line p-3">
            <div className="relative">
              <Search size={14} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint" />
              <Input
                value={filter}
                onChange={(event) => setFilter(event.target.value)}
                placeholder="Filter stored players"
                className="pl-8"
                spellCheck={false}
              />
            </div>
          </div>
          {loading && rows.length === 0 ? (
            <TableSkeleton rows={8} columns={4} />
          ) : (
            <DataTable<SearchResult>
              dense
              columns={[
                {
                  key: "player",
                  header: "Player",
                  render: (row) => (
                    <span className="flex items-center gap-2">
                      <Avatar userId={row.robloxUserId} name={row.username} size={22} />
                      <span className="min-w-0">
                        <span className="block truncate text-[12px] text-ink">{row.username}</span>
                        <span className="block text-[10px] text-faint">{row.robloxUserId}</span>
                      </span>
                    </span>
                  ),
                },
                {
                  key: "online",
                  header: "Presence",
                  render: (row) =>
                    row.online ? <Badge tone="positive" dot>Online</Badge> : <Badge tone="muted">Stored</Badge>,
                },
                {
                  key: "team",
                  header: "Last team",
                  render: (row) => <span className="text-[11px] text-muted">{row.team ?? "—"}</span>,
                },
                {
                  key: "callsign",
                  header: "Callsign",
                  render: (row) => (
                    <span className="mono text-[11px] text-muted">{row.callsign ?? "—"}</span>
                  ),
                },
              ]}
              rows={filtered}
              getKey={(row) => `${row.username}-${row.robloxUserId ?? "x"}`}
              onRowClick={(row) => setSelection({ id: row.robloxUserId, username: row.username })}
              selectedKey={selection ? `${selection.username}-${selection.id ?? "x"}` : null}
              emptyTitle={rows.length === 0 ? "No players stored yet" : "No records match"}
              emptyDescription={
                rows.length === 0
                  ? "Player records are created automatically whenever the live roster is synchronized."
                  : "Adjust the filter to find a stored player."
              }
              maxHeight="560px"
              footer={`Showing ${filtered.length} of ${rows.length} stored records`}
            />
          )}
        </Panel>

        <div className="space-y-3">
          <PlayerProfilePanel
            profile={profile}
            loading={profileLoading}
            error={profileError}
            onReload={() => setNonce((value) => value + 1)}
          />
          {profile ? (
            <Panel>
              <PanelHeader title="Record metadata" />
              <div className="grid grid-cols-2 gap-x-4 gap-y-1 px-4 py-3 text-[11px]">
                <span className="text-faint">First seen</span>
                <span className="text-right text-muted">{formatDateTime(profile.firstSeenAt)}</span>
                <span className="text-faint">Last seen</span>
                <span className="text-right text-muted">{formatRelative(profile.lastSeenAt)}</span>
                <span className="text-faint">Internal bans</span>
                <span className="text-right text-muted">{profile.internalBanCount}</span>
              </div>
            </Panel>
          ) : null}
        </div>
      </div>
    </div>
  );
}
