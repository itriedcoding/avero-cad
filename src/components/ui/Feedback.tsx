import type { ReactNode } from "react";
import { AlertTriangle, Inbox, RefreshCw } from "lucide-react";
import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/Button";
import { formatRelative } from "@/lib/format";

export function PageHeader({
  title,
  description,
  actions,
}: {
  title: string;
  description?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
      <div className="min-w-0">
        <h1 className="text-lg font-semibold tracking-tight text-ink">{title}</h1>
        {description ? (
          <p className="mt-1 max-w-3xl text-[13px] leading-5 text-muted">{description}</p>
        ) : null}
      </div>
      {actions ? <div className="flex flex-wrap items-center gap-2">{actions}</div> : null}
    </div>
  );
}

export function EmptyState({
  title,
  description,
  action,
  compact,
  icon,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
  compact?: boolean;
  icon?: ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex flex-col items-center justify-center text-center",
        compact ? "px-6 py-10" : "px-6 py-16",
      )}
    >
      <div className="mb-3 flex h-9 w-9 items-center justify-center rounded-md border border-line bg-surface-2 text-faint">
        {icon ?? <Inbox size={16} />}
      </div>
      <p className="text-[13px] font-medium text-ink">{title}</p>
      {description ? (
        <p className="mt-1 max-w-sm text-xs leading-5 text-muted">{description}</p>
      ) : null}
      {action ? <div className="mt-3">{action}</div> : null}
    </div>
  );
}

export function InlineError({ message, action }: { message: string; action?: ReactNode }) {
  return (
    <div className="flex items-start gap-2.5 rounded-md border border-danger/30 bg-danger/[0.06] px-3 py-2.5">
      <AlertTriangle size={14} className="mt-0.5 shrink-0 text-danger" />
      <div className="min-w-0 flex-1">
        <p className="break-words text-xs leading-5 text-ink">{message}</p>
      </div>
      {action}
    </div>
  );
}

export function Skeleton({ className }: { className?: string }) {
  return <div className={cn("animate-pulse rounded bg-surface-3", className)} />;
}

export function TableSkeleton({ rows = 6, columns = 4 }: { rows?: number; columns?: number }) {
  return (
    <div className="divide-y divide-line/70">
      {Array.from({ length: rows }).map((_, rowIndex) => (
        <div key={rowIndex} className="flex items-center gap-4 px-3 py-2.5">
          {Array.from({ length: columns }).map((__, columnIndex) => (
            <Skeleton
              key={columnIndex}
              className={cn("h-3", columnIndex === 0 ? "w-28" : "w-20", "flex-none")}
            />
          ))}
        </div>
      ))}
    </div>
  );
}

export function RefreshButton({
  onClick,
  refreshing,
  lastSync,
}: {
  onClick: () => void;
  refreshing: boolean;
  lastSync: number | null;
}) {
  return (
    <div className="flex items-center gap-2">
      {lastSync ? (
        <span className="hidden text-[11px] text-faint sm:inline">
          Synced {formatRelative(lastSync)}
        </span>
      ) : null}
      <Button
        size="sm"
        variant="secondary"
        onClick={onClick}
        loading={refreshing}
        leadingIcon={<RefreshCw size={13} />}
      >
        Refresh
      </Button>
    </div>
  );
}
