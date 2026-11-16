import type { CallPriority, CallStatus, UnitStatus } from "@/types";

export type Tone = "neutral" | "muted" | "positive" | "warning" | "danger";

export const UNIT_STATUSES: { value: UnitStatus; label: string; tone: Tone }[] = [
  { value: "available", label: "Available", tone: "positive" },
  { value: "on_duty", label: "On duty", tone: "neutral" },
  { value: "en_route", label: "En route", tone: "warning" },
  { value: "on_scene", label: "On scene", tone: "warning" },
  { value: "transporting", label: "Transporting", tone: "warning" },
  { value: "unavailable", label: "Unavailable", tone: "muted" },
  { value: "off_duty", label: "Off duty", tone: "muted" },
];

export function unitStatusLabel(status: string): string {
  return UNIT_STATUSES.find((entry) => entry.value === status)?.label ?? status;
}

export function unitStatusTone(status: string): Tone {
  return UNIT_STATUSES.find((entry) => entry.value === status)?.tone ?? "neutral";
}

export const CALL_STATUSES: { value: CallStatus; label: string; tone: Tone }[] = [
  { value: "open", label: "Open", tone: "warning" },
  { value: "assigned", label: "Assigned", tone: "neutral" },
  { value: "in_progress", label: "In progress", tone: "neutral" },
  { value: "closed", label: "Closed", tone: "muted" },
];

export function callStatusLabel(status: string): string {
  return CALL_STATUSES.find((entry) => entry.value === status)?.label ?? status;
}

export function callStatusTone(status: string): Tone {
  return CALL_STATUSES.find((entry) => entry.value === status)?.tone ?? "neutral";
}

export const CALL_PRIORITIES: { value: CallPriority; label: string; tone: Tone }[] = [
  { value: "low", label: "Low", tone: "muted" },
  { value: "medium", label: "Medium", tone: "neutral" },
  { value: "high", label: "High", tone: "warning" },
  { value: "critical", label: "Critical", tone: "danger" },
];

export function priorityTone(priority: string): Tone {
  return CALL_PRIORITIES.find((entry) => entry.value === priority)?.tone ?? "neutral";
}

export function actionLabel(action: string): string {
  switch (action) {
    case "kick":
      return "Kick";
    case "ban":
      return "Ban";
    case "unban":
      return "Unban";
    case "internal_ban":
      return "Internal ban";
    case "internal_unban":
      return "Internal unban";
    case "note":
      return "Note";
    default:
      return action;
  }
}

export function actionTone(action: string): Tone {
  switch (action) {
    case "kick":
    case "ban":
    case "internal_ban":
      return "danger";
    case "unban":
    case "internal_unban":
      return "positive";
    default:
      return "neutral";
  }
}

export function sourceLabel(source: string): string {
  return source === "erlc_api" ? "ER:LC API" : "Internal CAD";
}

export function serviceLabel(service: string): string {
  switch (service) {
    case "law_enforcement":
      return "Law enforcement";
    case "fire_ems":
      return "Fire / EMS";
    default:
      return "Civilian";
  }
}

export function permissionLabel(key: string): string {
  return key
    .split(".")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join(" ");
}
