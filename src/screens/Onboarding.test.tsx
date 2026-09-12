/**
 * Onboarding.
 *
 * The bug this pins: saving the profile updated the store but never changed the
 * view, so the student was left staring at the form they had just completed.
 * The app only recovered on the next launch, which reads as a freeze.
 */

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { OnboardingScreen } from "./Onboarding";
import { useStore } from "../lib/store";
import { api } from "../lib/api";

vi.mock("../lib/api", async () => {
  const actual =
    await vi.importActual<typeof import("../lib/api")>("../lib/api");
  return {
    ...actual,
    api: { saveProfile: vi.fn(), getProfile: vi.fn() },
  };
});

beforeEach(() => {
  vi.clearAllMocks();
  useStore.setState({ view: "onboarding", profile: null, error: null });
  vi.mocked(api.saveProfile).mockResolvedValue(undefined);
  vi.mocked(api.getProfile).mockResolvedValue({
    neo_id: "V9H0G6C4",
    reg_no: null,
    display_name: null,
    cohort: null,
    show_friend_cgpa: false,
  });
});

describe("finishing setup", () => {
  it("moves on once the profile is saved", async () => {
    const user = userEvent.setup();
    render(<OnboardingScreen />);

    await user.type(screen.getByLabelText("Neo ID"), "V9H0G6C4");
    await user.click(screen.getByRole("button", { name: "Continue" }));

    expect(vi.mocked(api.saveProfile)).toHaveBeenCalled();
    // The whole point: the student is not left on the form.
    expect(useStore.getState().view).toBe("shortlists");
  });

  it("stays put when saving fails, so nothing is silently lost", async () => {
    vi.mocked(api.saveProfile).mockRejectedValue({
      code: "bad_neo_id",
      message: "That doesn't look like a Neo ID.",
      detail: null,
    });
    const user = userEvent.setup();
    render(<OnboardingScreen />);

    await user.type(screen.getByLabelText("Neo ID"), "NOTANID1");
    await user.click(screen.getByRole("button", { name: "Continue" }));

    expect(useStore.getState().view).toBe("onboarding");
    expect(
      await screen.findByText(/doesn't look like a Neo ID/),
    ).toBeInTheDocument();
  });

  it("accepts a registration number alone", async () => {
    const user = userEvent.setup();
    render(<OnboardingScreen />);

    await user.type(screen.getByLabelText("Registration number"), "23BAI0002");
    await user.click(screen.getByRole("button", { name: "Continue" }));

    expect(useStore.getState().view).toBe("shortlists");
  });
});

describe("agency", () => {
  it("can be skipped, because a guided flow must be escapable", async () => {
    const user = userEvent.setup();
    render(<OnboardingScreen />);

    await user.click(screen.getByRole("button", { name: /Set this up later/ }));

    expect(useStore.getState().view).toBe("shortlists");
    expect(vi.mocked(api.saveProfile)).not.toHaveBeenCalled();
  });

  it("says plainly what skipping costs", () => {
    render(<OnboardingScreen />);
    expect(
      screen.getByText(/won't be able to say whether any of them include you/i),
    ).toBeInTheDocument();
  });

  it("will not submit an empty form", () => {
    render(<OnboardingScreen />);
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();
  });
});
