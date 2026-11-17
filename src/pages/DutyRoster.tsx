import { useEffect, useMemo, useState } from "react";
import { LogIn, LogOut, RefreshCw, Timer, UserX } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, formatElapsed, formatRelative, formatSpan } from "@/lib/format";
import { useSession } from "@/state/session";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Textarea } from "@/components/ui/Field";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Modal } from "@/components/ui/Modal";
import { Panel, PanelHeader, StatTile } from "@/components/ui/Panel";
import { Tabs } from "@/components/ui/Tabs";
import { DataTable } from "@/components/ui/Table";
import type { DutyShiftView, DutyStatusView } from "@/types";

type Scope = "mine" | "all";

export default function DutyRosterPage() {
  const { token, has } = useSession();
  const toast = useToast();
  const [scope, setScope] = useState<Scope>("mine");
  const [busy, setBusy] = useState(false);
  const [now, setNow] = useState(() => Date.now());
  const [endTarget, setEndTarget] = useState<DutyShiftView | null>(null);
  const [endReason, setEndReason] = useState("");
  const [ending, setEnding] = useState(false);

  const canViewRoster = has("shifts.view");
  const canManage = has("shifts.manage");

  const duty = useAsync<DutyStatusView>(
    () => {
      if (!token) return Promise.reject(new Error("Sign in to view the duty roster."));
      return api.dutyStatus(token);
    },
    [token],
    Boolean(token),
  );

  const history = useAsync<DutyShiftView[]>(
    () =>
      token
        ? api.listShifts(token, scope === "mine")
        : Promise.resolve([] as DutyShiftView[]),
    [token, scope, duty.data?.shiftCount],
    Boolean(token) && canViewRoster,
  );

  // Keep the running clock and open durations accurate without extra IPC calls.
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(timer);
  }, []);

  const status = duty.data;
  const current = status?.current ?? null;
  const liveElapsed = current ? now - current.startedAt : 0;

  const roster = useMemo(() => status?.roster ?? [], [status]);

  async function clock(action: "in" | "out") {
    if (!token) return;
    setBusy(true);
    try {
      const next = action === "in" ? await api.clockIn(token) : await api.clockOut(token);
      duty.setData(next);
      toast.success(
        action === "in" ? "You are on duty" : "You are off duty",
        action === "in"
          ? "Your shift started and is now visible to supervisors."
          : "Your shift was closed and recorded in the roster history.",
      );
      await history.reload();
    } catch (err) {
      toast.error(action === "in" ? "Could not clock in" : "Could not clock out", errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function endShift() {
    if (!token || !endTarget) return;
    setEnding(true);
    try {
      await api.endShift(token, endTarget.userId, endReason.trim());
      toast.success("Shift ended", `${endTarget.displayName} is now off duty`);
      setEndTarget(null);
      setEndReason("");
      const next = await api.dutyStatus(token);
      duty.setData(next);
      await history.reload();
    } catch (err) {
      toast.error("Could not end the shift", errorMessage(err));
    } finally {
      setEnding(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="Duty Roster"
        description="Real duty shifts recorded in this installation. Durations are calculated from the stored start and end timestamps, and supervisor actions are written to the audit log."
        actions={
          <>
            {current ? (
              <Button
                size="sm"
                variant="primary"
                loading={busy}
                onClick={() => void clock("out")}
                leadingIcon={<LogOut size={13} />}
              >
                Clock out
              </Button>
            ) : (
              <Button
                size="sm"
                variant="primary"
                loading={busy}
                onClick={() => void clock("in")}
                leadingIcon={<LogIn size={13} />}
              >
                Clock in
              </Button>
            )}
            <Button
              size="sm"
              variant="secondary"
              loading={duty.loading}
              onClick={() => void duty.reload()}
              leadingIcon={<RefreshCw size={13} />}
            >
              Refresh
            </Button>
          </>
        }
      />

      {duty.error ? (
        <div className="mb-3">
          <InlineError
            message={duty.error}
            action={
              <Button size="sm" variant="secondary" onClick={() => void duty.reload()}>
                Retry
              </Button>
            }
          />
        </div>
      ) : null}

      <div className="mb-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
        <StatTile
          label="Current shift"
          value={current ? formatElapsed(liveElapsed) : "Off duty"}
          detail={
            current ? (
              <span className="flex items-center gap-1.5">
                <span className="h-1.5 w-1.5 rounded-full bg-accent shadow-[0_0_10px_rgb(255_42_58/0.9)]" />
                Started {formatRelative(current.startedAt)}
              </span>
            ) : (
              "Clock in to start a shift"
            )
          }
          icon={<Timer size={14} />}
          accent={Boolean(current)}
        />
        <StatTile label="Today" value={formatSpan(status?.todayMs ?? 0)} detail="Recorded on this device" />
        <StatTile
          label="This week"
          value={formatSpan(status?.weekMs ?? 0)}
          detail="Monday to now, local time"
        />
        <StatTile
          label="All time"
          value={formatSpan(status?.totalMs ?? 0)}
          detail={`${status?.shiftCount ?? 0} recorded shift${(status?.shiftCount ?? 0) === 1 ? "" : "s"}`}
        />
      </div>

      {canViewRoster ? (
        <Panel className="mb-3">
          <PanelHeader
            title="On duty now"
            description="Open shifts across every staff account. Supervisors can end a forgotten shift with a recorded reason."
            actions={<Badge tone={roster.length > 0 ? "positive" : "muted"} dot>{roster.length} on duty</Badge>}
          />
          {duty.loading && !status ? (
            <TableSkeleton rows={3} columns={4} />
          ) : (
            <DataTable<DutyShiftView>
              columns={[
                {
                  key: "name",
                  header: "Staff member",
                  render: (row) => <span className="text-[12px] text-ink">{row.displayName}</span>,
                },
                {
                  key: "started",
                  header: "Clocked in",
                  render: (row) => (
                    <span className="block">
                      <span className="block text-[11px] text-muted">{formatDateTime(row.startedAt)}</span>
                      <span className="block text-[10px] text-faint">{formatRelative(row.startedAt)}</span>
                    </span>
                  ),
                },
                {
                  key: "duration",
                  header: "On duty",
                  render: (row) => (
                    <span className="mono text-[12px] text-ink">
                      {formatElapsed(Math.max(0, now - row.startedAt))}
                    </span>
                  ),
                },
                {
                  key: "action",
                  header: "",
                  align: "right",
                  render: (row) =>
                    canManage ? (
                      <Button
                        size="sm"
                        variant="secondary"
                        onClick={() => {
                          setEndTarget(row);
                          setEndReason("");
                        }}
                        leadingIcon={<UserX size={12} />}
                      >
                        End shift
                      </Button>
                    ) : (
                      <span className="text-[10px] text-faint">—</span>
                    ),
                },
              ]}
              rows={roster}
              getKey={(row) => row.id}
              emptyTitle="Nobody is on duty"
              emptyDescription="Staff appear here as soon as they clock in."
            />
          )}
        </Panel>
      ) : null}

      <Panel>
        <PanelHeader
          title="Shift history"
          description="Every closed shift, newest first, with its recorded duration."
          actions={
            canViewRoster ? (
              <Tabs<Scope>
                value={scope}
                onChange={setScope}
                items={[
                  { value: "mine", label: "My shifts" },
                  { value: "all", label: "All staff" },
                ]}
              />
            ) : null
          }
        />
        {!canViewRoster ? (
          <div className="px-4 py-6 text-[12px] text-muted">
            Your role records your own shifts but does not include roster visibility.
          </div>
        ) : history.error ? (
          <div className="p-3">
            <InlineError message={history.error} />
          </div>
        ) : history.loading && !history.data ? (
          <TableSkeleton rows={6} columns={4} />
        ) : (
          <DataTable<DutyShiftView>
            columns={[
              {
                key: "name",
                header: "Staff member",
                render: (row) => <span className="text-[12px] text-ink">{row.displayName}</span>,
              },
              {
                key: "window",
                header: "Shift",
                render: (row) => (
                  <span className="block">
                    <span className="block text-[11px] text-muted">{formatDateTime(row.startedAt)}</span>
                    <span className="block text-[10px] text-faint">
                      {row.endedAt ? `Ended ${formatDateTime(row.endedAt)}` : "Open shift"}
                    </span>
                  </span>
                ),
              },
              {
                key: "duration",
                header: "Duration",
                render: (row) => (
                  <span className="mono text-[12px] text-ink">
                    {formatSpan(row.durationMs)}
                  </span>
                ),
              },
              {
                key: "reason",
                header: "Ended because",
                render: (row) => (
                  <span className="text-[11px] text-muted">{row.endReason ?? "—"}</span>
                ),
              },
            ]}
            rows={history.data ?? []}
            getKey={(row) => row.id}
            emptyTitle="No shifts recorded yet"
            emptyDescription="Clock in to create the first shift on this installation."
            maxHeight="480px"
          />
        )}
      </Panel>

      <Modal
        open={endTarget !== null}
        onClose={() => setEndTarget(null)}
        title={`End shift — ${endTarget?.displayName ?? ""}`}
        description="Use this only for a shift that was left open. The reason is stored with the shift and in the audit log."
        footer={
          <>
            <Button variant="ghost" onClick={() => setEndTarget(null)} disabled={ending}>
              Cancel
            </Button>
            <Button
              variant="primary"
              loading={ending}
              disabled={endReason.trim().length < 3}
              onClick={() => void endShift()}
            >
              End shift
            </Button>
          </>
        }
      >
        <Textarea
          label="Reason"
          rows={3}
          value={endReason}
          onChange={(event) => setEndReason(event.target.value)}
          placeholder="e.g. Forgot to clock out at the end of the patrol"
        />
      </Modal>
    </div>
  );
}
