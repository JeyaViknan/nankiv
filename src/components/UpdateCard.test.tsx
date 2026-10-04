import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api } from "../lib/api";
import { UpdateCard } from "./UpdateCard";

vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("../lib/api", async () => {
  const actual =
    await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    api: {
      ...actual.api,
      appVersion: vi.fn(),
      checkForUpdate: vi.fn(),
      installUpdate: vi.fn(),
    },
  };
});

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(api.appVersion).mockResolvedValue({
    version: "0.1.3",
    built: "2026-10-05T10:00:00Z",
  });
});

describe("about nankiv", () => {
  it("shows the version, and checks only when asked", async () => {
    render(<UpdateCard />);
    expect(await screen.findByText(/Version 0\.1\.3/)).toBeInTheDocument();
    expect(api.checkForUpdate).not.toHaveBeenCalled();
  });

  it("says so when this is the newest version", async () => {
    vi.mocked(api.checkForUpdate).mockResolvedValue({ status: "up_to_date" });
    const user = userEvent.setup();
    render(<UpdateCard />);
    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(
      await screen.findByText("You have the newest version."),
    ).toBeInTheDocument();
  });

  it("offers a newer version, and installs it in place", async () => {
    vi.mocked(api.checkForUpdate).mockResolvedValue({
      status: "available",
      version: "0.1.4",
      notes: "Your circle always comes second.",
    });
    vi.mocked(api.installUpdate).mockResolvedValue(undefined);
    const user = userEvent.setup();
    render(<UpdateCard />);
    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    expect(await screen.findByText("nankiv 0.1.4")).toBeInTheDocument();
    expect(
      screen.getByText("Your circle always comes second."),
    ).toBeInTheDocument();
    await user.click(
      screen.getByRole("button", { name: "Install and restart" }),
    );
    expect(api.installUpdate).toHaveBeenCalled();
    expect(screen.getByText(/Downloading and installing/)).toBeInTheDocument();
  });

  it("falls back to the releases page, opened in the browser", async () => {
    vi.mocked(api.checkForUpdate).mockRejectedValue({
      code: "update_check_failed",
      message: "Couldn't check for updates.",
      detail: null,
    });
    const user = userEvent.setup();
    render(<UpdateCard />);
    await user.click(screen.getByRole("button", { name: "Check for updates" }));
    await user.click(
      await screen.findByRole("button", { name: "Open the releases page" }),
    );
    expect(openUrl).toHaveBeenCalledWith(
      "https://github.com/JeyaViknan/nankiv/releases",
    );
  });

  it("starts the check itself when opened from Check for Updates", async () => {
    vi.mocked(api.checkForUpdate).mockResolvedValue({ status: "up_to_date" });
    render(<UpdateCard autoCheck />);
    expect(
      await screen.findByText("You have the newest version."),
    ).toBeInTheDocument();
  });
});
