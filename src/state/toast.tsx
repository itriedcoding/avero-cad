import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import { AlertTriangle, CheckCircle2, Info, X } from "lucide-react";

export type ToastTone = "success" | "error" | "info" | "warning";

interface ToastItem {
  id: number;
  tone: ToastTone;
  title: string;
  message?: string;
}

interface ToastApi {
  push: (toast: { tone: ToastTone; title: string; message?: string }) => void;
  success: (title: string, message?: string) => void;
  error: (title: string, message?: string) => void;
  info: (title: string, message?: string) => void;
  warning: (title: string, message?: string) => void;
}

const ToastContext = createContext<ToastApi | null>(null);

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const counter = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((current) => current.filter((toast) => toast.id !== id));
  }, []);

  const push = useCallback<ToastApi["push"]>(
    ({ tone, title, message }) => {
      counter.current += 1;
      const id = counter.current;
      setToasts((current) => [...current.slice(-4), { id, tone, title, message }]);
      window.setTimeout(() => dismiss(id), tone === "error" ? 9000 : 6000);
    },
    [dismiss],
  );

  const api = useMemo<ToastApi>(
    () => ({
      push,
      success: (title, message) => push({ tone: "success", title, message }),
      error: (title, message) => push({ tone: "error", title, message }),
      info: (title, message) => push({ tone: "info", title, message }),
      warning: (title, message) => push({ tone: "warning", title, message }),
    }),
    [push],
  );

  return (
    <ToastContext.Provider value={api}>
      {children}
      <div className="pointer-events-none fixed bottom-4 right-4 z-50 flex w-[min(24rem,calc(100vw-2rem))] flex-col gap-2">
        {toasts.map((toast) => (
          <div
            key={toast.id}
            role="status"
            className="glass pointer-events-auto animate-fade-in rounded-xl"
          >
            <div className="flex items-start gap-3 p-3">
              <span className="mt-0.5 shrink-0 text-accent-soft">
                {toast.tone === "success" ? (
                  <CheckCircle2 size={16} />
                ) : toast.tone === "error" ? (
                  <AlertTriangle size={16} className="text-danger" />
                ) : toast.tone === "warning" ? (
                  <AlertTriangle size={16} className="text-warn" />
                ) : (
                  <Info size={16} />
                )}
              </span>
              <div className="min-w-0 flex-1">
                <p className="text-sm font-medium leading-5 text-ink">{toast.title}</p>
                {toast.message ? (
                  <p className="mt-0.5 break-words text-xs leading-5 text-muted">{toast.message}</p>
                ) : null}
              </div>
              <button
                type="button"
                onClick={() => dismiss(toast.id)}
                className="rounded p-1 text-faint transition-colors hover:text-ink"
                aria-label="Dismiss notification"
              >
                <X size={14} />
              </button>
            </div>
            <div
              className={
                toast.tone === "error"
                  ? "mx-3 mb-2.5 h-[2px] rounded-full bg-danger/70"
                  : toast.tone === "warning"
                    ? "mx-3 mb-2.5 h-[2px] rounded-full bg-warn/60"
                    : "mx-3 mb-2.5 h-[2px] rounded-full bg-accent/45"
              }
            />
          </div>
        ))}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast(): ToastApi {
  const context = useContext(ToastContext);
  if (!context) throw new Error("useToast must be used inside ToastProvider");
  return context;
}
