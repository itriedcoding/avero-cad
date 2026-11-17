import { describe, expect, it } from "vitest";
import { formatElapsed, formatSpan } from "./format";

describe("formatSpan", () => {
  it("renders sub-minute, minute and hour spans", () => {
    expect(formatSpan(0)).toBe("0m");
    expect(formatSpan(45_000)).toBe("45s");
    expect(formatSpan(45 * 60_000)).toBe("45m");
    expect(formatSpan(3 * 3_600_000 + 24 * 60_000)).toBe("3h 24m");
  });

  it("handles missing values", () => {
    expect(formatSpan(null)).toBe("—");
    expect(formatSpan(undefined)).toBe("—");
  });
});

describe("formatElapsed", () => {
  it("renders a running HH:MM:SS clock", () => {
    expect(formatElapsed(0)).toBe("00:00:00");
    expect(formatElapsed(3_723_000)).toBe("01:02:03");
    expect(formatElapsed(-5_000)).toBe("00:00:00");
  });
});
