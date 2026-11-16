import { useEffect, useState } from "react";
import { CheckCircle2, FlaskConical, KeyRound, ShieldCheck, Trash2, TriangleAlert } from "lucide-react";
import { api, errorMessage } from "@/lib/api";
import { formatDateTime, formatDuration } from "@/lib/format";
import { useSession } from "@/state/session";
import { useSnapshot } from "@/state/snapshot";
import { useAsync } from "@/hooks/useAsync";
import { useToast } from "@/state/toast";
import { Badge } from "@/components/ui/Badge";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Field";
import { ConfirmDialog } from "@/components/ui/Modal";
import { InlineError, PageHeader, TableSkeleton } from "@/components/ui/Feedback";
import { KeyValue, Panel, PanelBody, PanelHeader } from "@/components/ui/Panel";
import type { ApiConfigView, ConnectionTestView } from "@/types";

export default function ApiConfigPage() {
  const { token, has, refreshBootstrap } = useSession();
  const { refresh } = useSnapshot();
  const toast = useToast();
  const [baseUrl, setBaseUrl] = useState("https://api.erlc.gg");
  const [apiKey, setApiKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [result, setResult] = useState<ConnectionTestView | null>(null);
  const [clearOpen, setClearOpen] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const config = useAsync<ApiConfigView | null>(
    () => (token ? api.getApiConfig(token) : Promise.resolve(null)),
    [token],
    Boolean(token),
  );

  useEffect(() => {
    if (config.data) setBaseUrl(config.data.baseUrl || "https://api.erlc.gg");
  }, [config.data]);

  const canConfigure = has("api.configure");
  const configured = config.data?.configured ?? false;

  async function saveKey() {
    if (!token) return;
    setSaving(true);
    setError(null);
    try {
      const next = await api.saveApiKey(token, baseUrl.trim(), apiKey.trim());
      config.setData(next);
      setApiKey("");
      toast.success("API key saved", "Stored with Windows DPAPI, never in the database or logs.");
      await Promise.all([refreshBootstrap(), refresh(true)]);
    } catch (err) {
      const message = errorMessage(err);
      setError(message);
      toast.error("Could not save API key", message);
    } finally {
      setSaving(false);
    }
  }

  async function testConnection() {
    if (!token) return;
    setTesting(true);
    setResult(null);
    setError(null);
    try {
      const view = await api.testApiConnection(token, apiKey.trim() ? apiKey.trim() : null);
      setResult(view);
      if (view.ok) toast.success("Connection verified", view.serverName ?? view.message);
      else toast.error("Connection test failed", view.message);
    } catch (err) {
      const message = errorMessage(err);
      setError(message);
      toast.error("Connection test failed", message);
    } finally {
      setTesting(false);
    }
  }

  return (
    <div>
      <PageHeader
        title="API Configuration"
        description="Connect the application to your ER:LC private server. The server key is encrypted with Windows DPAPI for your Windows account and is never written to the database, logs or source control."
        actions={
          <Badge tone={configured ? "positive" : "warning"} dot>
            {configured ? "Key configured" : "Key missing"}
          </Badge>
        }
      />

      <div className="grid gap-4 xl:grid-cols-[minmax(0,1.15fr)_minmax(0,1fr)]">
        <Panel>
          <PanelHeader
            title="Server credentials"
            description="Create a server key in the ER:LC private server API settings, then paste it here."
          />
          <PanelBody className="space-y-3">
            {config.error ? <InlineError message={config.error} /> : null}
            {error ? <InlineError message={error} /> : null}
            {config.loading && !config.data ? (
              <TableSkeleton rows={3} columns={2} />
            ) : (
              <>
                <Input
                  label="API base URL"
                  value={baseUrl}
                  onChange={(event) => setBaseUrl(event.target.value)}
                  spellCheck={false}
                  disabled={!canConfigure}
                  hint="default https://api.erlc.gg"
                  className="mono"
                />
                <Input
                  label="Server key"
                  type="password"
                  value={apiKey}
                  onChange={(event) => setApiKey(event.target.value)}
                  placeholder={configured ? "Enter a new key to replace the stored one" : "Paste your ER:LC server key"}
                  autoComplete="off"
                  spellCheck={false}
                  disabled={!canConfigure}
                  hint={configured ? `stored key ends in ${config.data?.keyLast4 ?? "••••"}` : "required"}
                />
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    variant="primary"
                    size="sm"
                    onClick={saveKey}
                    loading={saving}
                    disabled={!canConfigure || apiKey.trim().length === 0 || baseUrl.trim().length === 0}
                    leadingIcon={<KeyRound size={13} />}
                  >
                    Save key
                  </Button>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={testConnection}
                    loading={testing}
                    disabled={!canConfigure}
                    leadingIcon={<FlaskConical size={13} />}
                  >
                    {apiKey.trim() ? "Test entered key" : "Test stored key"}
                  </Button>
                  {configured ? (
                    <Button
                      variant="danger"
                      size="sm"
                      onClick={() => setClearOpen(true)}
                      disabled={!canConfigure}
                      leadingIcon={<Trash2 size={13} />}
                    >
                      Remove key
                    </Button>
                  ) : null}
                </div>
                {!canConfigure ? (
                  <p className="text-[11px] text-faint">
                    Only a Server Owner with api.configure can change credentials.
                  </p>
                ) : null}
              </>
            )}
          </PanelBody>
        </Panel>

        <div className="space-y-4">
          <Panel>
            <PanelHeader title="Stored configuration" description="Metadata only — the key itself is never shown." />
            <PanelBody>
              {config.data ? (
                <div className="space-y-0.5">
                  <KeyValue label="Base URL" value={config.data.baseUrl} mono />
                  <KeyValue
                    label="Key"
                    value={config.data.configured ? `••••••••${config.data.keyLast4 ?? ""}` : "Not configured"}
                    mono
                  />
                  <KeyValue
                    label="Fingerprint"
                    value={config.data.keyFingerprint ? `${config.data.keyFingerprint.slice(0, 16)}…` : "—"}
                    mono
                  />
                  <KeyValue label="Storage" value={config.data.keyStorage} />
                  <KeyValue label="Verified" value={formatDateTime(config.data.lastVerifiedAt)} />
                  <KeyValue label="Server" value={config.data.serverName ?? "—"} />
                  <KeyValue label="Server ID" value={config.data.serverId ?? "—"} mono />
                </div>
              ) : (
                <p className="text-[12px] text-muted">Loading configuration…</p>
              )}
            </PanelBody>
          </Panel>

          <Panel>
            <PanelHeader title="Connection test" description="GET /v2/server with the supplied key." />
            <PanelBody>
              {result ? (
                <div className="space-y-2">
                  <div className="flex flex-wrap items-center gap-2">
                    <Badge tone={result.ok ? "positive" : "danger"} dot>
                      {result.ok ? "Reachable" : "Failed"}
                    </Badge>
                    <span className="text-[11px] text-faint">{formatDuration(result.latencyMs)}</span>
                    {result.ok && result.currentPlayers !== null ? (
                      <span className="text-[11px] text-muted">
                        {result.currentPlayers}/{result.maxPlayers ?? "?"} players
                      </span>
                    ) : null}
                  </div>
                  <p className="break-words text-[12px] leading-5 text-ink">{result.message}</p>
                  {result.serverName ? (
                    <p className="text-[11px] text-muted">Server: {result.serverName}</p>
                  ) : null}
                  {result.rateLimit ? (
                    <div className="rounded border border-line bg-surface-2 px-2.5 py-2 text-[10px] text-faint">
                      <span className="mono">
                        Rate limit {result.rateLimit.remaining ?? "?"}/{result.rateLimit.limit ?? "?"}
                      </span>
                      {result.rateLimit.bucket ? ` · bucket ${result.rateLimit.bucket}` : ""}
                    </div>
                  ) : null}
                </div>
              ) : (
                <div className="flex items-start gap-2 text-[12px] leading-5 text-muted">
                  <CheckCircle2 size={14} className="mt-0.5 shrink-0 text-faint" />
                  Run a test to verify the key, reachability and the API's rate-limit headers. Testing
                  uses the entered key when present, otherwise the stored one.
                </div>
              )}
            </PanelBody>
          </Panel>
        </div>
      </div>

      <div className="mt-4 grid gap-4 md:grid-cols-2">
        <Panel>
          <PanelHeader title="Security properties" />
          <PanelBody className="space-y-1.5 text-[11px] leading-5 text-muted">
            <p className="flex items-start gap-2">
              <ShieldCheck size={13} className="mt-0.5 shrink-0 text-faint" />
              Windows builds encrypt the key with DPAPI, bound to your Windows user profile. Other
              users on the machine cannot read it.
            </p>
            <p className="flex items-start gap-2">
              <ShieldCheck size={13} className="mt-0.5 shrink-0 text-faint" />
              The key is never returned to the frontend; only the last four characters and a SHA-256
              fingerprint are displayed.
            </p>
            <p className="flex items-start gap-2">
              <ShieldCheck size={13} className="mt-0.5 shrink-0 text-faint" />
              All HTTP requests, retries (none for commands) and rate limiting happen in Rust.
            </p>
          </PanelBody>
        </Panel>

        <Panel>
          <PanelHeader title="API version and limits" />
          <PanelBody className="space-y-1.5 text-[11px] leading-5 text-muted">
            <p className="flex items-start gap-2">
              <TriangleAlert size={13} className="mt-0.5 shrink-0 text-faint" />
              Live data uses <span className="mono text-ink">GET /v2/server</span>; commands use{" "}
              <span className="mono text-ink">POST /v2/server/command</span>. Bans are read from{" "}
              <span className="mono text-ink">GET /v1/server/bans</span>.
            </p>
            <p>
              The API allows one command every 5 seconds. CAD enforces this locally and never retries
              a command that might have executed.
            </p>
            <p>
              Logs, locations and vehicle data only appear when the key carries the matching query
              flags and the server reports them.
            </p>
          </PanelBody>
        </Panel>
      </div>

      <ConfirmDialog
        open={clearOpen}
        onClose={() => setClearOpen(false)}
        title="Remove the stored API key"
        description="Live data and command execution stop working until a new key is saved. The ER:LC server itself is unaffected."
        confirmLabel="Remove key"
        destructive
        loading={saving}
        onConfirm={async () => {
          if (!token) return;
          setSaving(true);
          try {
            await api.clearApiKey(token);
            const next = await api.getApiConfig(token);
            config.setData(next);
            setClearOpen(false);
            toast.success("API key removed");
            await Promise.all([refreshBootstrap(), refresh(true)]);
          } catch (err) {
            toast.error("Could not remove key", errorMessage(err));
          } finally {
            setSaving(false);
          }
        }}
      />
    </div>
  );
}
