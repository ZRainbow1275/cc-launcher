import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { NoviceHome } from "@/components/novice";
import { renderWithMockIPC, teardownMockIPC } from "@/lib/api/mock";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() },
}));

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

async function waitForButtons(): Promise<void> {
  await waitFor(() => {
    expect(screen.getByTestId("novice-button-install")).toBeInTheDocument();
    expect(
      screen.getByTestId("novice-button-switch-profile"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("novice-button-launch")).toBeInTheDocument();
  });
}

describe("NoviceHome", () => {
  it("new-user: all 3 buttons visible; install enabled; others disabled; onboarding open", async () => {
    renderWithMockIPC("new-user", <NoviceHome />);
    await waitForButtons();

    const install = screen.getByTestId("novice-button-install");
    const switchProfile = screen.getByTestId("novice-button-switch-profile");
    const launch = screen.getByTestId("novice-button-launch");

    expect(install).not.toBeDisabled();

    await waitFor(() => {
      expect(switchProfile).toBeDisabled();
      expect(launch).toBeDisabled();
    });

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-dialog")).toBeInTheDocument();
    });
  });

  it("claude-installed-codex-missing: switchProfile enabled, launch enabled (active profile exists), no onboarding", async () => {
    renderWithMockIPC("claude-installed-codex-missing", <NoviceHome />);
    await waitForButtons();

    await waitFor(() => {
      expect(
        screen.getByTestId("novice-button-switch-profile"),
      ).not.toBeDisabled();
    });

    await waitFor(() => {
      expect(screen.getByTestId("novice-button-launch")).not.toBeDisabled();
    });

    expect(screen.queryByTestId("onboarding-dialog")).not.toBeInTheDocument();
  });

  it("all-installed-no-profile: switchProfile enabled; launch disabled with no-profile reason", async () => {
    renderWithMockIPC("all-installed-no-profile", <NoviceHome />);
    await waitForButtons();

    await waitFor(() => {
      expect(
        screen.getByTestId("novice-button-switch-profile"),
      ).not.toBeDisabled();
    });

    await waitFor(() => {
      expect(screen.getByTestId("novice-button-launch")).toBeDisabled();
    });

    const launchWrapper = screen.getByTestId("novice-button-launch-wrapper");
    expect(launchWrapper).toBeInTheDocument();
  });

  it("fully-configured: all 3 buttons enabled; statusBar shows active profile name", async () => {
    renderWithMockIPC("fully-configured", <NoviceHome />);
    await waitForButtons();

    await waitFor(() => {
      expect(screen.getByTestId("novice-button-install")).not.toBeDisabled();
      expect(
        screen.getByTestId("novice-button-switch-profile"),
      ).not.toBeDisabled();
      expect(screen.getByTestId("novice-button-launch")).not.toBeDisabled();
    });

    await waitFor(() => {
      const seg = screen.getByTestId("novice-status-profile");
      expect(seg.textContent ?? "").not.toContain("novice.statusBar.noProfile");
      expect((seg.textContent ?? "").length).toBeGreaterThan(0);
    });
  });
});
