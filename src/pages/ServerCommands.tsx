import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { CornerDownLeft, Pencil, Plus, ShieldAlert, Terminal, Trash2 } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, formatDuration, formatTime } from "@/lib/format";
import { permissionLabel } from "@/lib/status";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button, IconButton } from "@/components/ui/Button";
import { Checkbox, Input, Select, Textarea } from "@/components/ui/Field";
import { ConfirmDialog, Modal } from "@/components/ui/Modal";
import { InlineError, PageHeader, Skeleton } from "@/components/ui/Feedback";
import { Panel, PanelBody, PanelHeader, SectionTitle } from "@/components/ui/Panel";
import { DataTable } from "@/components/ui/Table";
import type { CommandHistoryView, CommandPolicyView, CommandResultView } from "@/types";

const COMMAND_PATTERN = /^:([a-zA-Z0-9]{1,24})(?:\s+([\s\S]*))?$/;

function parseCommand(raw: string): { name: string; args: string } | null {
  const match = COMMAND_PATTERN.exec(raw.trim());
  if (!match) return null;
  return { name: `:${match[1]!.toLowerCase()}`, args: (match[2] ?? "").trim() };
}

export default function ServerCommandsPage() {
  const { token, has } = useSession();
  const { settings, snapshot } = useSnapshot();
  const toast = useToast();
  const [input, setInput] = useState("");
  const [confirmedCommand, setConfirmedCommand] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const [lastResult, setLastResult] = useState<CommandResultView | null>(null);
  const [policyOpen, setPolicyOpen] = useState(false);
  const [policyDraft, setPolicyDraft] = useState({
    command: ":",
    minPermission: "commands.execute",
    destructive: false,
    enabled: true,
    description: "",
  });
  const [savingPolicy, setSavingPolicy] = useState(false);
  const outputRef = useRef<HTMLDivElement>(null);

  const policies = useAsync<CommandPolicyView[]>(
    () => (token ? api.commandPolicies(token) : Promise.resolve([])),
    [token],
    Boolean(token) && has("commands.view"),
  );
  const history = useAsync<CommandHistoryView[]>(
    () => (token ? api.commandHistory(token, 100) : Promise.resolve([])),
    [token],
    Boolean(token) && has("commands.view"),
  );

  const parsed = useMemo(() => parseCommand(input), [input]);
  const policy = parsed ? (policies.data ?? []).find((entry) => entry.command === parsed.name) ?? null : null;
  const requiresConfirmation = Boolean(policy?.destructive) && (settings?.confirmDestructive ?? true);

  useEffect(() => {
    outputRef.current?.scrollTo({ top: outputRef.current.scrollHeight });
  }, [history.data]);

  async function send(raw: string) {
    if (!token) return;
    setSending(true);
    try {
      const result = await api.executeCommand(token, raw, true);
      setLastResult(result);
      toast.success("Command executed", result.message);
      setInput("");
      await history.reload();
    } catch (err) {
      const message = errorMessage(err);
      setLastResult({
        status: "failed",
        message,
        commandId: null,
        httpStatus: null,
        durationMs: 0,
        executedAt: Date.now(),
      });
      toast.error("Command failed", message);
      await history.reload();
    } finally {
      setSending(false);
    }
  }

  function submit(event: FormEvent) {
    event.preventDefault();
    const raw = input.trim();
    if (!raw) return;
    if (!parseCommand(raw)) {
      toast.error("Invalid command", "ER:LC commands start with ':' followed by the command name.");
      return;
    }
    if (!policy) {
      toast.error("Command not authorized", "A Server Owner must add this command to Command Policies.");
      return;
    }
    if (requiresConfirmation) {
      setConfirmedCommand(raw);
      return;
    }
    void send(raw);
  }

  async function savePolicy() {
    if (!token) return;
    setSavingPolicy(true);
    try {
      await api.upsertCommandPolicy(
        token,
        policyDraft.command.trim(),
        policyDraft.minPermission,
        policyDraft.destructive,
        policyDraft.enabled,
        policyDraft.description,
      );
      toast.success("Command policy saved");
      setPolicyOpen(false);
      await policies.reload();
    } catch (err) {
      toast.error("Could not save policy", errorMessage(err));
    } finally {
      setSavingPolicy(false);
    }
  }

  async function removePolicy(command: string) {
    if (!token) return;
    try {
      await api.deleteCommandPolicy(token, command);
      toast.success("Command policy removed");
      await policies.reload();
    } catch (err) {
      toast.error("Could not remove policy", errorMessage(err));
    }
  }

  const permissionOptions = [
    "commands.execute",
    "moderation.kick",
    "moderation.ban",
    "moderation.unban",
    "moderation.internal",
    "dispatch.update",
  ];

  return (
    <div>
      <PageHeader
        title="Server Commands"
        description="Execute ER:LC server commands as virtual server management. Commands are validated, permission-checked, confirmed when destructive, and never retried automatically."
        actions={
          <Badge tone={snapshot?.connection.state === "connected" ? "positive" : "muted"} dot>
            {snapshot?.connection.state === "connected" ? "Server online" : "Server unavailable"}
          </Badge>
        }
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]">
        <Panel className="flex min-h-[540px] flex-col">
          <PanelHeader
            title="Command terminal"
            description="Only commands authorized in Command Policies are accepted. Windows commands are rejected by the backend."
            actions={
              <Badge tone="neutral">
                <Terminal size={11} /> API command
              </Badge>
            }
          />
          <div
            ref={outputRef}
            className="mono min-h-0 flex-1 overflow-y-auto border-b border-line bg-[#08080a] px-3 py-2 text-[11px] leading-5"
          >
            {history.loading && !history.data ? (
              <div className="space-y-2 py-2">
                <Skeleton className="h-3 w-2/3" />
                <Skeleton className="h-3 w-1/2" />
                <Skeleton className="h-3 w-3/4" />
              </div>
            ) : (history.data ?? []).length === 0 ? (
              <p className="py-6 text-center text-[11px] text-faint">
                No commands executed yet. Try{" "}
                <span className="text-muted">:h Welcome to the server</span>.
              </p>
            ) : (
              <ul className="space-y-1.5">
                {(history.data ?? [])
                  .slice()
                  .reverse()
                  .map((entry) => (
                    <li key={entry.id} className="flex gap-2">
                      <span className="shrink-0 text-faint">{formatTime(entry.createdAt)}</span>
                      <span
                        className={
                          entry.status === "success"
                            ? "shrink-0 text-white"
                            : entry.status === "rejected"
                              ? "shrink-0 text-faint"
                              : "shrink-0 text-danger"
                        }
                      >
                        {entry.status === "success" ? "OK " : entry.status === "rejected" ? "DEN" : "ERR"}
                      </span>
                      <span className="min-w-0 flex-1">
                        <span className="block break-all text-ink">{entry.command}</span>
                        <span className="block break-words text-faint">
                          {entry.response || "—"} · {entry.actorName} · {formatDuration(entry.durationMs)}
                        </span>
                      </span>
                    </li>
                  ))}
              </ul>
            )}
          </div>

          <form onSubmit={submit} className="space-y-2 p-3">
            <div className="flex gap-2">
              <div className="relative flex-1">
                <span className="mono pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-[12px] text-faint">
                  &gt;
                </span>
                <Input
                  aria-label="Command"
                  value={input}
                  onChange={(event) => setInput(event.target.value)}
                  placeholder=":h Welcome to the server"
                  className="mono pl-7"
                  spellCheck={false}
                  autoComplete="off"
                  disabled={!has("commands.execute")}
                />
              </div>
              <Button
                type="submit"
                variant="primary"
                loading={sending}
                disabled={input.trim().length === 0 || !has("commands.execute")}
                leadingIcon={<CornerDownLeft size={13} />}
              >
                Send
              </Button>
            </div>
            <div className="flex flex-wrap items-center gap-1.5">
              {(policies.data ?? []).filter((entry) => entry.enabled).map((entry) => (
                <button
                  key={entry.command}
                  type="button"
                  onClick={() => setInput(`${entry.command} `)}
                  className="mono rounded border border-line bg-surface-2 px-1.5 py-0.5 text-[10px] text-muted transition-colors hover:border-line-strong hover:text-ink"
                  title={entry.description || entry.command}
                >
                  {entry.command}
                </button>
              ))}
            </div>
            {parsed && !policy ? (
              <p className="text-[11px] text-danger">
                {parsed.name} is not authorized. Add it to Command Policies first.
              </p>
            ) : null}
            {requiresConfirmation ? (
              <p className="flex items-center gap-1.5 text-[11px] text-muted">
                <ShieldAlert size={12} /> Destructive command — confirmation is required before it is
                sent.
              </p>
            ) : policy?.destructive ? (
              <p className="flex items-center gap-1.5 text-[11px] text-muted">
                <ShieldAlert size={12} /> Destructive command — extra confirmation is disabled in
                Settings.
              </p>
            ) : null}
            {!has("commands.execute") ? (
              <p className="text-[11px] text-faint">Your role can view the console but not execute commands.</p>
            ) : null}
          </form>
        </Panel>

        <div className="space-y-4">
          {lastResult ? (
            <Panel>
              <PanelHeader
                title="Last response"
                description={`${formatDateTime(lastResult.executedAt)} · ${formatDuration(lastResult.durationMs)}`}
              />
              <PanelBody className="space-y-2">
                <div className="flex flex-wrap items-center gap-2">
                  <Badge tone={lastResult.status === "success" ? "positive" : lastResult.status === "rejected" ? "muted" : "danger"}>
                    {lastResult.status}
                  </Badge>
                  {lastResult.httpStatus ? <Badge tone="neutral">HTTP {lastResult.httpStatus}</Badge> : null}
                  {lastResult.commandId ? (
                    <span className="mono text-[10px] text-faint">command {lastResult.commandId}</span>
                  ) : null}
                </div>
                <p className="break-words text-[12px] leading-5 text-ink">{lastResult.message}</p>
              </PanelBody>
            </Panel>
          ) : null}

          <Panel>
            <PanelHeader
              title="Command policies"
              description="Owner-managed authorization. Each command names the minimum permission it requires."
              actions={
                has("commands.policies.manage") ? (
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => {
                      setPolicyDraft({
                        command: ":",
                        minPermission: "commands.execute",
                        destructive: false,
                        enabled: true,
                        description: "",
                      });
                      setPolicyOpen(true);
                    }}
                    leadingIcon={<Plus size={13} />}
                  >
                    Policy
                  </Button>
                ) : null
              }
            />
            {policies.error ? (
              <div className="p-3">
                <InlineError message={policies.error} />
              </div>
            ) : (
              <DataTable<CommandPolicyView>
                columns={[
                  {
                    key: "command",
                    header: "Command",
                    render: (row) => (
                      <span className="mono text-[12px] text-ink">{row.command}</span>
                    ),
                  },
                  {
                    key: "permission",
                    header: "Minimum permission",
                    render: (row) => (
                      <span className="mono text-[10px] text-muted">
                        {permissionLabel(row.minPermission)}
                      </span>
                    ),
                  },
                  {
                    key: "flags",
                    header: "Flags",
                    render: (row) => (
                      <span className="flex flex-wrap gap-1">
                        {row.destructive ? <Badge tone="danger">confirm</Badge> : null}
                        {!row.enabled ? <Badge tone="muted">disabled</Badge> : null}
                      </span>
                    ),
                  },
                  {
                    key: "actions",
                    header: "",
                    align: "right",
                    render: (row) =>
                      has("commands.policies.manage") ? (
                        <span className="flex justify-end gap-1">
                          <IconButton
                            label={`Edit ${row.command}`}
                            onClick={() => {
                              setPolicyDraft({
                                command: row.command,
                                minPermission: row.minPermission,
                                destructive: row.destructive,
                                enabled: row.enabled,
                                description: row.description,
                              });
                              setPolicyOpen(true);
                            }}
                          >
                            <Pencil size={12} />
                          </IconButton>
                          <IconButton
                            label={`Delete ${row.command}`}
                            onClick={() => void removePolicy(row.command)}
                          >
                            <Trash2 size={12} />
                          </IconButton>
                        </span>
                      ) : (
                        <span className="text-[10px] text-faint">—</span>
                      ),
                  },
                ]}
                rows={policies.data ?? []}
                getKey={(row) => row.command}
                emptyTitle="No command policies"
                emptyDescription="No ER:LC command is authorized yet. A Server Owner must add policies before commands can run."
                maxHeight="260px"
              />
            )}
          </Panel>

          <Panel>
            <PanelHeader title="How command safety works" />
            <PanelBody className="space-y-1.5 text-[11px] leading-5 text-muted">
              <p>· Permissions are validated in Rust before any HTTP request is made.</p>
              <p>· Commands must be `:` in-game commands; Windows shell commands are rejected.</p>
              <p>· The API's documented one-command-per-5-seconds limit is enforced locally.</p>
              <p>· A failed command is never retried, so nothing can execute twice.</p>
            </PanelBody>
          </Panel>
        </div>
      </div>

      <SectionTitle>Execution history</SectionTitle>
      <Panel>
        <DataTable<CommandHistoryView>
          columns={[
            {
              key: "command",
              header: "Command",
              render: (row) => <span className="mono text-[11px] text-ink">{row.command}</span>,
            },
            {
              key: "status",
              header: "Status",
              render: (row) => (
                <Badge tone={row.status === "success" ? "positive" : row.status === "rejected" ? "muted" : "danger"}>
                  {row.status}
                </Badge>
              ),
            },
            {
              key: "response",
              header: "Response",
              render: (row) => <span className="text-[11px] text-muted">{row.response}</span>,
            },
            {
              key: "actor",
              header: "Staff",
              render: (row) => <span className="text-[11px] text-muted">{row.actorName}</span>,
            },
            {
              key: "when",
              header: "When",
              align: "right",
              render: (row) => (
                <span className="text-[10px] text-faint">{formatDateTime(row.createdAt)}</span>
              ),
            },
          ]}
          rows={history.data ?? []}
          getKey={(row) => row.id}
          emptyTitle="No executions recorded"
          emptyDescription="Commands executed from this installation appear here with their API response."
          maxHeight="320px"
        />
      </Panel>

      <ConfirmDialog
        open={confirmedCommand !== null}
        onClose={() => setConfirmedCommand(null)}
        onConfirm={() => {
          const raw = confirmedCommand;
          setConfirmedCommand(null);
          if (raw) void send(raw);
        }}
        title="Confirm destructive command"
        description={`"${confirmedCommand ?? ""}" will be sent to your ER:LC server immediately. It cannot be undone from CAD and will not be retried.`}
        confirmLabel="Execute command"
        destructive
        loading={sending}
      />

      <Modal
        open={policyOpen}
        onClose={() => setPolicyOpen(false)}
        title="Command policy"
        description="Authorize an in-game command and choose the minimum permission required to run it."
        footer={
          <>
            <Button variant="ghost" onClick={() => setPolicyOpen(false)} disabled={savingPolicy}>
              Cancel
            </Button>
            <Button variant="primary" onClick={savePolicy} loading={savingPolicy}>
              Save policy
            </Button>
          </>
        }
      >
        <div className="space-y-3">
          <Input
            label="Command"
            value={policyDraft.command}
            placeholder=":kick"
            onChange={(event) => setPolicyDraft({ ...policyDraft, command: event.target.value })}
            spellCheck={false}
            className="mono"
          />
          <Select
            label="Minimum permission"
            value={policyDraft.minPermission}
            onChange={(event) => setPolicyDraft({ ...policyDraft, minPermission: event.target.value })}
          >
            {permissionOptions.map((permission) => (
              <option key={permission} value={permission}>
                {permissionLabel(permission)}
              </option>
            ))}
          </Select>
          <Textarea
            label="Description"
            rows={2}
            value={policyDraft.description}
            onChange={(event) => setPolicyDraft({ ...policyDraft, description: event.target.value })}
          />
          <div className="space-y-1 rounded border border-line bg-surface-2 px-3 py-1">
            <Checkbox
              checked={policyDraft.destructive}
              onChange={(next) => setPolicyDraft({ ...policyDraft, destructive: next })}
              label="Destructive — require explicit confirmation and a target argument"
            />
            <Checkbox
              checked={policyDraft.enabled}
              onChange={(next) => setPolicyDraft({ ...policyDraft, enabled: next })}
              label="Enabled"
            />
          </div>
        </div>
      </Modal>
    </div>
  );
}
