import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatDuration,
  initials,
  humanize,
  safeHexColor,
  toMillis,
  truncate,
} from "./format";

describe("toMillis", () => {
  it("converts ER:LC unix seconds to milliseconds", () => {
    expect(toMillis(1_700_000_000)).toBe(1_700_000_000_000);
  });

  it("keeps millisecond timestamps", () => {
    expect(toMillis(1_700_000_000_123)).toBe(1_700_000_000_123);
  });

  it("treats empty values as missing", () => {
    expect(toMillis(0)).toBeNull();
    expect(toMillis(null)).toBeNull();
    expect(toMillis(undefined)).toBeNull();
  });
});

describe("formatting helpers", () => {
  it("formats byte sizes", () => {
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(null)).toBe("—");
  });

  it("formats durations", () => {
    expect(formatDuration(420)).toBe("420 ms");
    expect(formatDuration(2500)).toBe("2.5 s");
    expect(formatDuration(65_000)).toBe("1m 5s");
  });

  it("builds compact initials", () => {
    expect(initials("Builderman")).toBe("BU");
    expect(initials("john doe")).toBe("JD");
    expect(initials("")).toBe("?");
  });

  it("humanises permission keys", () => {
    expect(humanize("commands.policies.manage")).toBe("Commands policies manage");
  });

  it("truncates long text only", () => {
    expect(truncate("short", 10)).toBe("short");
    expect(truncate("a very long value", 8)).toBe("a very …");
  });
});

describe("safeHexColor", () => {
  it("accepts valid hex colours", () => {
    expect(safeHexColor("#fff")).toBe("#fff");
    expect(safeHexColor("#112233")).toBe("#112233");
  });

  it("rejects CSS injection attempts", () => {
    expect(safeHexColor("red; background:url(x)")).toBeNull();
    expect(safeHexColor("#12345")).toBeNull();
  });
});
