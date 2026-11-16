import { describe, expect, it } from "vitest";
import {
  CALL_PRIORITIES,
  CALL_STATUSES,
  UNIT_STATUSES,
  actionLabel,
  actionTone,
  callStatusLabel,
  callStatusTone,
  permissionLabel,
  priorityTone,
  serviceLabel,
  sourceLabel,
  unitStatusLabel,
  unitStatusTone,
} from "./status";

describe("unit statuses", () => {
  it("covers every documented status", () => {
    expect(UNIT_STATUSES.map((entry) => entry.value)).toEqual([
      "available",
      "on_duty",
      "en_route",
      "on_scene",
      "transporting",
      "unavailable",
      "off_duty",
    ]);
  });

  it("maps labels and tones with a safe fallback", () => {
    expect(unitStatusLabel("en_route")).toBe("En route");
    expect(unitStatusTone("available")).toBe("positive");
    expect(unitStatusLabel("unknown")).toBe("unknown");
    expect(unitStatusTone("unknown")).toBe("neutral");
  });
});

describe("call statuses and priorities", () => {
  it("labels lifecycle statuses", () => {
    expect(CALL_STATUSES).toHaveLength(4);
    expect(callStatusLabel("in_progress")).toBe("In progress");
    expect(callStatusTone("closed")).toBe("muted");
    expect(callStatusLabel("weird")).toBe("weird");
  });

  it("tones priorities", () => {
    expect(priorityTone("critical")).toBe("danger");
    expect(priorityTone("low")).toBe("muted");
    expect(CALL_PRIORITIES.find((entry) => entry.value === "high")?.label).toBe("High");
  });
});

describe("moderation and permission labels", () => {
  it("labels every moderation action", () => {
    expect(actionLabel("kick")).toBe("Kick");
    expect(actionLabel("ban")).toBe("Ban");
    expect(actionLabel("unban")).toBe("Unban");
    expect(actionLabel("internal_ban")).toBe("Internal ban");
    expect(actionLabel("internal_unban")).toBe("Internal unban");
    expect(actionLabel("note")).toBe("Note");
  });

  it("separates ER:LC from internal sources", () => {
    expect(sourceLabel("erlc_api")).toBe("ER:LC API");
    expect(sourceLabel("internal_cad")).toBe("Internal CAD");
    expect(actionTone("ban")).toBe("danger");
    expect(actionTone("unban")).toBe("positive");
  });

  it("renders permission keys and services", () => {
    expect(permissionLabel("moderation.ban")).toBe("Moderation Ban");
    expect(serviceLabel("fire_ems")).toBe("Fire / EMS");
    expect(serviceLabel("other")).toBe("Civilian");
  });
});
