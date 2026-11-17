import AppShell from "@/layouts/AppShell";
import AuthPage from "@/pages/Auth";
import { SessionProvider, useSession } from "@/state/session";
import { SnapshotProvider } from "@/state/snapshot";
import { ToastProvider } from "@/state/toast";

function Root() {
  const { loading, session } = useSession();

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center bg-canvas">
        <div className="flex flex-col items-center gap-3">
          <div className="flex h-9 w-9 items-center justify-center rounded-md border border-line-strong bg-surface-2 text-[12px] font-bold text-ink">
            AC
          </div>
          <p className="text-[11px] uppercase tracking-[0.16em] text-faint">Starting console</p>
        </div>
      </div>
    );
  }

  if (!session) {
    return <AuthPage />;
  }

  return (
    <SnapshotProvider>
      <AppShell />
    </SnapshotProvider>
  );
}

export default function App() {
  return (
    <ToastProvider>
      <SessionProvider>
        <Root />
      </SessionProvider>
    </ToastProvider>
  );
}
