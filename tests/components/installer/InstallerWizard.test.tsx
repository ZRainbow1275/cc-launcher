import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { InstallerWizard } from "@/components/installer/InstallerWizard";
import {
  installer,
  renderWithMockIPC,
  settings,
  teardownMockIPC,
} from "@/lib/api/mock";

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("InstallerWizard — happy path", () => {
  it("advances 4 steps and lands on card view with both CLIs installed (fully-configured)", async () => {
    renderWithMockIPC("fully-configured", <InstallerWizard />);

    // Step 1
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-1")).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-1-ok")).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-next"));
    });

    // Step 2 - node ready
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-2")).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-2-ready")).toBeInTheDocument();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-next"));
    });

    // Step 3
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-3")).toBeInTheDocument();
    });
    expect(
      screen.getByTestId("installer-step-3-installed-claude"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("installer-step-3-installed-codex"),
    ).toBeInTheDocument();

    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-next"));
    });

    // Step 4 — both already installed so no install needed; goes to card view
    await waitFor(() => {
      expect(
        screen.getByTestId("installer-card-view") ||
          screen.getByTestId("installer-step-4"),
      ).toBeInTheDocument();
    });

    await waitFor(
      () => {
        expect(screen.getByTestId("installer-card-view")).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    expect(screen.getByTestId("cli-card-claude")).toHaveAttribute(
      "data-installed",
      "true",
    );
    expect(screen.getByTestId("cli-card-codex")).toHaveAttribute(
      "data-installed",
      "true",
    );
  });
});

describe("InstallerWizard — new-user red light", () => {
  it("blocks step 1 next button when red lights exist", async () => {
    renderWithMockIPC("new-user", <InstallerWizard />);

    await waitFor(() => {
      expect(screen.getByTestId("installer-step-1")).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(
        screen.getByTestId("installer-step-1-red-block"),
      ).toBeInTheDocument();
    });

    const nextBtn = screen.getByTestId("installer-next") as HTMLButtonElement;
    expect(nextBtn.disabled).toBe(true);

    // SystemCheckSummary still openable
    expect(screen.getByTestId("system-check-summary")).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("system-check-summary"));
    await waitFor(() => {
      expect(
        screen.getByTestId("system-check-summary-dialog"),
      ).toBeInTheDocument();
    });
  });
});

describe("InstallerWizard — network-failure step 4", () => {
  it("shows the all-failed branch on registry probe when network is down", async () => {
    renderWithMockIPC("network-failure", <InstallerWizard initialStep={4} />);

    await waitFor(
      () => {
        expect(
          screen.getByTestId("registry-picker-all-failed"),
        ).toBeInTheDocument();
      },
      { timeout: 3000 },
    );
    expect(screen.getByTestId("registry-picker-retry")).toBeInTheDocument();
  });
});

describe("InstallerWizard — step 3 select toggle", () => {
  it("disables next when all CLIs are deselected and re-enables on re-check", async () => {
    const user = userEvent.setup();
    renderWithMockIPC(
      "all-installed-no-profile",
      <InstallerWizard initialStep={3} />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("installer-step-3")).toBeInTheDocument();
    });

    const claudeBox = screen.getByTestId("installer-step-3-checkbox-claude");
    const codexBox = screen.getByTestId("installer-step-3-checkbox-codex");

    await user.click(claudeBox);
    await user.click(codexBox);

    await waitFor(() => {
      expect(screen.getByTestId("installer-step-3-empty")).toBeInTheDocument();
    });

    const next = screen.getByTestId("installer-next") as HTMLButtonElement;
    expect(next.disabled).toBe(true);

    await user.click(claudeBox);
    await waitFor(() => {
      expect(screen.queryByTestId("installer-step-3-empty")).toBeNull();
    });
  });
});

describe("InstallerWizard — card view uninstall confirm", () => {
  it("opens a destructive confirm dialog when uninstall is clicked", async () => {
    renderWithMockIPC("fully-configured", <InstallerWizard />);

    // jump straight through to card view by clicking next 3 times
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-1-ok")).toBeInTheDocument();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-next"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-2-ready")).toBeInTheDocument();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-next"));
    });
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-3")).toBeInTheDocument();
    });
    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-next"));
    });

    await waitFor(
      () => {
        expect(screen.getByTestId("installer-card-view")).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    fireEvent.click(screen.getByTestId("cli-card-claude-uninstall"));

    await waitFor(() => {
      expect(screen.getAllByRole("dialog").length).toBeGreaterThan(0);
    });
  });
});

describe("InstallerWizard — install single CLI happy path", () => {
  it("streams an install for a missing codex CLI and reaches success block", async () => {
    renderWithMockIPC(
      "claude-installed-codex-missing",
      <InstallerWizard initialStep={4} />,
    );

    // wait for registry picker to come up
    await waitFor(
      () => {
        expect(screen.getByTestId("registry-picker")).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    // npmmirror is auto-fastest
    await waitFor(() => {
      expect(
        screen.getByTestId("registry-row-npmmirror-fastest"),
      ).toBeInTheDocument();
    });

    // click start button
    await waitFor(() => {
      expect(screen.getByTestId("installer-step-4-start")).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-step-4-start"));
    });

    // Should observe a streaming install for codex (codex is the missing one)
    await waitFor(
      () => {
        expect(
          screen.getByTestId("install-progress-codex"),
        ).toBeInTheDocument();
      },
      { timeout: 3000 },
    );

    await waitFor(
      () => {
        expect(
          screen.getByTestId("install-progress-codex-success"),
        ).toBeInTheDocument();
      },
      { timeout: 5000 },
    );

    // verify mock state reflects install
    const status = await installer.detect_cli("codex");
    expect(status.installed).toBe(true);
  });

  it("saves a private npm registry and uses it for the install stream", async () => {
    const user = userEvent.setup();
    renderWithMockIPC(
      "claude-installed-codex-missing",
      <InstallerWizard initialStep={4} />,
    );

    await waitFor(() => {
      expect(
        screen.getByTestId("installer-source-settings"),
      ).toBeInTheDocument();
    });
    await waitFor(() => {
      expect(
        (screen.getByTestId("installer-source-save") as HTMLButtonElement)
          .disabled,
      ).toBe(false);
    });

    await user.type(
      screen.getByTestId("installer-source-npm-registry"),
      "https://vps.example.com/npm",
    );
    await user.click(screen.getByTestId("installer-source-save"));

    await waitFor(async () => {
      const saved = await settings.get_installer_source_config();
      expect(saved.npmRegistry).toBe("https://vps.example.com/npm");
    });

    await waitFor(() => {
      expect(screen.getByTestId("installer-step-4-start")).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("installer-step-4-start"));
    });

    await waitFor(
      () => {
        expect(
          screen.getByTestId("install-progress-codex-registry"),
        ).toHaveTextContent("https://vps.example.com/npm");
      },
      { timeout: 5000 },
    );

    await waitFor(
      () => {
        expect(
          screen.getByTestId("install-progress-codex-success"),
        ).toBeInTheDocument();
      },
      { timeout: 5000 },
    );
  });
});
