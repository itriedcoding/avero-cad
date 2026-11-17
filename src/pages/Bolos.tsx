import { useState } from "react";
import { CheckCircle2, Plus, RefreshCw, RotateCcw, ScanEye } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime } from "@/lib/format";
import { useSession } from "@/state/session";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input, Select, Textarea } from "@/components/ui/Field";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Modal } from "@/components/ui/Modal";
import { Panel, PanelHeader } from "@/components/ui/Panel";
import { Tabs } from "@/components/ui/Tabs";
import { DataTable } from "@/components/ui/Table";
import type { BoloPriority, BoloView } from "@/types";

type Tab = "active" | "cleared";

const PRIORITY_TONE = {
  critical: "danger",
  high: "warning",
  medium: "neutral",
  low: "muted",
} as const;

interface DraftBolo {
  kind: "person" | "vehicle";
  subjectName: string;
  plate: string;
  description: string;
  lastSeen: string;
  priority: BoloPriority;
}

const EMPTY_DRAFT: DraftBolo = {
  kind: "person",
  subjectName: "",
  plate: "",
  description: "",
  lastSeen: "",
  priority: "medium",
};

export default function BolosPage() {
  const { token, has } = useSession();
  const toast = useToast();
  const [tab, setTab] = useState<Tab>("active");
  const [draft, setDraft] = useState<DraftBolo>(EMPTY_DRAFT);
  const [creating, setCreating] = useState(false);
  const [formOpen, setFormOpen] = useState(false);
  const [clearTarget, setClearTarget] = useState<BoloView | null>(null);
  const [clearNote, setClearNote] = useState("");
  const [clearing, setClearing] = useState(false);
  const [busyId, setBusyId] = useState<number | null>(null);

  const canManage = has("bolos.manage");

  const bolos = useAsync<BoloView[]>(
    () => (token ? api.listBolos(token, tab) : Promise.resolve([])),
    [token, tab],
    Boolean(token),
  );

  const rows = bolos.data ?? [];

  function openForm() {
    setDraft(EMPTY_DRAFT);
    setFormOpen(true);
  }

  async function submitDraft() {
    if (!token) return;
    setCreating(true);
    try {
      await api.createBolo(token, draft);
      toast.success("BOLO created", `${draft.subjectName} is now on the active board`);
      setFormOpen(false);
      setTab("active");
      await bolos.reload();
    } catch (err) {
      toast.error("Could not create the BOLO", errorMessage(err));
    } finally {
      setCreating(false);
    }
  }

  async function clearBolo() {
    if (!token || !clearTarget) return;
    setClearing(true);
    try {
      await api.clearBolo(token, clearTarget.id, clearNote.trim());
      toast.success("BOLO cleared", clearTarget.subjectName);
      setClearTarget(null);
      setClearNote("");
      await bolos.reload();
    } catch (err) {
      toast.error("Could not clear the BOLO", errorMessage(err));
    } finally {
      setClearing(false);
    }
  }

  async function reopenBolo(bolo: BoloView) {
    if (!token) return;
    setBusyId(bolo.id);
    try {
      await api.reopenBolo(token, bolo.id);
      toast.success("BOLO reopened", bolo.subjectName);
      await bolos.reload();
    } catch (err) {
      toast.error("Could not reopen the BOLO", errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  const draftValid =
    draft.subjectName.trim().length >= 2 &&
    (draft.kind !== "vehicle" || draft.plate.replace(/[^a-zA-Z0-9]/g, "").length >= 2);

  return (
    <div>
      <PageHeader
        title="BOLO Board"
        description="Be-on-the-lookout reports for people and vehicles. Active reports are matched against the latest live server snapshot when one is available; a match is only shown for an exact username or plate."
        actions={
          <>
            {canManage ? (
              <Button size="sm" variant="primary" onClick={openForm} leadingIcon={<Plus size={13} />}>
                New BOLO
              </Button>
            ) : null}
            <Button
              size="sm"
              variant="secondary"
              loading={bolos.loading}
              onClick={() => void bolos.reload()}
              leadingIcon={<RefreshCw size={13} />}
            >
              Refresh
            </Button>
          </>
        }
      />

      <div className="mb-3 flex flex-wrap items-center justify-between gap-2">
        <Tabs<Tab>
          value={tab}
          onChange={setTab}
          items={[
            { value: "active", label: "Active", count: tab === "active" ? rows.length : undefined },
            { value: "cleared", label: "Cleared", count: tab === "cleared" ? rows.length : undefined },
          ]}
        />
        <span className="text-[11px] text-faint">
          Matching uses the most recent live snapshot held by the app.
        </span>
      </div>

      <Panel>
        <PanelHeader
          title={tab === "active" ? "Active BOLOs" : "Cleared BOLOs"}
          description={
            tab === "active"
              ? "Everyone with BOLO access sees this board. Clearing a BOLO requires a resolution note and is written to the audit log."
              : "Cleared reports stay searchable so the record of what was broadcast is preserved."
          }
          actions={<Badge tone="neutral">{rows.length} shown</Badge>}
        />
        {bolos.error ? (
          <div className="p-3">
            <InlineError
              message={bolos.error}
              action={
                <Button size="sm" variant="secondary" onClick={() => void bolos.reload()}>
                  Retry
                </Button>
              }
            />
          </div>
        ) : bolos.loading && !bolos.data ? (
          <TableSkeleton rows={6} columns={5} />
        ) : (
          <DataTable<BoloView>
            columns={[
              {
                key: "subject",
                header: "Subject",
                render: (row) => (
                  <span className="flex items-center gap-2">
                    <span className="flex h-6 w-6 shrink-0 items-center justify-center rounded border border-line bg-surface-3 text-faint">
                      <ScanEye size={12} />
                    </span>
                    <span className="min-w-0">
                      <span className="block truncate text-[12px] text-ink">{row.subjectName}</span>
                      <span className="block text-[10px] uppercase tracking-[0.08em] text-faint">
                        {row.kind}
                        {row.plate ? ` · ${row.plate}` : ""}
                      </span>
                    </span>
                  </span>
                ),
              },
              {
                key: "detail",
                header: "Details",
                render: (row) => (
                  <span className="block max-w-[280px]">
                    <span className="block truncate text-[11px] text-muted">
                      {row.description || "No description"}
                    </span>
                    <span className="block truncate text-[10px] text-faint">
                      {row.lastSeen ? `Last seen: ${row.lastSeen}` : "No last-seen location"}
                    </span>
                  </span>
                ),
              },
              {
                key: "priority",
                header: "Priority",
                render: (row) => (
                  <Badge tone={PRIORITY_TONE[row.priority]} dot>
                    {row.priority}
                  </Badge>
                ),
              },
              {
                key: "live",
                header: "Live",
                render: (row) =>
                  row.liveMatch ? (
                    <Badge tone="positive" dot>
                      {row.liveMatch}
                    </Badge>
                  ) : row.status === "active" ? (
                    <span className="text-[10px] text-faint">Not seen</span>
                  ) : (
                    <span className="text-[10px] text-faint">
                      {row.clearedAt ? `Cleared ${formatDateTime(row.clearedAt)}` : "Cleared"}
                    </span>
                  ),
              },
              {
                key: "meta",
                header: "Filed",
                render: (row) => (
                  <span className="block">
                    <span className="block text-[11px] text-muted">{row.createdByName}</span>
                    <span className="block text-[10px] text-faint">{formatDateTime(row.createdAt)}</span>
                  </span>
                ),
              },
              {
                key: "action",
                header: "",
                align: "right",
                render: (row) =>
                  canManage ? (
                    row.status === "active" ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => {
                          setClearTarget(row);
                          setClearNote("");
                        }}
                        leadingIcon={<CheckCircle2 size={12} />}
                      >
                        Clear
                      </Button>
                    ) : (
                      <Button
                        size="sm"
                        variant="ghost"
                        loading={busyId === row.id}
                        onClick={() => void reopenBolo(row)}
                        leadingIcon={<RotateCcw size={12} />}
                      >
                        Reopen
                      </Button>
                    )
                  ) : (
                    <span className="text-[10px] text-faint">—</span>
                  ),
              },
            ]}
            rows={rows}
            getKey={(row) => row.id}
            emptyTitle={tab === "active" ? "No active BOLOs" : "No cleared BOLOs"}
            emptyDescription={
              tab === "active"
                ? "Create a BOLO when a person or vehicle needs to be watched for."
                : "Cleared reports appear here with their resolution notes."
            }
            maxHeight="560px"
          />
        )}
      </Panel>

      <Modal
        open={formOpen}
        onClose={() => setFormOpen(false)}
        title="New BOLO"
        description="This is a CAD record for staff. It does not change anything inside the ER:LC server."
        footer={
          <>
            <Button variant="ghost" onClick={() => setFormOpen(false)} disabled={creating}>
              Cancel
            </Button>
            <Button
              variant="primary"
              loading={creating}
              disabled={!draftValid}
              onClick={() => void submitDraft()}
            >
              Create BOLO
            </Button>
          </>
        }
      >
        <div className="grid grid-cols-2 gap-3">
          <Select
            label="Subject type"
            value={draft.kind}
            onChange={(event) =>
              setDraft({ ...draft, kind: event.target.value as DraftBolo["kind"] })
            }
          >
            <option value="person">Person</option>
            <option value="vehicle">Vehicle</option>
          </Select>
          <Select
            label="Priority"
            value={draft.priority}
            onChange={(event) =>
              setDraft({ ...draft, priority: event.target.value as BoloPriority })
            }
          >
            <option value="low">Low</option>
            <option value="medium">Medium</option>
            <option value="high">High</option>
            <option value="critical">Critical</option>
          </Select>
        </div>
        <div className="mt-3">
          <Input
            label={draft.kind === "vehicle" ? "Vehicle or model" : "Roblox username"}
            value={draft.subjectName}
            maxLength={80}
            onChange={(event) => setDraft({ ...draft, subjectName: event.target.value })}
            placeholder={draft.kind === "vehicle" ? "Falcon" : "Builderman"}
          />
        </div>
        {draft.kind === "vehicle" ? (
          <div className="mt-3">
            <Input
              label="Plate"
              value={draft.plate}
              maxLength={12}
              onChange={(event) => setDraft({ ...draft, plate: event.target.value })}
              placeholder="ABC-123"
              hint="Matched against live spawned vehicles"
            />
          </div>
        ) : null}
        <div className="mt-3">
          <Input
            label="Last seen"
            value={draft.lastSeen}
            maxLength={200}
            onChange={(event) => setDraft({ ...draft, lastSeen: event.target.value })}
            placeholder="e.g. Route 1 near the tunnel, 10 minutes ago"
          />
        </div>
        <div className="mt-3">
          <Textarea
            label="Description"
            rows={3}
            value={draft.description}
            maxLength={2000}
            onChange={(event) => setDraft({ ...draft, description: event.target.value })}
            placeholder="What staff should look for, why the subject is wanted, and any handling notes."
          />
        </div>
      </Modal>

      <Modal
        open={clearTarget !== null}
        onClose={() => setClearTarget(null)}
        title={`Clear BOLO — ${clearTarget?.subjectName ?? ""}`}
        description="The report stays in the cleared list with your note for the audit trail."
        footer={
          <>
            <Button variant="ghost" onClick={() => setClearTarget(null)} disabled={clearing}>
              Cancel
            </Button>
            <Button
              variant="primary"
              loading={clearing}
              disabled={clearNote.trim().length < 3}
              onClick={() => void clearBolo()}
            >
              Clear BOLO
            </Button>
          </>
        }
      >
        <Textarea
          label="Resolution"
          rows={3}
          value={clearNote}
          onChange={(event) => setClearNote(event.target.value)}
          placeholder="e.g. Subject located and detained by 1A-1"
        />
      </Modal>
    </div>
  );
}
