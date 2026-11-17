import type { ReactNode } from "react";
import { cn } from "@/lib/cn";
import type { Tone } from "@/lib/status";

const TONES: Record<Tone, string> = {
  neutral: "border-white/12 bg-white/[0.06] text-ink",
  muted: "border-white/10 bg-transparent text-muted",
  positive: "border-accent/45 bg-accent/12 text-accent-soft",
  warning: "border-warn/40 bg-warn/10 text-warn",
  danger: "border-danger/55 bg-danger/16 text-[#ffd9dc]",
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
        "inline-flex items-center gap-1.5 rounded border px-1.5 py-0.5 text-[11px] font-bold uppercase leading-4 tracking-[0.04em]",
        TONES[tone],
        className,
      )}
    >
      {dot ? (
        <span
          className={cn(
            "h-1.5 w-1.5 rounded-full",
            tone === "positive"
              ? "bg-accent"
              : tone === "danger"
                ? "bg-danger"
                : tone === "warning"
                  ? "bg-warn"
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
      ? "bg-accent shadow-[0_0_10px_rgb(255_42_58/0.85)]"
      : tone === "offline"
        ? "bg-danger"
        : tone === "warn"
          ? "bg-warn"
          : "bg-faint";
  return (
    <span
      className={cn("inline-block h-1.5 w-1.5 rounded-full", color, pulse && "animate-live", className)}
    />
  );
}
