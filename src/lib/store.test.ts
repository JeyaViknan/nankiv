/**
 * The interaction rules that live in the store.
 *
 * These are the behaviours the redesign turns on, so they are pinned here
 * rather than left to manual checking: a duplicate must not read as a failure,
 * a deletion must be recoverable, and a file nankiv cannot read must never
 * reach the interface looking like a verdict.
 */

import { beforeEach, describe, expect, it, vi } from "vitest";
import { useStore } from "./store";
import { api } from "./api";
import type { DriveSnapshot, ImportOutcome } from "./api";

vi.mock("./api", async () => {
  const actual = await vi.importActual<typeof import("./api")>("./api");
  return {
    ...actual,
    api: {
      getProfile: vi.fn(),
      listFriends: vi.fn(),
      listDrives: vi.fn(),
      identityStats: vi.fn(),
      importShortlist: vi.fn(),
      getDriveDetail: vi.fn(),
      deleteDrive: vi.fn(),
      restoreDrive: vi.fn(),
      renameDrive: vi.fn(),
    },
  };
});

function outcome(id: number, company: string): ImportOutcome {
  return {
    drive_id: id,
    company,
    drive_date: null,
    total_students: 166,
    shape: "neo_id_only",
    primary_key: "neo_id",
    you: {
      label: "You",
      neo_id: "V9H0G6C4",
      reg_no: null,
      verdict: { status: "shortlisted" },
      confidence: "verified",
      cgpa: null,
    },
    friends: [],
    analysis: {
      total_students: 166,
      matched_students: 0,
      coverage: 0,
      sufficient: false,
      cgpa: null,
      cutoff: null,
      branches: null,
      your_percentile: null,
    },
    learned_verified: 0,
    learned_named: 0,
    unreadable_headers: null,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({
    view: "shortlists",
    current: null,
    drives: [],
    importStage: { phase: "idle" },
    toast: null,
    error: null,
  });
  vi.mocked(api.listDrives).mockResolvedValue([]);
  vi.mocked(api.identityStats).mockResolvedValue({
    students_known: 0,
    names_known: 0,
    academics_known: 0,
    edges: 0,
    conflicts: 0,
  });
});

describe("importing", () => {
  it("acknowledges the file by name while it is being read", async () => {
    let resolve!: (v: ImportOutcome) => void;
    vi.mocked(api.importShortlist).mockReturnValue(
      new Promise<ImportOutcome>((r) => {
        resolve = r;
      }),
    );

    const p = useStore.getState().importFile("/tmp/Siemens SISW 2027.xlsx");

    // Before anything is computed, the student can see the drop landed.
    expect(useStore.getState().importStage).toEqual({
      phase: "reading",
      filename: "Siemens SISW 2027.xlsx",
    });

    resolve(outcome(1, "Siemens SISW"));
    await p;

    expect(useStore.getState().view).toBe("drive");
    expect(useStore.getState().current?.company).toBe("Siemens SISW");
    expect(useStore.getState().importStage.phase).toBe("idle");
  });

  it("opens the existing drive when the same file is dropped again", async () => {
    // Re-dropping yesterday's file is ordinary behaviour. It used to render as
    // a red error banner.
    vi.mocked(api.importShortlist).mockRejectedValue({
      code: "duplicate_drive",
      message: "You already imported this Siemens shortlist.",
      detail: "7",
    });
    vi.mocked(api.getDriveDetail).mockResolvedValue(outcome(7, "Siemens"));

    await useStore.getState().importFile("/tmp/again.xlsx");

    const s = useStore.getState();
    expect(vi.mocked(api.getDriveDetail)).toHaveBeenCalledWith(7);
    expect(s.view).toBe("drive");
    expect(s.current?.drive_id).toBe(7);
    expect(s.error).toBeNull();
    expect(s.importStage.phase).toBe("idle");
    expect(s.toast?.message).toMatch(/already imported/i);
  });

  it("reports an unreadable file as a failed import, not as a verdict", async () => {
    vi.mocked(api.importShortlist).mockRejectedValue({
      code: "file_not_understood",
      message: "This file doesn't use Neo IDs or registration numbers.",
      detail: "Columns found: REFERENCE_ID",
    });

    await useStore.getState().importFile("/tmp/TCS.xlsx");

    const s = useStore.getState();
    expect(s.importStage.phase).toBe("failed");
    // Crucially: no drive was opened, so nothing can be mistaken for a result.
    expect(s.current).toBeNull();
    expect(s.view).toBe("shortlists");
  });

  it("keeps the filename on a failure so the student knows which file failed", async () => {
    vi.mocked(api.importShortlist).mockRejectedValue({
      code: "parse_failed",
      message: "Couldn't read that file.",
      detail: null,
    });
    await useStore.getState().importFile("/a/b/broken.xlsx");
    const stage = useStore.getState().importStage;
    expect(stage.phase === "failed" && stage.filename).toBe("broken.xlsx");
  });
});

describe("deleting", () => {
  const snapshot: DriveSnapshot = {
    company: "Amazon",
    drive_date: null,
    source_filename: "amazon.xlsx",
    content_hash: "H",
    shape: "neo_id_only",
    primary_key: "neo_id",
    round_label: null,
    neo_ids: ["V9H0G6C4"],
    reg_nos: [],
  };

  it("offers undo instead of asking for confirmation", async () => {
    vi.mocked(api.deleteDrive).mockResolvedValue(snapshot);
    await useStore.getState().deleteDrive(3, "Amazon");

    const t = useStore.getState().toast;
    expect(t?.message).toBe("Removed Amazon");
    expect(typeof t?.undo).toBe("function");
  });

  it("puts the drive back when undo is taken", async () => {
    vi.mocked(api.deleteDrive).mockResolvedValue(snapshot);
    vi.mocked(api.restoreDrive).mockResolvedValue(9);

    await useStore.getState().deleteDrive(3, "Amazon");
    await useStore.getState().toast?.undo?.();

    expect(vi.mocked(api.restoreDrive)).toHaveBeenCalledWith(snapshot);
    expect(useStore.getState().toast?.message).toMatch(/Restored Amazon/);
  });

  it("leaves the detail view when the drive being shown is deleted", async () => {
    useStore.setState({ current: outcome(3, "Amazon"), view: "drive" });
    vi.mocked(api.deleteDrive).mockResolvedValue(snapshot);

    await useStore.getState().deleteDrive(3, "Amazon");

    expect(useStore.getState().view).toBe("shortlists");
    expect(useStore.getState().current).toBeNull();
  });

  it("still reports the removal when no snapshot came back", async () => {
    vi.mocked(api.deleteDrive).mockResolvedValue(null);
    await useStore.getState().deleteDrive(3, "Amazon");
    const t = useStore.getState().toast;
    expect(t?.message).toBe("Removed Amazon");
    expect(t?.undo).toBeUndefined();
  });
});

describe("renaming", () => {
  it("updates the open drive immediately", async () => {
    useStore.setState({ current: outcome(4, "Unnamed drive"), view: "drive" });
    vi.mocked(api.renameDrive).mockResolvedValue(undefined);

    await useStore.getState().renameDrive(4, "Fractal Analytics");

    expect(vi.mocked(api.renameDrive)).toHaveBeenCalledWith(
      4,
      "Fractal Analytics",
    );
    expect(useStore.getState().current?.company).toBe("Fractal Analytics");
  });

  it("does not touch a different drive that happens to be open", async () => {
    useStore.setState({ current: outcome(4, "Siemens"), view: "drive" });
    vi.mocked(api.renameDrive).mockResolvedValue(undefined);
    await useStore.getState().renameDrive(99, "Something else");
    expect(useStore.getState().current?.company).toBe("Siemens");
  });
});
