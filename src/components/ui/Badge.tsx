import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import type { Tone } from "@/lib/status";

const TONES: Record<Tone, string> = {
  neutral: "border-line-strong bg-surface-3 text-ink",
  muted: "border-line bg-surface-2 text-muted",
  positive: "border-white/25 bg-white/10 text-white",
  warning: "border-white/20 bg-transparent text-paper-200",
  danger: "border-danger/35 bg-danger/10 text-danger",
};

export function Badge({
  children,
  tone = "neutral",
  dot,
  className,
}: {
  children: ReactNode;
  tone?: Tone;
  dot?: boolean;
  className?: string;
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded border px-1.5 py-0.5 text-[11px] font-medium leading-4",
        TONES[tone],
        className,
      )}
    >
      {dot ? (
        <span
          className={cn(
            "h-1.5 w-1.5 rounded-full",
            tone === "positive"
              ? "bg-white"
              : tone === "danger"
                ? "bg-danger"
                : tone === "warning"
                  ? "bg-paper-300"
                  : "bg-faint",
          )}
        />
      ) : null}
      {children}
    </span>
  );
}

/** Small dot + text used for connection indicators. */
export function StatusDot({
  tone = "neutral",
  pulse,
  className,
}: {
  tone?: "online" | "offline" | "warn" | "neutral";
  pulse?: boolean;
  className?: string;
}) {
  const color =
    tone === "online"
      ? "bg-white"
      : tone === "offline"
        ? "bg-danger"
        : tone === "warn"
          ? "bg-paper-300"
          : "bg-faint";
  return (
    <span
      className={cn("inline-block h-1.5 w-1.5 rounded-full", color, pulse && "animate-live", className)}
    />
  );
}
