import { useCallback, useState } from "react";
import { Activity, BarChart3, Clock, RefreshCw, Users } from "lucide-react";
import { api } from "@/lib/api";
import { formatDateTime, formatNumber, formatSpan } from "@/lib/format";
import { useSession } from "@/state/session";
import { useAsync } from "@/hooks/useAsync";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { Panel, PanelHeader, StatTile } from "@/components/ui/Panel";
import { Tabs } from "@/components/ui/Tabs";
import { DataTable } from "@/components/ui/Table";
import type { DailyStatView, HourStatView, StatsOverviewView, TopModeratorView } from "@/types";

const RANGES = [
  { value: "7", label: "7 days" },
  { value: "14", label: "14 days" },
  { value: "30", label: "30 days" },
  { value: "90", label: "90 days" },
];

function DailyChart({ data }: { data: DailyStatView[] }) {
  const peak = Math.max(
    1,
    ...data.map((day) => Math.max(day.callsOpened, day.moderation, day.commands)),
  );
  return (
    <div>
      <div className="flex h-40 items-end gap-1">
        {data.map((day) => (
          <div key={day.date} className="flex h-full min-w-0 flex-1 items-end gap-[2px]">
            {[
              { value: day.callsOpened, tone: "bg-accent", label: "calls opened" },
              { value: day.moderation, tone: "bg-accent/55", label: "moderation actions" },
              { value: day.commands, tone: "bg-accent/28", label: "commands" },
            ].map((series) => (
              <div
                key={series.label}
                className={`w-1/3 rounded-t ${series.tone}`}
                style={{
                  height: series.value > 0 ? `${Math.max(3, (series.value / peak) * 100)}%` : "0%",
                }}
                title={`${day.date}: ${series.value} ${series.label}`}
              />
            ))}
          </div>
        ))}
      </div>
      <div className="mt-1.5 flex justify-between text-[9px] text-faint">
        <span>{data[0]?.date.slice(5) ?? ""}</span>
        <span className="flex items-center gap-3">
          <span className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-sm bg-accent" /> Calls
          </span>
          <span className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-sm bg-accent/55" /> Moderation
          </span>
          <span className="flex items-center gap-1">
            <span className="h-2 w-2 rounded-sm bg-accent/28" /> Commands
          </span>
        </span>
        <span>{data[data.length - 1]?.date.slice(5) ?? ""}</span>
      </div>
    </div>
  );
}

function HourChart({ data }: { data: HourStatView[] }) {
  const peak = Math.max(1, ...data.map((hour) => hour.calls + hour.moderation + hour.commands));
  return (
    <div>
      <div className="flex h-28 items-end gap-[3px]">
        {data.map((hour) => {
          const total = hour.calls + hour.moderation + hour.commands;
          return (
            <div
              key={hour.hour}
              className="flex-1 rounded-t bg-accent/70"
              style={{ height: total > 0 ? `${Math.max(4, (total / peak) * 100)}%` : "0%" }}
              title={`${hour.hour.toString().padStart(2, "0")}:00 — ${hour.calls} calls, ${hour.moderation} moderation, ${hour.commands} commands`}
            />
          );
        })}
      </div>
      <div className="mt-1.5 flex justify-between text-[9px] text-faint">
        <span>00:00</span>
        <span>06:00</span>
        <span>12:00</span>
        <span>18:00</span>
        <span>23:00</span>
      </div>
    </div>
  );
}

export default function StatisticsPage() {
  const { token } = useSession();
  const [days, setDays] = useState("14");

  const load = useCallback(async () => {
    if (!token) throw new Error("Sign in to view statistics.");
    return api.statsOverview(token, Number(days));
  }, [token, days]);

  const stats = useAsync<StatsOverviewView>(load, [load]);
  const data = stats.data;

  return (
    <div>
      <PageHeader
        title="Statistics"
        description="Aggregated from the records this installation actually stored: CAD calls, moderation actions, executed commands, duty shifts and sampled player counts. Nothing here is estimated."
        actions={
          <>
            <Tabs<string>
              value={days}
              onChange={setDays}
              items={RANGES.map((range) => ({ value: range.value, label: range.label }))}
            />
            <Button
              size="sm"
              variant="secondary"
              loading={stats.loading}
              onClick={() => void stats.reload()}
              leadingIcon={<RefreshCw size={13} />}
            >
              Refresh
            </Button>
          </>
        }
      />

      {stats.error ? (
        <div className="mb-3">
          <InlineError
            message={stats.error}
            action={
              <Button size="sm" variant="secondary" onClick={() => void stats.reload()}>
                Retry
              </Button>
            }
          />
        </div>
      ) : null}

      {!data && stats.loading ? (
        <Panel>
          <TableSkeleton rows={6} columns={4} />
        </Panel>
      ) : data ? (
        <>
          <div className="mb-3 grid grid-cols-2 gap-3 lg:grid-cols-4">
            <StatTile
              label="Calls opened"
              value={formatNumber(data.callsOpened)}
              detail={`${data.callsClosed} closed · ${data.callsOpenNow} still open`}
              icon={<Activity size={14} />}
              accent
            />
            <StatTile
              label="Average call time"
              value={data.avgCloseMs === null ? "—" : formatSpan(data.avgCloseMs)}
              detail="From creation to close"
              icon={<Clock size={14} />}
            />
            <StatTile
              label="Moderation"
              value={formatNumber(data.moderationTotal)}
              detail={`${data.kicks} kicks · ${data.bans} bans · ${data.unbans} unbans${
                data.moderationFailed > 0 ? ` · ${data.moderationFailed} failed` : ""
              }`}
            />
            <StatTile
              label="Commands"
              value={formatNumber(data.commandsTotal)}
              detail={
                data.commandsFailed > 0
                  ? `${data.commandsFailed} rejected or failed`
                  : "All commands succeeded"
              }
            />
            <StatTile
              label="Duty hours"
              value={formatSpan(data.dutyMs)}
              detail={`${data.dutyShifts} shifts · ${data.onDutyNow} on duty now`}
              icon={<Users size={14} />}
            />
            <StatTile
              label="Players tracked"
              value={formatNumber(data.playersTracked)}
              detail={`${data.playersNew} first seen in this range`}
            />
            <StatTile
              label="Active BOLOs"
              value={formatNumber(data.bolosActive)}
              detail={`${data.bolosCleared} cleared in total`}
            />
            <StatTile
              label="Internal bans"
              value={formatNumber(data.internalBans)}
              detail="CAD-only records in this range"
            />
          </div>

          <div className="mb-3 grid gap-3 lg:grid-cols-2">
            <Panel>
              <PanelHeader
                title="Daily activity"
                description={`Calls, moderation and commands per day over the last ${data.rangeDays} days (local time).`}
                actions={<Badge tone="neutral">{data.rangeDays}d</Badge>}
              />
              <div className="p-4">
                <DailyChart data={data.daily} />
              </div>
            </Panel>
            <Panel>
              <PanelHeader
                title="Busiest hours"
                description="When activity happens in your local time zone. Useful for staffing decisions."
                actions={<BarChart3 size={14} className="text-faint" />}
              />
              <div className="p-4">
                <HourChart data={data.hours} />
              </div>
            </Panel>
          </div>

          <div className="grid gap-3 lg:grid-cols-2">
            <Panel>
              <PanelHeader
                title="Peak player counts"
                description="Highest recorded player count per day, taken from the app's own metric samples."
              />
              <DataTable<DailyStatView>
                columns={[
                  { key: "date", header: "Day", render: (row) => <span className="mono text-[11px] text-muted">{row.date}</span> },
                  {
                    key: "peak",
                    header: "Peak players",
                    align: "right",
                    render: (row) => <span className="text-[12px] text-ink">{row.peakPlayers}</span>,
                  },
                  {
                    key: "calls",
                    header: "Calls",
                    align: "right",
                    render: (row) => <span className="text-[12px] text-ink">{row.callsOpened}</span>,
                  },
                ]}
                rows={[...data.daily].reverse().slice(0, 14)}
                getKey={(row) => row.date}
                emptyTitle="No metric samples yet"
                emptyDescription="Player counts are recorded while the app is connected to the server."
                maxHeight="320px"
              />
            </Panel>
            <Panel>
              <PanelHeader
                title="Most active moderators"
                description={`Moderation records attributed to each staff member in the last ${data.rangeDays} days.`}
                actions={<Badge tone="neutral">Top {data.topModerators.length}</Badge>}
              />
              <DataTable<TopModeratorView>
                columns={[
                  { key: "name", header: "Moderator", render: (row) => <span className="text-[12px] text-ink">{row.name}</span> },
                  {
                    key: "actions",
                    header: "Actions",
                    align: "right",
                    render: (row) => <span className="text-[12px] text-ink">{row.actions}</span>,
                  },
                  {
                    key: "failed",
                    header: "Failed",
                    align: "right",
                    render: (row) =>
                      row.failed > 0 ? (
                        <Badge tone="danger">{row.failed}</Badge>
                      ) : (
                        <span className="text-[11px] text-faint">0</span>
                      ),
                  },
                ]}
                rows={data.topModerators}
                getKey={(row) => row.name}
                emptyTitle="No moderation activity"
                emptyDescription="Moderation records appear here once staff take action."
              />
            </Panel>
          </div>

          <p className="mt-3 text-[11px] leading-5 text-faint">
            Range starts {formatDateTime(data.since)}. Call durations are computed from the stored
            creation and close timestamps, so they reflect exactly the calls that were closed in this
            range. A blank chart means no matching records were stored, not that data is missing.
          </p>
        </>
      ) : null}
    </div>
  );
}
