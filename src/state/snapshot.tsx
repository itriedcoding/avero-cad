import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { api, errorMessage } from "@/lib/api";
import { useSession } from "@/state/session";
import { useToast } from "@/state/toast";
import type { SettingsUpdate, SettingsView, Snapshot } from "@/types";

interface SnapshotContextValue {
  snapshot: Snapshot | null;
  settings: SettingsView | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  lastSync: number | null;
  refresh: (force?: boolean) => Promise<void>;
  reloadSettings: () => Promise<SettingsView | null>;
  saveSettings: (update: SettingsUpdate) => Promise<void>;
}

const SnapshotContext = createContext<SnapshotContextValue | null>(null);

export function SnapshotProvider({ children }: { children: ReactNode }) {
  const { token, has, session } = useSession();
  const toast = useToast();
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [settings, setSettings] = useState<SettingsView | null>(null);
  const [loading, setLoading] = useState(false);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const seenNotifications = useRef<Set<string>>(new Set());
  const connectionState = useRef<string | null>(null);
  const inFlight = useRef(false);

  const loadSettings = useCallback(async (): Promise<SettingsView | null> => {
    if (!token) return null;
    try {
      const next = await api.getSettings(token);
      setSettings(next);
      return next;
    } catch {
      /* settings are optional for rendering; keep the last known values */
      return null;
    }
  }, [token]);

  const refresh = useCallback(
    async (force = false) => {
      if (!token || !has("dashboard.view") || inFlight.current) return;
      inFlight.current = true;
      if (!snapshot) setLoading(true);
      setRefreshing(true);
      try {
        const next = await api.snapshot(token, force);
        setSnapshot(next);
        setError(null);

        const fresh = next.connection.state === "connected";
        if (fresh) {
          for (const note of next.notifications) {
            if (seenNotifications.current.has(note.id)) continue;
            seenNotifications.current.add(note.id);
            if (note.kind === "emergency" && (settings?.notifyEmergencyCalls ?? true)) {
              toast.push({ tone: "warning", title: note.title, message: note.detail });
            } else if (note.kind === "modcall" && (settings?.notifyModCalls ?? true)) {
              toast.push({ tone: "info", title: note.title, message: note.detail });
            }
          }
          if (seenNotifications.current.size > 400) {
            seenNotifications.current = new Set(next.notifications.map((note) => note.id));
          }
        }
        if (connectionState.current !== next.connection.state) {
          if (
            next.connection.state === "error" &&
            connectionState.current === "connected" &&
            (settings?.notifyConnection ?? true)
          ) {
            toast.error("Lost connection to ER:LC", next.connection.detail ?? undefined);
          }
          connectionState.current = next.connection.state;
        }
      } catch (err) {
        const message = errorMessage(err);
        setError(message);
      } finally {
        inFlight.current = false;
        setLoading(false);
        setRefreshing(false);
      }
    },
    [token, has, snapshot, toast, settings],
  );

  // Initial load honours the auto-connect preference; the header refresh always works.
  useEffect(() => {
    if (!token || !session) {
      setSnapshot(null);
      return;
    }
    void (async () => {
      const loaded = await loadSettings();
      if (loaded?.autoConnect ?? true) await refresh(false);
    })();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, session?.token]);

  useEffect(() => {
    if (!token || !session) return;
    const intervalMs = (settings?.refreshIntervalSeconds ?? 15) * 1000;
    const interval = window.setInterval(() => {
      if (document.visibilityState === "hidden") return;
      void refresh(false);
    }, intervalMs);
    return () => window.clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [token, session?.token, settings?.refreshIntervalSeconds]);

  const saveSettings = useCallback(
    async (update: SettingsUpdate) => {
      if (!token) return;
      const next = await api.updateSettings(token, update);
      setSettings(next);
    },
    [token],
  );

  const value = useMemo<SnapshotContextValue>(
    () => ({
      snapshot,
      settings,
      loading,
      refreshing,
      error,
      lastSync: snapshot?.fetchedAt || null,
      refresh,
      reloadSettings: loadSettings,
      saveSettings,
    }),
    [snapshot, settings, loading, refreshing, error, refresh, loadSettings, saveSettings],
  );

  return <SnapshotContext.Provider value={value}>{children}</SnapshotContext.Provider>;
}

export function useSnapshot(): SnapshotContextValue {
  const context = useContext(SnapshotContext);
  if (!context) throw new Error("useSnapshot must be used inside SnapshotProvider");
  return context;
}

/** For components that may render outside the provider (e.g. shared UI). */
export function useSnapshotOptional(): SnapshotContextValue | null {
  return useContext(SnapshotContext);
}
