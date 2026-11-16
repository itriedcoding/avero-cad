import { useEffect, useState } from "react";
import { Search, UserSearch } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatRelative } from "@/lib/format";
import { useDebounced } from "@/hooks/useAsync";
import { useSession } from "@/state/session";
import { useToast } from "@/state/toast";
import { Avatar } from "@/components/Avatar";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Field";
import { PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import { PlayerProfilePanel } from "@/components/PlayerProfilePanel";
import type { PlayerProfile, SearchResult } from "@/types";

export default function PlayersPage() {
  const { token } = useSession();
  const toast = useToast();
  const [query, setQuery] = useState("");
  const debounced = useDebounced(query, 280);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [searching, setSearching] = useState(false);
  const [selection, setSelection] = useState<{ id: number | null; username: string } | null>(null);
  const [profile, setProfile] = useState<PlayerProfile | null>(null);
  const [profileLoading, setProfileLoading] = useState(false);
  const [profileError, setProfileError] = useState<string | null>(null);
  const [profileNonce, setProfileNonce] = useState(0);

  useEffect(() => {
    if (!token) return;
    const needle = debounced.trim();
    if (needle.length < 2) {
      setResults([]);
      return;
    }
    let cancelled = false;
    setSearching(true);
    api
      .searchPlayers(token, needle)
      .then((rows) => {
        if (!cancelled) setResults(rows);
      })
      .catch((err) => {
        if (!cancelled) {
          setResults([]);
          toast.error("Player search failed", errorMessage(err));
        }
      })
      .finally(() => {
        if (!cancelled) setSearching(false);
      });
    return () => {
      cancelled = true;
    };
  }, [debounced, token, toast]);

  useEffect(() => {
    if (!token || !selection) {
      setProfile(null);
      return;
    }
    let cancelled = false;
    setProfileLoading(true);
    setProfileError(null);
    api
      .playerProfile(token, selection.id, selection.id === null ? selection.username : null)
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
  }, [token, selection, profileNonce]);

  async function lookupExact() {
    if (!token) return;
    const identifier = query.trim();
    if (!identifier) return;
    try {
      const user = await api.lookupRoblox(token, identifier);
      setSelection({ id: user.id, username: user.name });
      toast.info(`Loaded ${user.name} from Roblox`);
    } catch (err) {
      toast.error("Roblox lookup failed", errorMessage(err));
    }
  }

  return (
    <div>
      <PageHeader
        title="Players"
        description="Look up players by Roblox username, user ID or callsign. Search reads the live ER:LC roster and your local player records, then resolves the real Roblox identity and avatar."
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.05fr)_minmax(0,1fr)]">
        <Panel className="min-h-[520px]">
          <PanelHeader
            title="Search"
            description="At least 2 characters. Matching players in the server are listed first."
            actions={
              <Button
                size="sm"
                variant="secondary"
                onClick={lookupExact}
                disabled={query.trim().length === 0}
                leadingIcon={<UserSearch size={13} />}
              >
                Roblox lookup
              </Button>
            }
          />
          <div className="border-b border-line p-3">
            <div className="relative">
              <Search size={14} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint" />
              <Input
                value={query}
                onChange={(event) => setQuery(event.target.value)}
                placeholder="Username, Roblox ID or callsign"
                className="pl-8"
                autoFocus
                spellCheck={false}
              />
            </div>
          </div>
          <div className="p-0">
            {searching ? (
              <TableSkeleton rows={5} columns={3} />
            ) : (
              <DataTable<SearchResult>
                dense
                columns={[
                  {
                    key: "player",
                    header: "Player",
                    render: (row) => (
                      <span className="flex items-center gap-2">
                        <Avatar userId={row.robloxUserId} name={row.username} size={24} />
                        <span className="min-w-0">
                          <span className="block truncate text-[12px] text-ink">{row.username}</span>
                          <span className="block truncate text-[10px] text-faint">
                            {row.robloxUserId ?? "ID unknown"}
                          </span>
                        </span>
                      </span>
                    ),
                  },
                  {
                    key: "status",
                    header: "Status",
                    render: (row) =>
                      row.online ? <Badge tone="positive" dot>In server</Badge> : <Badge tone="muted">Offline</Badge>,
                  },
                  {
                    key: "team",
                    header: "Team",
                    render: (row) => (
                      <span className="text-[11px] text-muted">
                        {row.team ?? "—"}
                        {row.callsign ? ` · ${row.callsign}` : ""}
                      </span>
                    ),
                  },
                ]}
                rows={results}
                getKey={(row) => `${row.username}-${row.robloxUserId ?? "x"}`}
                onRowClick={(row) => setSelection({ id: row.robloxUserId, username: row.username })}
                selectedKey={
                  selection ? `${selection.username}-${selection.id ?? "x"}` : null
                }
                emptyTitle={query.trim().length < 2 ? "Start typing to search" : "No matching players"}
                emptyDescription={
                  query.trim().length < 2
                    ? "Search the live roster and stored player records."
                    : "No live or stored records match. Try the Roblox lookup to resolve an exact username or ID."
                }
                maxHeight="480px"
              />
            )}
          </div>
          {results.length > 0 ? (
            <div className="border-t border-line px-3 py-2 text-[10px] text-faint">
              {results.filter((row) => row.online).length} in server · last refresh{" "}
              {formatRelative(Date.now())}
            </div>
          ) : null}
        </Panel>

        <PlayerProfilePanel
          profile={profile}
          loading={profileLoading}
          error={profileError}
          onReload={() => setProfileNonce((value) => value + 1)}
        />
      </div>
    </div>
  );
}
