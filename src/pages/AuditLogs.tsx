import { useState } from "react";
import { History, RefreshCw, Search } from "lucide-react";
import { api } from "@/lib/api";
import { formatDateTime, humanize } from "@/lib/format";
import { useSession } from "@/state/session";
import { useAsync, useDebounced } from "@/hooks/useAsync";
import { Button } from "@/components/ui/Button";
import { Input, Select } from "@/components/ui/Field";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import type { AuditLogView } from "@/types";

export default function AuditLogsPage() {
  const { token } = useSession();
  const [category, setCategory] = useState<string>("");
  const [query, setQuery] = useState("");
  const debounced = useDebounced(query, 250);

  const categories = useAsync<string[]>(
    () => (token ? api.auditCategories(token) : Promise.resolve([])),
    [token],
    Boolean(token),
  );
  const logs = useAsync<AuditLogView[]>(
    () =>
      token
        ? api.auditLogs(token, category || null, debounced.trim() || null, 300)
        : Promise.resolve([]),
    [token, category, debounced],
    Boolean(token),
  );

  return (
    <div>
      <PageHeader
        title="Audit Logs"
        description="Append-only record of permission changes, moderation actions, command executions, credential updates and configuration changes."
        actions={
          <Button
            size="sm"
            variant="secondary"
            onClick={() => void logs.reload()}
            loading={logs.loading}
            leadingIcon={<RefreshCw size={13} />}
          >
            Refresh
          </Button>
        }
      />

      <Panel>
        <PanelHeader
          title="Activity"
          description={`${logs.data?.length ?? 0} most recent entries`}
          actions={
            <div className="flex flex-wrap items-center gap-2">
              <Select
                aria-label="Filter audit category"
                value={category}
                className="h-8 w-[150px] text-xs"
                onChange={(event) => setCategory(event.target.value)}
              >
                <option value="">All categories</option>
                {(categories.data ?? []).map((entry) => (
                  <option key={entry} value={entry}>
                    {humanize(entry)}
                  </option>
                ))}
              </Select>
              <div className="relative">
                <Search size={13} className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-faint" />
                <Input
                  aria-label="Search audit logs"
                  value={query}
                  onChange={(event) => setQuery(event.target.value)}
                  placeholder="Search action, target, staff"
                  className="h-8 w-[220px] pl-8 text-xs"
                  spellCheck={false}
                />
              </div>
            </div>
          }
        />
        {logs.error ? (
          <div className="p-3">
            <InlineError
              message={logs.error}
              action={
                <Button size="sm" variant="secondary" onClick={() => void logs.reload()}>
                  Retry
                </Button>
              }
            />
          </div>
        ) : logs.loading && !logs.data ? (
          <TableSkeleton rows={10} columns={5} />
        ) : (
          <DataTable<AuditLogView>
            dense
            columns={[
              {
                key: "when",
                header: "Timestamp",
                width: "170px",
                render: (row) => (
                  <span className="text-[10px] text-faint">{formatDateTime(row.createdAt)}</span>
                ),
              },
              {
                key: "staff",
                header: "Staff",
                render: (row) => <span className="text-[12px] text-ink">{row.actorName}</span>,
              },
              {
                key: "category",
                header: "Category",
                render: (row) => (
                  <span className="mono text-[10px] uppercase tracking-wide text-muted">{row.category}</span>
                ),
              },
              {
                key: "action",
                header: "Action",
                render: (row) => <span className="text-[11px] text-ink">{row.action}</span>,
              },
              {
                key: "target",
                header: "Target",
                render: (row) => (
                  <span className="mono text-[11px] text-muted">{row.target || "—"}</span>
                ),
              },
              {
                key: "details",
                header: "Details",
                render: (row) => (
                  <span className="text-[11px] leading-4 text-muted">{row.details || "—"}</span>
                ),
              },
            ]}
            rows={logs.data ?? []}
            getKey={(row) => row.id}
            emptyTitle={query || category ? "No matching audit entries" : "No audit entries yet"}
            emptyDescription={
              query || category
                ? "Adjust the filters to widen the search."
                : "Administrative actions performed in this installation are recorded here."
            }
            maxHeight="640px"
            footer={
              <span className="inline-flex items-center gap-1.5">
                <History size={11} /> Audit records cannot be edited or deleted from the application.
              </span>
            }
          />
        )}
      </Panel>
    </div>
  );
}
