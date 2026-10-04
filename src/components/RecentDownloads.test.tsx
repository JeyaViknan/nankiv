import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { api } from "../lib/api";
import { useStore } from "../lib/store";
import { RecentDownloads, downloadedWhen } from "./RecentDownloads";

vi.mock("../lib/api", async () => {
  const actual =
    await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return { ...actual, api: { ...actual.api, recentDownloads: vi.fn() } };
});

const now = new Date();
const hoursAgo = (h: number) =>
  new Date(now.getTime() - h * 3_600_000).toISOString();

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.recentDownloads).mockResolvedValue([
    {
      path: "/Users/a/Downloads/Siemens shortlist.xlsx",
      name: "Siemens shortlist.xlsx",
      modified: hoursAgo(0.5),
      imported: false,
    },
    {
      path: "/Users/a/Downloads/Zluri.xlsx",
      name: "Zluri.xlsx",
      modified: hoursAgo(30),
      imported: true,
    },
  ]);
});

describe("importing a recent download", () => {
  it("does not look in Downloads until asked", () => {
    render(<RecentDownloads disabled={false} />);
    expect(api.recentDownloads).not.toHaveBeenCalled();
  });

  it("lists the newest spreadsheets, and imports one in a click", async () => {
    const importFile = vi.fn().mockResolvedValue(undefined);
    useStore.setState({ importFile });
    const user = userEvent.setup();
    render(<RecentDownloads disabled={false} />);

    await user.click(
      screen.getByRole("button", { name: "Import a recent download" }),
    );
    expect(
      await screen.findByText("Siemens shortlist.xlsx"),
    ).toBeInTheDocument();
    expect(screen.getByText(/Imported · /)).toBeInTheDocument();

    await user.click(screen.getByText("Siemens shortlist.xlsx"));
    expect(importFile).toHaveBeenCalledWith(
      "/Users/a/Downloads/Siemens shortlist.xlsx",
    );
  });

  it("says plainly when it isn't allowed to look", async () => {
    vi.mocked(api.recentDownloads).mockRejectedValue({
      code: "downloads_denied",
      message: "nankiv isn't allowed to look in Downloads.",
      detail: null,
    });
    const user = userEvent.setup();
    render(<RecentDownloads disabled={false} />);
    await user.click(
      screen.getByRole("button", { name: "Import a recent download" }),
    );
    expect(await screen.findByRole("alert")).toHaveTextContent(
      "isn't allowed to look in Downloads",
    );
  });
});

describe("saying when it was downloaded", () => {
  const at = new Date("2026-10-05T18:00:00");
  it("counts minutes, then hours, then days", () => {
    expect(downloadedWhen("2026-10-05T17:59:30", at)).toBe("Just now");
    expect(downloadedWhen("2026-10-05T17:40:00", at)).toBe("20 minutes ago");
    expect(downloadedWhen("2026-10-05T16:00:00", at)).toBe("2 hours ago");
    expect(downloadedWhen("2026-10-04T22:00:00", at)).toBe("Yesterday");
    expect(downloadedWhen("2026-10-02T10:00:00", at)).toBe("3 days ago");
  });
});
