import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { api, errorMessage } from "@/lib/api";
import type { BootstrapView, SessionView } from "@/types";

const TOKEN_KEY = "erlc-cad.session-token";

interface SessionContextValue {
  bootstrap: BootstrapView | null;
  session: SessionView | null;
  token: string | null;
  permissions: Set<string>;
  isOwner: boolean;
  loading: boolean;
  error: string | null;
  has: (permission: string) => boolean;
  login: (username: string, password: string) => Promise<void>;
  setupOwner: (username: string, displayName: string, password: string) => Promise<void>;
  signOut: () => Promise<void>;
  refreshSession: () => Promise<void>;
  refreshBootstrap: () => Promise<void>;
}

const SessionContext = createContext<SessionContextValue | null>(null);

function readToken(): string | null {
  try {
    return window.sessionStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

function storeToken(token: string | null) {
  try {
    if (token) window.sessionStorage.setItem(TOKEN_KEY, token);
    else window.sessionStorage.removeItem(TOKEN_KEY);
  } catch {
    /* session storage can be unavailable; the in-memory token still works */
  }
}

export function SessionProvider({ children }: { children: ReactNode }) {
  const [bootstrap, setBootstrap] = useState<BootstrapView | null>(null);
  const [session, setSession] = useState<SessionView | null>(null);
  const [token, setToken] = useState<string | null>(() => readToken());
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refreshBootstrap = useCallback(async () => {
    try {
      const info = await api.bootstrap();
      setBootstrap(info);
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  const refreshSession = useCallback(async () => {
    const existing = token ?? readToken();
    if (!existing) {
      setSession(null);
      return;
    }
    try {
      const info = await api.sessionInfo(existing);
      setSession(info);
      setToken(info.token);
      storeToken(info.token);
    } catch {
      setSession(null);
      setToken(null);
      storeToken(null);
    }
  }, [token]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      await refreshBootstrap();
      const existing = readToken();
      if (existing && !cancelled) {
        try {
          const info = await api.sessionInfo(existing);
          if (!cancelled) {
            setSession(info);
            setToken(info.token);
          }
        } catch {
          if (!cancelled) {
            setToken(null);
            storeToken(null);
          }
        }
      }
      if (!cancelled) setLoading(false);
    })();
    return () => {
      cancelled = true;
    };
    // Runs once on mount.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const adopt = useCallback((info: SessionView) => {
    setSession(info);
    setToken(info.token);
    storeToken(info.token);
    setError(null);
  }, []);

  const login = useCallback(
    async (username: string, password: string) => {
      const info = await api.login(username, password);
      adopt(info);
      await refreshBootstrap();
    },
    [adopt, refreshBootstrap],
  );

  const setupOwner = useCallback(
    async (username: string, displayName: string, password: string) => {
      const info = await api.setupOwner(username, displayName, password);
      adopt(info);
      await refreshBootstrap();
    },
    [adopt, refreshBootstrap],
  );

  const signOut = useCallback(async () => {
    const current = token;
    setSession(null);
    setToken(null);
    storeToken(null);
    if (current) {
      try {
        await api.logout(current);
      } catch {
        /* the session is already gone locally */
      }
    }
  }, [token]);

  const permissions = useMemo(() => new Set(session?.permissions ?? []), [session]);
  const isOwner = session?.isOwner ?? false;

  const has = useCallback(
    (permission: string) => isOwner || permissions.has(permission),
    [isOwner, permissions],
  );

  // Detect expired sessions while the app is open.
  useEffect(() => {
    if (!token) return;
    const interval = window.setInterval(async () => {
      try {
        await api.sessionInfo(token);
      } catch {
        setSession(null);
        setToken(null);
        storeToken(null);
      }
    }, 5 * 60 * 1000);
    return () => window.clearInterval(interval);
  }, [token]);

  const value = useMemo<SessionContextValue>(
    () => ({
      bootstrap,
      session,
      token,
      permissions,
      isOwner,
      loading,
      error,
      has,
      login,
      setupOwner,
      signOut,
      refreshSession,
      refreshBootstrap,
    }),
    [
      bootstrap,
      session,
      token,
      permissions,
      isOwner,
      loading,
      error,
      has,
      login,
      setupOwner,
      signOut,
      refreshSession,
      refreshBootstrap,
    ],
  );

  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
}

export function useSession(): SessionContextValue {
  const context = useContext(SessionContext);
  if (!context) throw new Error("useSession must be used inside SessionProvider");
  return context;
}

/** Token for IPC calls; throws a helpful error when signed out. */
export function useToken(): string {
  const { token } = useSession();
  return token ?? "";
}
