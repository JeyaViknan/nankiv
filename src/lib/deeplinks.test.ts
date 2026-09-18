/**
 * `nankiv://` links reach the interface from outside the app, so the shape of
 * what arrives is checked, and a link to something that is gone lands softly.
 */

import { describe, expect, it } from "vitest";
import { asPaths, isRoute, resolveRoute } from "./deeplinks";
import type { DriveRecord } from "./api";

const drive = (id: number) => ({ id }) as DriveRecord;

describe("resolveRoute", () => {
  it("opens a drive that exists", () => {
    expect(
      resolveRoute({ kind: "drive", id: 3 }, [drive(5), drive(3)]),
    ).toEqual({ kind: "open", id: 3 });
  });

  it("goes home with a notice when the drive is gone", () => {
    const action = resolveRoute({ kind: "drive", id: 3 }, [drive(5)]);
    expect(action.kind).toBe("home");
    expect(action.kind === "home" && action.notice).toBeTruthy();
  });

  it("treats latest as the first drive in the list", () => {
    expect(resolveRoute({ kind: "latest" }, [drive(9), drive(2)])).toEqual({
      kind: "open",
      id: 9,
    });
  });

  it("goes home quietly when there is no latest drive", () => {
    expect(resolveRoute({ kind: "latest" }, [])).toEqual({ kind: "home" });
  });

  it("goes home for the shortlists link", () => {
    expect(resolveRoute({ kind: "shortlists" }, [drive(1)])).toEqual({
      kind: "home",
    });
  });
});

describe("isRoute", () => {
  it("accepts every route the core emits", () => {
    expect(isRoute({ kind: "drive", id: 12 })).toBe(true);
    expect(isRoute({ kind: "latest" })).toBe(true);
    expect(isRoute({ kind: "shortlists" })).toBe(true);
  });

  it("rejects anything else", () => {
    for (const value of [
      null,
      undefined,
      "nankiv://drive/1",
      {},
      { kind: "wipe" },
      { kind: "drive" },
      { kind: "drive", id: "7" },
      { kind: "drive", id: 0 },
      { kind: "drive", id: -4 },
      { kind: "drive", id: 1.5 },
    ]) {
      expect(isRoute(value)).toBe(false);
    }
  });
});

describe("asPaths", () => {
  it("takes the paths out of what the core sent", () => {
    expect(asPaths(["/a/list.xlsx", "/b/other.csv"])).toEqual([
      "/a/list.xlsx",
      "/b/other.csv",
    ]);
  });

  it("ignores anything that is not a list of paths", () => {
    expect(asPaths(undefined)).toEqual([]);
    expect(asPaths("/a/list.xlsx")).toEqual([]);
    expect(asPaths([1, null, "/a/list.xlsx"])).toEqual(["/a/list.xlsx"]);
  });
});
