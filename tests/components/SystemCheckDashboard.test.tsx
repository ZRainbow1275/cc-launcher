import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { screen, within, waitFor, fireEvent } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { SystemCheckDashboard } from "@/components/system-check/SystemCheckDashboard";
import { SystemCheckSummary } from "@/components/system-check/SystemCheckSummary";
import { renderWithMockIPC, teardownMockIPC } from "@/lib/api/mock";

afterEach(() => {
  teardownMockIPC();
});

describe("SystemCheckDashboard", () => {
  beforeEach(() => {
    // no-op; renderWithMockIPC sets scenario
  });

  it("renders all five groups when probe report is loaded (new-user scenario)", async () => {
    renderWithMockIPC("new-user", <SystemCheckDashboard />);

    await waitFor(() => {
      expect(screen.getByTestId("system-check-dashboard")).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(screen.getByTestId("system-check-groups")).toBeInTheDocument();
    });

    const groups = await screen.findAllByTestId(/^probe-group-/);
    expect(groups.length).toBeGreaterThanOrEqual(5);
  });

  it("shows red status overall for new-user scenario with fixable items", async () => {
    renderWithMockIPC("new-user", <SystemCheckDashboard />);

    await waitFor(() => {
      const node = screen.getByTestId("probe-item-node");
      expect(node).toHaveAttribute("data-status", "missing");
    });

    const nodeCard = screen.getByTestId("probe-item-node");
    expect(
      within(nodeCard).getByTestId("probe-item-node-fix"),
    ).toBeInTheDocument();
  });

  it("shows all green overall for fully-configured scenario", async () => {
    renderWithMockIPC("fully-configured", <SystemCheckDashboard />);

    await waitFor(() => {
      const node = screen.getByTestId("probe-item-node");
      expect(node).toHaveAttribute("data-status", "green");
    });

    expect(screen.queryByTestId("probe-item-node-fix")).not.toBeInTheDocument();
  });

  it("flags the network item red in network-failure scenario", async () => {
    renderWithMockIPC("network-failure", <SystemCheckDashboard />);

    await waitFor(() => {
      const net = screen.getByTestId("probe-item-network");
      expect(net).toHaveAttribute("data-status", "red");
    });
  });

  it("re-probes when the reprobe button is clicked", async () => {
    const user = userEvent.setup();
    const { queryClient } = renderWithMockIPC(
      "new-user",
      <SystemCheckDashboard />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("probe-item-node")).toBeInTheDocument();
    });

    const reprobe = screen.getByTestId("system-check-reprobe");
    await user.click(reprobe);

    await waitFor(() => {
      const state = queryClient.getQueryState(["system_probe", "run"]);
      expect(state?.dataUpdateCount ?? 0).toBeGreaterThanOrEqual(2);
    });
  });

  it("opens the FixActionDialog when a fix button is clicked and streams progress", async () => {
    const user = userEvent.setup();
    renderWithMockIPC("new-user", <SystemCheckDashboard />);

    await waitFor(() => {
      expect(screen.getByTestId("probe-item-node-fix")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("probe-item-node-fix"));

    await waitFor(() => {
      expect(screen.getByTestId("fix-action-dialog")).toBeInTheDocument();
    });

    await waitFor(
      () => {
        const bar = screen.getByTestId("fix-progress-bar");
        expect(bar).toHaveAttribute("aria-valuenow", "100");
      },
      { timeout: 5000 },
    );

    const closeBtn = screen.getByTestId("fix-dialog-action");
    fireEvent.click(closeBtn);

    await waitFor(() => {
      expect(screen.queryByTestId("fix-action-dialog")).not.toBeInTheDocument();
    });
  });

  it("queues remaining items when fix-all is clicked", async () => {
    const user = userEvent.setup();
    renderWithMockIPC("new-user", <SystemCheckDashboard />);

    await waitFor(() => {
      expect(screen.getByTestId("probe-item-node-fix")).toBeInTheDocument();
    });

    const fixAllBtn = screen.getByTestId("system-check-fix-all");
    await waitFor(() => {
      expect(fixAllBtn).not.toBeDisabled();
    });

    await user.click(fixAllBtn);

    await waitFor(() => {
      expect(screen.getByTestId("fix-action-dialog")).toBeInTheDocument();
    });
  });
});

describe("SystemCheckSummary", () => {
  it("renders compact summary with counts", async () => {
    renderWithMockIPC("new-user", <SystemCheckSummary />);

    const summary = await screen.findByTestId("system-check-summary");
    expect(summary).toBeInTheDocument();

    await waitFor(() => {
      expect(summary).toHaveAttribute("data-status", "red");
    });
  });

  it("opens dashboard dialog when clicked", async () => {
    const user = userEvent.setup();
    renderWithMockIPC("new-user", <SystemCheckSummary />);

    const summary = await screen.findByTestId("system-check-summary");
    await user.click(summary);

    await waitFor(() => {
      expect(
        screen.getByTestId("system-check-summary-dialog"),
      ).toBeInTheDocument();
    });
  });

  it("calls onOpenDashboard if provided instead of dialog", async () => {
    const user = userEvent.setup();
    let opened = false;
    renderWithMockIPC(
      "new-user",
      <SystemCheckSummary
        onOpenDashboard={() => {
          opened = true;
        }}
      />,
    );

    const summary = await screen.findByTestId("system-check-summary");
    await user.click(summary);
    expect(opened).toBe(true);
    expect(
      screen.queryByTestId("system-check-summary-dialog"),
    ).not.toBeInTheDocument();
  });
});
