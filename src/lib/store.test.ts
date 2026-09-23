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
import type { DriveRecord, DriveSnapshot, ImportOutcome } from "./api";

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
      takePendingRoute: vi.fn(),
      takePendingFiles: vi.fn(),
      season: vi.fn(),
      tap: vi.fn(),
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
  vi.mocked(api.takePendingRoute).mockResolvedValue(null);
  vi.mocked(api.takePendingFiles).mockResolvedValue([]);
  vi.mocked(api.tap).mockResolvedValue(undefined);
  vi.mocked(api.season).mockResolvedValue({
    drives: 0,
    shortlisted: 0,
    not_shortlisted: 0,
    undetermined: 0,
  });
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

describe("dropping a reference sheet", () => {
  it("treats it as a success, not a failed import", async () => {
    // The batch CGPA sheet is the right kind of thing to hand nankiv — it just
    // isn't a shortlist. Dropping it used to create a nonsense drive.
    vi.mocked(api.importShortlist).mockRejectedValue({
      code: "imported_reference",
      message: "Added academic records for 2,506 students.",
      detail: "name_cgpa_resume.xlsx",
    });

    await useStore.getState().importFile("/tmp/name_cgpa_resume.xlsx");

    const s = useStore.getState();
    expect(s.importStage.phase).toBe("idle");
    expect(s.toast?.message).toMatch(/Added academic records/);
    // Nothing was opened, and crucially nothing failed.
    expect(s.current).toBeNull();
    expect(s.error).toBeNull();
  });

  it("re-runs the open drive so the analysis appears immediately", async () => {
    useStore.setState({ current: outcome(5, "Tredence"), view: "drive" });
    vi.mocked(api.importShortlist).mockRejectedValue({
      code: "imported_reference",
      message: "Added academic records for 2,506 students.",
      detail: "sheet.xlsx",
    });
    vi.mocked(api.getDriveDetail).mockResolvedValue(outcome(5, "Tredence"));

    await useStore.getState().importFile("/tmp/sheet.xlsx");

    // Without this the student adds the data and still sees "not set up".
    expect(vi.mocked(api.getDriveDetail)).toHaveBeenCalledWith(5);
  });
});

describe("following a widget link", () => {
  function record(id: number, company: string): DriveRecord {
    return {
      id,
      company,
      drive_date: null,
      imported_at: "2026-09-01 10:00:00",
      source_filename: `${company}.xlsx`,
      content_hash: `H${id}`,
      shape: "neo_id_only",
      primary_key: "neo_id",
      total_students: 166,
      round_label: null,
      parent_drive_id: null,
    };
  }

  it("opens the shortlist the widget was showing", async () => {
    vi.mocked(api.listDrives).mockResolvedValue([record(7, "Amazon")]);
    vi.mocked(api.getDriveDetail).mockResolvedValue(outcome(7, "Amazon"));
    await useStore.getState().followRoute({ kind: "drive", id: 7 });
    expect(api.getDriveDetail).toHaveBeenCalledWith(7);
    expect(useStore.getState().view).toBe("drive");
    expect(useStore.getState().current?.drive_id).toBe(7);
  });

  it("explains, rather than fails, when that shortlist was deleted", async () => {
    vi.mocked(api.listDrives).mockResolvedValue([record(8, "Siemens")]);
    await useStore.getState().followRoute({ kind: "drive", id: 7 });
    expect(api.getDriveDetail).not.toHaveBeenCalled();
    expect(useStore.getState().view).toBe("shortlists");
    expect(useStore.getState().toast?.message).toMatch(/no longer in nankiv/);
    expect(useStore.getState().error).toBeNull();
  });

  it("checks against the drives that exist now, not the cached list", async () => {
    useStore.setState({ drives: [record(7, "Amazon")] });
    vi.mocked(api.listDrives).mockResolvedValue([]);
    await useStore.getState().followRoute({ kind: "drive", id: 7 });
    expect(api.getDriveDetail).not.toHaveBeenCalled();
  });

  it("opens the newest shortlist for the latest link", async () => {
    vi.mocked(api.listDrives).mockResolvedValue([
      record(9, "Tredence"),
      record(4, "Elgi"),
    ]);
    vi.mocked(api.getDriveDetail).mockResolvedValue(outcome(9, "Tredence"));
    await useStore.getState().followRoute({ kind: "latest" });
    expect(api.getDriveDetail).toHaveBeenCalledWith(9);
  });

  it("clears the held link so it is never followed twice", async () => {
    await useStore.getState().followRoute({ kind: "shortlists" });
    expect(api.takePendingRoute).toHaveBeenCalled();
  });

  it("does not skip setup", async () => {
    useStore.setState({ view: "onboarding" });
    vi.mocked(api.listDrives).mockResolvedValue([record(7, "Amazon")]);
    await useStore.getState().followRoute({ kind: "drive", id: 7 });
    expect(useStore.getState().view).toBe("onboarding");
    expect(api.getDriveDetail).not.toHaveBeenCalled();
  });
});

describe("shortlists dropped on the app icon", () => {
  it("imports each one in turn", async () => {
    vi.mocked(api.importShortlist)
      .mockResolvedValueOnce(outcome(1, "Elgi"))
      .mockResolvedValueOnce(outcome(2, "Siemens"));

    await useStore.getState().openFiles(["/a/Elgi.xlsx", "/a/Siemens.xlsx"]);

    expect(vi.mocked(api.importShortlist).mock.calls.map((c) => c[0])).toEqual([
      "/a/Elgi.xlsx",
      "/a/Siemens.xlsx",
    ]);
    expect(useStore.getState().current?.company).toBe("Siemens");
  });

  it("does not import before setup is finished", async () => {
    useStore.setState({ view: "onboarding" });
    await useStore.getState().openFiles(["/a/Elgi.xlsx"]);
    expect(api.importShortlist).not.toHaveBeenCalled();
  });

  it("clears the held files so they are not imported twice", async () => {
    vi.mocked(api.importShortlist).mockResolvedValue(outcome(1, "Elgi"));
    await useStore.getState().openFiles(["/a/Elgi.xlsx"]);
    expect(api.takePendingFiles).toHaveBeenCalled();
  });
});

describe("the moment a result lands", () => {
  it("taps the trackpad when you're in", async () => {
    vi.mocked(api.importShortlist).mockResolvedValue(outcome(1, "Siemens"));
    await useStore.getState().importFile("/a/Siemens.xlsx");
    expect(api.tap).toHaveBeenCalledTimes(1);
  });

  it("stays silent when you're not", async () => {
    const no = outcome(1, "Siemens");
    no.you.verdict = { status: "not_shortlisted" };
    vi.mocked(api.importShortlist).mockResolvedValue(no);
    await useStore.getState().importFile("/a/Siemens.xlsx");
    expect(api.tap).not.toHaveBeenCalled();
  });

  it("stays silent when the answer is unknown", async () => {
    const unknown = outcome(1, "Siemens");
    unknown.you.verdict = {
      status: "undetermined",
      reason: "needs_neo_id" as never,
    };
    vi.mocked(api.importShortlist).mockResolvedValue(unknown);
    await useStore.getState().importFile("/a/Siemens.xlsx");
    expect(api.tap).not.toHaveBeenCalled();
  });

  it("does not fail an import when the trackpad cannot be tapped", async () => {
    vi.mocked(api.tap).mockRejectedValue(new Error("no haptics here"));
    vi.mocked(api.importShortlist).mockResolvedValue(outcome(1, "Siemens"));
    await useStore.getState().importFile("/a/Siemens.xlsx");
    expect(useStore.getState().current?.company).toBe("Siemens");
    expect(useStore.getState().error).toBeNull();
  });
});
