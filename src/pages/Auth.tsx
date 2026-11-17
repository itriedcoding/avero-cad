import { useState, type FormEvent } from "react";
import { ArrowRight, Shield } from "lucide-react";
import { errorMessage } from "@/lib/api";
import { useSession } from "@/state/session";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Field";
import { InlineError } from "@/components/ui/Feedback";

export default function AuthPage() {
  const { bootstrap, login, setupOwner, loading } = useSession();
  const needsSetup = bootstrap?.needsSetup ?? false;
  const [username, setUsername] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    if (needsSetup && password !== confirmPassword) {
      setError("The passwords do not match.");
      return;
    }
    setSubmitting(true);
    try {
      if (needsSetup) {
        await setupOwner(username.trim(), displayName.trim(), password);
      } else {
        await login(username.trim(), password);
      }
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="flex h-full min-h-0">
      <div className="chrome hidden w-[46%] flex-col justify-between border-r border-line/80 p-8 lg:flex">
        <div className="flex items-center gap-2.5">
          <div className="flex h-8 w-8 items-center justify-center rounded-[8px] border border-accent/50 bg-accent/15 text-[11px] font-bold text-accent-soft shadow-[inset_0_1px_0_rgb(255_255_255/0.16),0_0_20px_-6px_rgb(255_42_58/0.9)]">
            AC
          </div>
          <div>
            <p className="text-[13px] font-bold tracking-tight text-ink">Avero CAD</p>
            <p className="text-[10px] uppercase tracking-[0.14em] text-faint">Windows desktop console</p>
          </div>
        </div>

        <div className="max-w-sm">
          <h1 className="text-2xl font-bold leading-tight tracking-tight text-ink">
            Dispatch, moderation and server control for your ER:LC private server.
          </h1>
          <p className="mt-3 text-[13px] leading-6 text-muted">
            Live roster and vehicle data, player records with Roblox avatars, a working CAD dispatch
            board and an audited command console — all backed by the official ER:LC Private Server
            API.
          </p>
          <div className="mt-6 space-y-2.5 border-t border-line pt-5">
            <p className="flex items-center gap-2 text-xs text-muted">
              <Shield size={13} className="text-faint" />
              Server keys are encrypted with Windows DPAPI and never leave the Rust backend.
            </p>
            <p className="text-xs leading-5 text-faint">
              Every command, moderation action and permission change is written to the audit log.
            </p>
          </div>
        </div>

        <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-[10px] text-faint">
          <span>Version {bootstrap?.appVersion ?? "—"}</span>
          <span>Platform {bootstrap?.platform ?? "—"}</span>
          <span className="truncate">{bootstrap?.apiBaseUrl ?? "https://api.erlc.gg"}</span>
        </div>
      </div>

      <div className="flex min-w-0 flex-1 items-center justify-center p-6">
        <div className="glass w-full max-w-sm rounded-2xl p-6">
          <div className="mb-6 lg:hidden">
            <div className="flex h-8 w-8 items-center justify-center rounded-[8px] border border-accent/50 bg-accent/15 text-[11px] font-bold text-accent-soft">
              AC
            </div>
          </div>
          <h2 className="text-lg font-bold tracking-tight text-ink">
            {needsSetup ? "Set up your server owner account" : "Sign in"}
          </h2>
          <p className="mt-1 text-[13px] leading-5 text-muted">
            {needsSetup
              ? "This installation has no staff accounts yet. Create the Server Owner account that will manage roles, permissions and the ER:LC server key."
              : "Staff accounts are local to this installation. Your role determines which modules you can use."}
          </p>

          <form onSubmit={onSubmit} className="mt-5 space-y-3.5">
            <Input
              label="Username"
              value={username}
              autoFocus
              autoComplete="username"
              spellCheck={false}
              placeholder={needsSetup ? "owner" : "your username"}
              onChange={(event) => setUsername(event.target.value)}
            />
            {needsSetup ? (
              <Input
                label="Display name"
                value={displayName}
                placeholder="e.g. Chief of Police"
                onChange={(event) => setDisplayName(event.target.value)}
              />
            ) : null}
            <Input
              label="Password"
              type="password"
              value={password}
              autoComplete={needsSetup ? "new-password" : "current-password"}
              placeholder={needsSetup ? "At least 10 characters" : "••••••••••"}
              onChange={(event) => setPassword(event.target.value)}
            />
            {needsSetup ? (
              <Input
                label="Confirm password"
                type="password"
                value={confirmPassword}
                autoComplete="new-password"
                onChange={(event) => setConfirmPassword(event.target.value)}
              />
            ) : null}

            {error ? <InlineError message={error} /> : null}

            <Button
              type="submit"
              variant="primary"
              className="w-full"
              loading={submitting || loading}
              leadingIcon={<ArrowRight size={14} />}
            >
              {needsSetup ? "Create owner account" : "Sign in"}
            </Button>
          </form>

          <div className="mt-5 border-t border-line/80 pt-4">
            <p className="text-[11px] leading-5 text-faint">
              Database: <span className="mono break-all">{bootstrap?.databasePath ?? "—"}</span>
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
