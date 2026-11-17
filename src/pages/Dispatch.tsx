import { useCallback, useEffect, useState } from "react";
import { Plus, Trash2, UserPlus, X } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatRelative, formatTime } from "@/lib/format";
import { CALL_PRIORITIES, CALL_STATUSES, UNIT_STATUSES, callStatusLabel, callStatusTone, priorityTone, unitStatusLabel, unitStatusTone } from "@/lib/status";
import { useSession } from "@/state/session";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { FieldLabel, Input, Select, Textarea } from "@/components/ui/Field";
import { ConfirmDialog, Modal } from "@/components/ui/Modal";
import { EmptyState, InlineError, PageHeader, Skeleton } from "@/components/ui/Feedback";
import { Panel, PanelBody, PanelHeader, SectionTitle } from "@/components/ui/Panel";
import { Tabs } from "@/components/ui/Tabs";
import type { CallView, SearchResult, UnitView } from "@/types";

type Filter = "active" | "closed" | "all";

const EMPTY_FORM = { title: "", description: "", location: "", postal: "", priority: "medium" };

export default function DispatchPage() {
  const { token: sessionToken, has } = useSession();
  // AppShell only renders this page with an active session.
  const token = sessionToken ?? "";
  const toast = useToast();
  const [filter, setFilter] = useState<Filter>("active");
  const [calls, setCalls] = useState<CallView[]>([]);
  const [units, setUnits] = useState<UnitView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [form, setForm] = useState(EMPTY_FORM);
  const [saving, setSaving] = useState(false);
  const [unitOpen, setUnitOpen] = useState(false);
  const [unitForm, setUnitForm] = useState({ callsign: "", label: "", team: "Police", status: "available" });
  const [attachOpen, setAttachOpen] = useState(false);
  const [attachQuery, setAttachQuery] = useState("");
  const [attachResults, setAttachResults] = useState<SearchResult[]>([]);
  const [closeTarget, setCloseTarget] = useState<CallView | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<UnitView | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    if (!token) return;
    setLoading(true);
    try {
      const status = filter === "closed" ? "closed" : null;
      const [nextCalls, nextUnits] = await Promise.all([
        api.listCalls(token, status, 200),
        api.listUnits(token),
      ]);
      setCalls(filter === "active" ? nextCalls.filter((call) => call.status !== "closed") : nextCalls);
      setUnits(nextUnits);
      setError(null);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, [token, filter]);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    if (!attachOpen || !token) return;
    const timer = window.setTimeout(() => {
      if (attachQuery.trim().length < 2) {
        setAttachResults([]);
        return;
      }
      api
        .searchPlayers(token, attachQuery.trim())
        .then((results) => setAttachResults(results.filter((row) => row.online)))
        .catch(() => setAttachResults([]));
    }, 250);
    return () => window.clearTimeout(timer);
  }, [attachQuery, attachOpen, token]);

  const selected = calls.find((call) => call.id === selectedId) ?? null;

  async function run(action: () => Promise<unknown>, success: string) {
    setBusy(true);
    try {
      await action();
      toast.success(success);
      await load();
    } catch (err) {
      toast.error("Dispatch action failed", errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function createCall() {
    if (!token) return;
    setSaving(true);
    try {
      const created = await api.createCall(token, form);
      setCreateOpen(false);
      setForm(EMPTY_FORM);
      setSelectedId(created.id);
      toast.success(`Call #${created.callNumber} created`);
      await load();
    } catch (err) {
      toast.error("Could not create call", errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  async function createUnit() {
    if (!token) return;
    setSaving(true);
    try {
      await api.createUnit(token, unitForm.callsign, unitForm.label, unitForm.team, unitForm.status);
      setUnitOpen(false);
      setUnitForm({ callsign: "", label: "", team: "Police", status: "available" });
      toast.success("Unit created");
      await load();
    } catch (err) {
      toast.error("Could not create unit", errorMessage(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="CAD Dispatch"
        description="Create calls, assign units and track incident status. CAD calls are internal records; calls mirrored from ER:LC are labelled as such."
        actions={
          has("dispatch.create") ? (
            <Button variant="primary" size="sm" onClick={() => setCreateOpen(true)} leadingIcon={<Plus size={13} />}>
              New call
            </Button>
          ) : null
        }
      />

      {error ? <div className="mb-3"><InlineError message={error} /></div> : null}

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
        <Panel className="flex min-h-[560px] flex-col">
          <PanelHeader
            title="Calls"
            description={`${calls.filter((call) => call.status !== "closed").length} active`}
            actions={
              <Tabs<Filter>
                value={filter}
                onChange={setFilter}
                items={[
                  { value: "active", label: "Active" },
                  { value: "closed", label: "Closed" },
                  { value: "all", label: "All" },
                ]}
              />
            }
          />
          <PanelBody padded={false} className="min-h-0 flex-1 overflow-y-auto">
            {loading ? (
              <div className="space-y-2 p-3">
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
              </div>
            ) : calls.length === 0 ? (
              <EmptyState
                title={filter === "closed" ? "No closed calls" : "No calls on the board"}
                description="Create a call to start dispatching units."
                action={
                  has("dispatch.create") ? (
                    <Button size="sm" variant="secondary" onClick={() => setCreateOpen(true)}>
                      New call
                    </Button>
                  ) : undefined
                }
              />
            ) : (
              <ul className="divide-y divide-line/70">
                {calls.map((call) => (
                  <li key={call.id}>
                    <button
                      type="button"
                      onClick={() => setSelectedId(call.id)}
                      className={`flex w-full items-start gap-3 px-3.5 py-3 text-left transition-colors ${
                        selectedId === call.id ? "bg-accent/[0.1]" : "row-hover"
                      }`}
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex flex-wrap items-center gap-1.5">
                          <span className="mono text-[12px] font-semibold text-ink">#{call.callNumber}</span>
                          <Badge tone={callStatusTone(call.status)}>{callStatusLabel(call.status)}</Badge>
                          <Badge tone={priorityTone(call.priority)}>{call.priority}</Badge>
                          {call.source === "erlc" ? <Badge tone="neutral">ER:LC</Badge> : null}
                        </div>
                        <p className="mt-1 truncate text-[13px] text-ink">{call.title}</p>
                        <p className="truncate text-[11px] text-faint">
                          {call.location || call.postal || "No location"} · {formatTime(call.createdAt)}
                          {call.units.length > 0 ? ` · ${call.units.length} unit(s)` : ""}
                        </p>
                      </div>
                      <span className="shrink-0 pt-0.5 text-[10px] text-faint">
                        {call.closedAt ? `closed ${formatRelative(call.closedAt)}` : formatRelative(call.updatedAt)}
                      </span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </PanelBody>
        </Panel>

        <div className="space-y-4">
          <Panel>
            <PanelHeader
              title={selected ? `Call #${selected.callNumber}` : "Call details"}
              description={
                selected
                  ? `${callStatusLabel(selected.status)} · opened by ${selected.createdByName}`
                  : "Select a call to assign units, edit details or close it."
              }
              actions={
                selected && has("dispatch.close") && selected.status !== "closed" ? (
                  <Button size="sm" variant="danger" onClick={() => setCloseTarget(selected)}>
                    Close call
                  </Button>
                ) : null
              }
            />
            <PanelBody>
              {!selected ? (
                <p className="py-6 text-center text-xs text-faint">No call selected.</p>
              ) : (
                <div className="space-y-3">
                  <div className="grid gap-3 sm:grid-cols-2">
                    <div className="sm:col-span-2">
                      <FieldLabel htmlFor="call-title">Title</FieldLabel>
                      <Input
                        id="call-title"
                        value={selected.title}
                        disabled={!has("dispatch.update") || selected.status === "closed"}
                        onChange={(event) =>
                          setCalls((current) =>
                            current.map((call) =>
                              call.id === selected.id ? { ...call, title: event.target.value } : call,
                            ),
                          )
                        }
                      />
                    </div>
                    <Input
                      label="Location"
                      value={selected.location}
                      disabled={!has("dispatch.update") || selected.status === "closed"}
                      onChange={(event) =>
                        setCalls((current) =>
                          current.map((call) =>
                            call.id === selected.id ? { ...call, location: event.target.value } : call,
                          ),
                        )
                      }
                    />
                    <Input
                      label="Postal"
                      value={selected.postal}
                      disabled={!has("dispatch.update") || selected.status === "closed"}
                      onChange={(event) =>
                        setCalls((current) =>
                          current.map((call) =>
                            call.id === selected.id ? { ...call, postal: event.target.value } : call,
                          ),
                        )
                      }
                    />
                    <Select
                      label="Priority"
                      value={selected.priority}
                      disabled={!has("dispatch.update") || selected.status === "closed"}
                      onChange={(event) =>
                        setCalls((current) =>
                          current.map((call) =>
                            call.id === selected.id
                              ? { ...call, priority: event.target.value as CallView["priority"] }
                              : call,
                          ),
                        )
                      }
                    >
                      {CALL_PRIORITIES.map((priority) => (
                        <option key={priority.value} value={priority.value}>
                          {priority.label}
                        </option>
                      ))}
                    </Select>
                    <Select
                      label="Status"
                      value={selected.status}
                      disabled={!has("dispatch.update") || selected.status === "closed"}
                      onChange={(event) =>
                        run(
                          () => api.setCallStatus(token, selected.id, event.target.value),
                          "Call status updated",
                        )
                      }
                    >
                      {CALL_STATUSES.filter((status) => status.value !== "closed").map((status) => (
                        <option key={status.value} value={status.value}>
                          {status.label}
                        </option>
                      ))}
                    </Select>
                  </div>
                  <div>
                    <FieldLabel htmlFor="call-description">Description</FieldLabel>
                    <Textarea
                      id="call-description"
                      rows={3}
                      value={selected.description}
                      disabled={!has("dispatch.update") || selected.status === "closed"}
                      onChange={(event) =>
                        setCalls((current) =>
                          current.map((call) =>
                            call.id === selected.id
                              ? { ...call, description: event.target.value }
                              : call,
                          ),
                        )
                      }
                    />
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    {has("dispatch.update") && selected.status !== "closed" ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        loading={busy}
                        onClick={() =>
                          run(
                            () =>
                              api.updateCall(token, selected.id, {
                                title: selected.title,
                                description: selected.description,
                                location: selected.location,
                                postal: selected.postal,
                                priority: selected.priority,
                              }),
                            "Call updated",
                          )
                        }
                      >
                        Save changes
                      </Button>
                    ) : null}
                    {has("dispatch.update") && selected.status !== "closed" ? (
                      <Button size="sm" variant="secondary" onClick={() => setAttachOpen(true)} leadingIcon={<UserPlus size={13} />}>
                        Attach player
                      </Button>
                    ) : null}
                  </div>

                  <div className="border-t border-line pt-3">
                    <SectionTitle
                      actions={
                        has("dispatch.update") && selected.status !== "closed" ? (
                          <Select
                            aria-label="Assign unit"
                            value=""
                            onChange={(event) => {
                              const unitId = Number(event.target.value);
                              if (!unitId) return;
                              void run(
                                () => api.assignUnit(token, selected.id, unitId),
                                "Unit assigned to call",
                              );
                            }}
                            className="h-7 w-40 text-[11px]"
                          >
                            <option value="">Assign unit…</option>
                            {units
                              .filter((unit) => unit.assignedCallId !== selected.id)
                              .map((unit) => (
                                <option key={unit.id} value={unit.id}>
                                  {unit.callsign} · {unitStatusLabel(unit.status)}
                                </option>
                              ))}
                          </Select>
                        ) : undefined
                      }
                    >
                      Assigned units
                    </SectionTitle>
                    {selected.units.length === 0 ? (
                      <p className="text-xs text-faint">No units assigned.</p>
                    ) : (
                      <ul className="space-y-1.5">
                        {selected.units.map((unit) => (
                          <li key={unit.id} className="flex items-center justify-between gap-2 rounded border border-line bg-surface-2 px-2.5 py-1.5">
                            <span className="flex items-center gap-2">
                              <span className="mono text-[12px] font-medium text-ink">{unit.callsign}</span>
                              <Badge tone={unitStatusTone(unit.status)}>{unitStatusLabel(unit.status)}</Badge>
                              <span className="text-[11px] text-faint">{unit.team}</span>
                            </span>
                            {has("dispatch.update") ? (
                              <IconButton label="Unassign unit" onClick={() => void run(() => api.unassignUnit(token, unit.id), "Unit unassigned")}>
                                <X size={13} />
                              </IconButton>
                            ) : null}
                          </li>
                        ))}
                      </ul>
                    )}
                  </div>

                  {selected.players.length > 0 ? (
                    <div className="border-t border-line pt-3">
                      <SectionTitle>Associated players</SectionTitle>
                      <div className="flex flex-wrap gap-1.5">
                        {selected.players.map((player) => (
                          <span key={player.robloxUserId} className="inline-flex items-center gap-1.5 rounded border border-line bg-surface-2 px-2 py-1 text-[11px] text-ink">
                            {player.username}
                            {has("dispatch.update") ? (
                              <button
                                type="button"
                                className="text-faint hover:text-ink"
                                onClick={() =>
                                  void run(
                                    () =>
                                      api.detachPlayerFromCall(token, selected.id, player.robloxUserId),
                                    "Player detached",
                                  )
                                }
                                aria-label={`Detach ${player.username}`}
                              >
                                <X size={11} />
                              </button>
                            ) : null}
                          </span>
                        ))}
                      </div>
                    </div>
                  ) : null}

                  {selected.closedAt ? (
                    <p className="border-t border-line pt-3 text-[11px] text-faint">
                      Closed by {selected.closedByName ?? "unknown"} · {formatRelative(selected.closedAt)}
                    </p>
                  ) : null}
                </div>
              )}
            </PanelBody>
          </Panel>

          <Panel>
            <PanelHeader
              title="Units"
              description="CAD unit board. Units are internal records, not synced game entities."
              actions={
                has("dispatch.units") ? (
                  <Button size="sm" variant="secondary" onClick={() => setUnitOpen(true)} leadingIcon={<Plus size={13} />}>
                    Unit
                  </Button>
                ) : null
              }
            />
            <PanelBody padded={false}>
              {units.length === 0 ? (
                <EmptyState compact title="No units" description="Create a unit to assign it to calls." />
              ) : (
                <ul className="divide-y divide-line/70">
                  {units.map((unit) => (
                    <li key={unit.id} className="flex flex-wrap items-center gap-2 px-3.5 py-2">
                      <span className="mono w-16 shrink-0 text-[12px] font-semibold text-ink">{unit.callsign}</span>
                      <span className="min-w-0 flex-1 truncate text-[11px] text-muted">
                        {unit.team || "Unassigned team"}
                        {unit.assignedCallId
                          ? ` · #${calls.find((call) => call.id === unit.assignedCallId)?.callNumber ?? unit.assignedCallId}`
                          : ""}
                      </span>
                      {has("dispatch.units") ? (
                        <Select
                          aria-label={`Status for ${unit.callsign}`}
                          value={unit.status}
                          onChange={(event) =>
                            void run(
                              () =>
                                api.updateUnit(token, unit.id, unit.label, unit.team, event.target.value),
                              "Unit status updated",
                            )
                          }
                          className="h-7 w-32 text-[11px]"
                        >
                          {UNIT_STATUSES.map((status) => (
                            <option key={status.value} value={status.value}>
                              {status.label}
                            </option>
                          ))}
                        </Select>
                      ) : (
                        <Badge tone={unitStatusTone(unit.status)}>{unitStatusLabel(unit.status)}</Badge>
                      )}
                      {has("dispatch.units") ? (
                        <IconButton label={`Delete ${unit.callsign}`} onClick={() => setDeleteTarget(unit)}>
                          <Trash2 size={13} />
                        </IconButton>
                      ) : null}
                    </li>
                  ))}
                </ul>
              )}
            </PanelBody>
          </Panel>
        </div>
      </div>

      <Modal
        open={createOpen}
        onClose={() => setCreateOpen(false)}
        title="Create CAD call"
        description="CAD calls are internal dispatch records. They are not created inside ER:LC."
        footer={
          <>
            <Button variant="ghost" onClick={() => setCreateOpen(false)} disabled={saving}>
              Cancel
            </Button>
            <Button variant="primary" onClick={createCall} loading={saving} disabled={form.title.trim().length < 3}>
              Create call
            </Button>
          </>
        }
      >
        <div className="space-y-3">
          <Input
            label="Title"
            value={form.title}
            autoFocus
            placeholder="e.g. Two-vehicle collision"
            onChange={(event) => setForm({ ...form, title: event.target.value })}
          />
          <Textarea
            label="Description"
            rows={3}
            value={form.description}
            placeholder="What is being reported?"
            onChange={(event) => setForm({ ...form, description: event.target.value })}
          />
          <div className="grid gap-3 sm:grid-cols-3">
            <Input
              label="Location"
              value={form.location}
              placeholder="Street or landmark"
              onChange={(event) => setForm({ ...form, location: event.target.value })}
            />
            <Input
              label="Postal"
              value={form.postal}
              placeholder="e.g. 302"
              onChange={(event) => setForm({ ...form, postal: event.target.value })}
            />
            <Select
              label="Priority"
              value={form.priority}
              onChange={(event) => setForm({ ...form, priority: event.target.value })}
            >
              {CALL_PRIORITIES.map((priority) => (
                <option key={priority.value} value={priority.value}>
                  {priority.label}
                </option>
              ))}
            </Select>
          </div>
        </div>
      </Modal>

      <Modal
        open={unitOpen}
        onClose={() => setUnitOpen(false)}
        title="Create unit"
        footer={
          <>
            <Button variant="ghost" onClick={() => setUnitOpen(false)} disabled={saving}>
              Cancel
            </Button>
            <Button variant="primary" onClick={createUnit} loading={saving} disabled={unitForm.callsign.trim().length === 0}>
              Create unit
            </Button>
          </>
        }
      >
        <div className="space-y-3">
          <Input
            label="Callsign"
            value={unitForm.callsign}
            autoFocus
            placeholder="e.g. 1A-12"
            onChange={(event) => setUnitForm({ ...unitForm, callsign: event.target.value })}
          />
          <Input
            label="Label"
            value={unitForm.label}
            placeholder="Optional description"
            onChange={(event) => setUnitForm({ ...unitForm, label: event.target.value })}
          />
          <div className="grid gap-3 sm:grid-cols-2">
            <Input
              label="Team"
              value={unitForm.team}
              onChange={(event) => setUnitForm({ ...unitForm, team: event.target.value })}
            />
            <Select
              label="Status"
              value={unitForm.status}
              onChange={(event) => setUnitForm({ ...unitForm, status: event.target.value })}
            >
              {UNIT_STATUSES.map((status) => (
                <option key={status.value} value={status.value}>
                  {status.label}
                </option>
              ))}
            </Select>
          </div>
        </div>
      </Modal>

      <Modal
        open={attachOpen}
        onClose={() => setAttachOpen(false)}
        title="Attach player to call"
        description="Only players currently in your ER:LC server can be attached."
      >
        <div className="space-y-3">
          <Input
            label="Search players"
            value={attachQuery}
            autoFocus
            placeholder="Username, ID or callsign"
            onChange={(event) => setAttachQuery(event.target.value)}
          />
          <div className="max-h-64 overflow-y-auto rounded border border-line">
            {attachResults.length === 0 ? (
              <p className="px-3 py-4 text-center text-xs text-faint">
                {attachQuery.trim().length < 2 ? "Type at least 2 characters." : "No matching players in the server."}
              </p>
            ) : (
              <ul className="divide-y divide-line/70">
                {attachResults.map((result) => (
                  <li key={`${result.username}-${result.robloxUserId ?? "x"}`}>
                    <button
                      type="button"
                      className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left row-hover"
                      onClick={() => {
                        if (!selected || !result.robloxUserId) return;
                        void run(
                          () =>
                            api.attachPlayerToCall(
                              token,
                              selected.id,
                              result.robloxUserId as number,
                              result.username,
                            ),
                          "Player attached to call",
                        ).then(() => setAttachOpen(false));
                      }}
                    >
                      <span className="text-[12px] text-ink">{result.username}</span>
                      <span className="text-[10px] text-faint">{result.team ?? "—"}</span>
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={closeTarget !== null}
        onClose={() => setCloseTarget(null)}
        onConfirm={() => {
          if (!closeTarget) return;
          void run(() => api.setCallStatus(token, closeTarget.id, "closed"), "Call closed");
          setCloseTarget(null);
        }}
        title={`Close call #${closeTarget?.callNumber ?? ""}`}
        description="Closing a call releases its assigned units and cannot be undone from the CAD board."
        confirmLabel="Close call"
        destructive
      />

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={() => {
          if (!deleteTarget) return;
          void run(() => api.deleteUnit(token, deleteTarget.id), "Unit deleted");
          setDeleteTarget(null);
        }}
        title={`Delete unit ${deleteTarget?.callsign ?? ""}`}
        description="The unit is removed from the CAD unit board. This does not affect players in ER:LC."
        confirmLabel="Delete unit"
        destructive
      />
    </div>
  );
}
