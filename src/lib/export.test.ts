/**
 * Downloading the names on a shortlist.
 *
 * The rules pinned here are about what the student is told, because the file
 * itself is tested in Rust: a partly named list must never be announced as if
 * it were complete, and backing out of the save panel is not an error.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { exportFilename, exportMessage, useStore } from "./store";
import { api, type ImportOutcome } from "./api";
import { save } from "@tauri-apps/plugin-dialog";

vi.mock("@tauri-apps/plugin-dialog", () => ({ save: vi.fn(), open: vi.fn() }));

vi.mock("./api", async () => {
  const actual = await vi.importActual<typeof import("./api")>("./api");
  return { ...actual, api: { exportShortlist: vi.fn() } };
});

const current = { drive_id: 7, company: "Tredence" } as ImportOutcome;

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ current: null, toast: null });
});

describe("the default filename", () => {
  it("names the file after the drive, with the right extension", () => {
    expect(exportFilename("Tredence", "xlsx")).toBe("Tredence shortlist.xlsx");
    expect(exportFilename("Tredence", "csv")).toBe("Tredence shortlist.csv");
  });

  it("strips characters macOS or Windows would refuse", () => {
    expect(exportFilename('A/B: "Test"?', "csv")).toBe(
      "A B Test shortlist.csv",
    );
  });

  it("still produces a usable name for a blank drive name", () => {
    expect(exportFilename("  ", "xlsx")).toBe("Shortlist shortlist.xlsx");
  });
});

describe("the confirmation", () => {
  it("never announces a partly named list as complete", () => {
    const msg = exportMessage(1815, 479);
    expect(msg).toContain("1,815");
    expect(msg).toContain("479 named");
    expect(msg).toMatch(/rest by ID/);
  });

  it("says so plainly when nobody could be named", () => {
    expect(exportMessage(527, 0)).toMatch(/none could be named/);
  });

  it("only claims 'all named' when it is true", () => {
    expect(exportMessage(40, 40)).toMatch(/all named/);
    expect(exportMessage(40, 39)).not.toMatch(/all named/);
  });

  it("uses the singular for one student", () => {
    expect(exportMessage(1, 1)).toMatch(/1 student,/);
  });
});

describe("exporting", () => {
  it("asks for a shortlist to be open rather than failing silently", async () => {
    await useStore.getState().exportNames("xlsx");
    expect(vi.mocked(save)).not.toHaveBeenCalled();
    expect(useStore.getState().toast?.message).toMatch(/Open a shortlist/);
  });

  it("treats dismissing the save panel as a normal outcome", async () => {
    useStore.setState({ current });
    vi.mocked(save).mockResolvedValue(null);

    await useStore.getState().exportNames("csv");

    expect(vi.mocked(api.exportShortlist)).not.toHaveBeenCalled();
    expect(useStore.getState().toast).toBeNull();
  });

  it("offers the matching file type and a sensible default name", async () => {
    useStore.setState({ current });
    vi.mocked(save).mockResolvedValue(null);

    await useStore.getState().exportNames("xlsx");

    const opts = vi.mocked(save).mock.calls[0]![0]!;
    expect(opts.defaultPath).toBe("Tredence shortlist.xlsx");
    expect(opts.filters?.[0]?.extensions).toEqual(["xlsx"]);
  });

  it("writes to the chosen path in the chosen format and reports honestly", async () => {
    useStore.setState({ current });
    vi.mocked(save).mockResolvedValue(
      "/Users/me/Desktop/Tredence shortlist.csv",
    );
    vi.mocked(api.exportShortlist).mockResolvedValue({
      path: "/Users/me/Desktop/Tredence shortlist.csv",
      rows: 1815,
      named: 479,
    });

    await useStore.getState().exportNames("csv");

    expect(vi.mocked(api.exportShortlist)).toHaveBeenCalledWith(
      7,
      "/Users/me/Desktop/Tredence shortlist.csv",
      "csv",
    );
    expect(useStore.getState().toast?.message).toMatch(/479 named/);
  });

  it("surfaces a write failure in words the student can act on", async () => {
    useStore.setState({ current });
    vi.mocked(save).mockResolvedValue("/nowhere/list.xlsx");
    vi.mocked(api.exportShortlist).mockRejectedValue({
      code: "export_failed",
      message: "Couldn't save the file there.",
      detail: null,
    });

    await useStore.getState().exportNames("xlsx");

    expect(useStore.getState().toast?.message).toMatch(/Couldn't save/);
  });
});
