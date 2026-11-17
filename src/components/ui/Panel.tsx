import type { ReactNode } from "react";
import { cn } from "@/lib/cn";

export function Panel({
  children,
  className,
  as: Component = "section",
}: {
  children: ReactNode;
  className?: string;
  as?: "section" | "div" | "article";
}) {
  return <Component className={cn("panel", className)}>{children}</Component>;
}

export function PanelHeader({
  title,
  description,
  actions,
  compact,
}: {
  title: ReactNode;
  description?: ReactNode;
  actions?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div
      className={cn(
        "flex flex-wrap items-start justify-between gap-3 border-b border-line/80",
        compact ? "px-3.5 py-2.5" : "px-4 py-3",
      )}
    >
      <div className="min-w-0">
        <h2 className="text-[13px] font-semibold tracking-tight text-ink">{title}</h2>
        {description ? (
          <p className="mt-0.5 max-w-2xl text-xs leading-5 text-muted">{description}</p>
        ) : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-1.5">{actions}</div> : null}
    </div>
  );
}

export function PanelBody({
  children,
  className,
  padded = true,
}: {
  children: ReactNode;
  className?: string;
  padded?: boolean;
}) {
  return <div className={cn(padded && "p-4", className)}>{children}</div>;
}

export function StatTile({
  label,
  value,
  detail,
  icon,
  accent,
}: {
  label: string;
  value: ReactNode;
  detail?: ReactNode;
  icon?: ReactNode;
  accent?: boolean;
}) {
  return (
    <div className="panel flex flex-col justify-between gap-3 px-4 py-3.5">
      <div className="flex items-center justify-between gap-2">
        <span className="text-[11px] font-bold uppercase tracking-[0.1em] text-faint">
          {label}
        </span>
        {icon ? <span className="text-accent/70">{icon}</span> : null}
      </div>
      <div>
        <p
          className={cn(
            "text-2xl font-bold leading-none tracking-tight",
            accent ? "text-accent-soft" : "text-ink",
          )}
        >
          {value}
        </p>
        {detail ? <p className="mt-1.5 text-xs text-muted">{detail}</p> : null}
      </div>
    </div>
  );
}

export function KeyValue({
  label,
  value,
  mono,
  className,
}: {
  label: string;
  value: ReactNode;
  mono?: boolean;
  className?: string;
}) {
  return (
    <div className={cn("flex items-baseline justify-between gap-4 py-1.5", className)}>
      <span className="shrink-0 text-xs text-faint">{label}</span>
      <span className={cn("min-w-0 truncate text-right text-[13px] text-ink", mono && "mono")}>
        {value}
      </span>
    </div>
  );
}

export function SectionTitle({ children, actions }: { children: ReactNode; actions?: ReactNode }) {
  return (
    <div className="mb-2 flex items-center justify-between gap-3">
      <h3 className="text-[11px] font-medium uppercase tracking-[0.08em] text-faint">{children}</h3>
      {actions}
    </div>
  );
}
