import { screen, waitFor, fireEvent, act } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { LauncherPanel } from "@/components/launcher/LauncherPanel";
import {
  cliStateMock,
  launcherMock,
  renderWithMockIPC,
  teardownMockIPC,
} from "@/lib/api/mock";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

beforeEach(() => {});

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("LauncherPanel (fully-configured scenario)", () => {
  it("renders the active profile card", async () => {
    renderWithMockIPC("fully-configured", <LauncherPanel />);

    const activeId = await cliStateMock.get_active("claude");
    expect(activeId).not.toBeNull();

    await waitFor(() => {
      expect(
        screen.getByTestId("launcher-active-profile-card"),
      ).toBeInTheDocument();
    });
    expect(screen.getByTestId("launcher-profile-name")).toBeInTheDocument();
    expect(
      screen.getByTestId("launcher-profile-mcp-count"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("launcher-profile-skills-count"),
    ).toBeInTheDocument();
  });

  it("auto-selects the recommended terminal (wt) on Windows scenario", async () => {
    renderWithMockIPC("fully-configured", <LauncherPanel />);
    await waitFor(() => {
      expect(
        screen.getByTestId("launcher-terminal-recommended-wt"),
      ).toBeInTheDocument();
    });
  });

  it("enables the launch button when profile and terminal are ready, and start_cli succeeds", async () => {
    const startSpy = vi.spyOn(launcherMock, "start_cli");
    renderWithMockIPC("fully-configured", <LauncherPanel />);

    await waitFor(() => {
      expect(screen.getByTestId("launcher-launch-button")).not.toBeDisabled();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("launcher-launch-button"));
    });

    await waitFor(() => {
      expect(startSpy).toHaveBeenCalled();
    });

    const calledWith = startSpy.mock.calls[0]![0];
    expect(calledWith.target_cli).toBe("claude");
    expect(calledWith.terminal_id).toBe("wt");
    expect(calledWith.profile_id).toBeTruthy();
  });
});

describe("LauncherPanel (all-installed-no-profile scenario)", () => {
  it("shows the empty-profile state with create-profile link and disabled launch button", async () => {
    const onNavigateProfileManager = vi.fn();
    renderWithMockIPC(
      "all-installed-no-profile",
      <LauncherPanel onNavigateProfileManager={onNavigateProfileManager} />,
    );

    await waitFor(() => {
      expect(
        screen.getByTestId("launcher-active-profile-empty"),
      ).toBeInTheDocument();
    });

    expect(
      screen.getByTestId("launcher-active-profile-create-link"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("launcher-launch-button")).toBeDisabled();

    fireEvent.click(screen.getByTestId("launcher-active-profile-create-link"));
    expect(onNavigateProfileManager).toHaveBeenCalled();
  });
});

describe("LauncherPanel (error path)", () => {
  it("shows the error dialog with cli_missing message + fix link when start_cli returns CLI_MISSING", async () => {
    const onNavigateInstaller = vi.fn();

    // Force start_cli to return a CLI_MISSING typed error
    const originalStartCli = launcherMock.start_cli.bind(launcherMock);
    const spy = vi
      .spyOn(launcherMock, "start_cli")
      .mockImplementation(async (args) => {
        return {
          success: false,
          profileId: args.profile_id,
          targetCli: args.target_cli,
          terminalId: args.terminal_id ?? "",
          cwd: args.cwd ?? "C:\\Users\\you\\cc-launcher-projects\\x",
          launchedAt: new Date().toISOString(),
          error: {
            code: "CLI_MISSING",
            message: {
              zh: "未检测到 Claude Code CLI",
              en: "Claude Code CLI not detected",
              ja: "Claude Code CLI が見つかりません",
            },
            retryable: false,
          },
        };
      });

    renderWithMockIPC(
      "fully-configured",
      <LauncherPanel onNavigateInstaller={onNavigateInstaller} />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("launcher-launch-button")).not.toBeDisabled();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("launcher-launch-button"));
    });

    await waitFor(() => {
      expect(screen.getByTestId("launcher-error-dialog")).toBeInTheDocument();
    });

    expect(screen.getByTestId("launcher-error-message").textContent).toContain(
      "launcher.error.cliMissing",
    );
    expect(screen.getByTestId("launcher-error-fix-link")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("launcher-error-fix-link"));
    expect(onNavigateInstaller).toHaveBeenCalled();

    spy.mockRestore();
    // restore original behavior in case the test re-runs
    void originalStartCli;
  });
});
