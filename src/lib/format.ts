/** ER:LC log timestamps are Unix seconds; CAD records use milliseconds. */
export function toMillis(value: number | null | undefined): number | null {
  if (value === null || value === undefined || value === 0) return null;
  return value < 1_000_000_000_000 ? value * 1000 : value;
}

export function formatTime(value: number | null | undefined): string {
  const ms = toMillis(value);
  if (ms === null) return "—";
  return new Date(ms).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

export function formatDateTime(value: number | null | undefined): string {
  const ms = toMillis(value);
  if (ms === null) return "—";
  return new Date(ms).toLocaleString([], {
    year: "numeric",
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function formatRelative(value: number | null | undefined): string {
  const ms = toMillis(value);
  if (ms === null) return "—";
  const delta = Date.now() - ms;
  const future = delta < 0;
  const seconds = Math.round(Math.abs(delta) / 1000);
  const suffix = future ? "from now" : "ago";
  if (seconds < 10) return future ? "in a moment" : "just now";
  if (seconds < 60) return `${seconds}s ${suffix}`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ${suffix}`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ${suffix}`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days}d ${suffix}`;
  return formatDateTime(value);
}

export function formatDuration(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return "—";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(seconds < 10 ? 1 : 0)} s`;
  const minutes = Math.floor(seconds / 60);
  return `${minutes}m ${Math.round(seconds % 60)}s`;
}

/** Compact human duration for shift totals: "45m", "3h 24m". */
export function formatSpan(ms: number | null | undefined): string {
  if (ms === null || ms === undefined) return "—";
  if (ms <= 0) return "0m";
  const totalMinutes = Math.floor(ms / 60_000);
  if (totalMinutes === 0) return `${Math.max(1, Math.floor(ms / 1000))}s`;
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours === 0) return `${minutes}m`;
  return `${hours}h ${minutes.toString().padStart(2, "0")}m`;
}

/** Running clock for an open shift: "HH:MM:SS". */
export function formatElapsed(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  return [hours, minutes, seconds].map((value) => value.toString().padStart(2, "0")).join(":");
}

export function formatBytes(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return "—";
  const units = ["B", "KB", "MB", "GB"];
  let size = bytes;
  let unit = 0;
  while (size >= 1024 && unit < units.length - 1) {
    size /= 1024;
    unit += 1;
  }
  return unit === 0 ? `${bytes} B` : `${size.toFixed(1)} ${units[unit]}`;
}

export function formatNumber(value: number | null | undefined): string {
  if (value === null || value === undefined) return "—";
  return value.toLocaleString();
}

export function initials(name: string): string {
  const parts = name.replace(/[^\p{L}\p{N} _-]/gu, "").split(/[\s_-]+/).filter(Boolean);
  if (parts.length === 0) return "?";
  if (parts.length === 1) return parts[0]!.slice(0, 2).toUpperCase();
  return `${parts[0]![0] ?? ""}${parts[1]![0] ?? ""}`.toUpperCase();
}

/** "player.kicked" -> "Player kicked" */
export function humanize(key: string): string {
  const text = key.replace(/[._]/g, " ").trim();
  return text.charAt(0).toUpperCase() + text.slice(1);
}

export function truncate(value: string, max: number): string {
  if (value.length <= max) return value;
  return `${value.slice(0, Math.max(0, max - 1))}…`;
}

export function safeHexColor(value: string): string | null {
  const trimmed = value.trim();
  return /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(trimmed) ? trimmed : null;
}
