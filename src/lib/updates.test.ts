import { describe, expect, it } from "vitest";
import { builtLabel, shouldRemind } from "./updates";

const now = new Date("2026-12-01T12:00:00Z");
const daysAgo = (d: number) =>
  new Date(now.getTime() - d * 86_400_000).toISOString();

describe("reminding about updates, offline", () => {
  it("stays quiet while the build is recent", () => {
    expect(shouldRemind(daysAgo(10), null, now)).toBe(false);
    expect(shouldRemind(daysAgo(41), null, now)).toBe(false);
  });

  it("mentions an aging build once", () => {
    expect(shouldRemind(daysAgo(50), null, now)).toBe(true);
  });

  it("then leaves it for a fortnight", () => {
    expect(shouldRemind(daysAgo(50), daysAgo(3), now)).toBe(false);
    expect(shouldRemind(daysAgo(50), daysAgo(15), now)).toBe(true);
  });

  it("says nothing when it can't tell how old the build is", () => {
    expect(shouldRemind(null, null, now)).toBe(false);
    expect(shouldRemind("not a date", null, now)).toBe(false);
  });
});

describe("the build date", () => {
  it("is written out in full", () => {
    expect(builtLabel("2026-10-05T10:00:00Z")).toMatch(
      /5 October 2026|October 5, 2026/,
    );
    expect(builtLabel(null)).toBeNull();
  });
});
