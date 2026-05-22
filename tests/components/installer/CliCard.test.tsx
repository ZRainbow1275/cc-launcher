import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { CliCard } from "@/components/installer/CliCard";
import type { CliInstallStatus } from "@/lib/api/contracts";

function status(installed: boolean): CliInstallStatus {
  return installed
    ? {
        cli: "claude",
        installed: true,
        version: "2.1.148",
        path: "C:\\Users\\you\\.cc-switch\\runtime\\node_modules\\.bin\\claude.cmd",
        lastChecked: new Date().toISOString(),
      }
    : {
        cli: "claude",
        installed: false,
        lastChecked: new Date().toISOString(),
      };
}

describe("CliCard", () => {
  it("shows version, profile count, uninstall and view-profiles buttons when installed", () => {
    render(
      <CliCard
        cli="claude"
        status={status(true)}
        profileCount={3}
        onUninstall={vi.fn()}
        onViewProfiles={vi.fn()}
      />,
    );

    expect(screen.getByTestId("cli-card-claude")).toHaveAttribute(
      "data-installed",
      "true",
    );
    expect(screen.getByTestId("cli-card-claude-version").textContent).toBe(
      "2.1.148",
    );
    expect(
      screen.getByTestId("cli-card-claude-profile-count"),
    ).toBeInTheDocument();
    expect(screen.getByTestId("cli-card-claude-uninstall")).toBeInTheDocument();
    expect(
      screen.getByTestId("cli-card-claude-view-profiles"),
    ).toBeInTheDocument();
  });

  it("shows install hint when not installed and optional install button", () => {
    const onInstall = vi.fn();
    render(
      <CliCard
        cli="claude"
        status={status(false)}
        profileCount={0}
        onUninstall={vi.fn()}
        onViewProfiles={vi.fn()}
        onInstall={onInstall}
      />,
    );

    expect(screen.getByTestId("cli-card-claude")).toHaveAttribute(
      "data-installed",
      "false",
    );
    expect(screen.queryByTestId("cli-card-claude-version")).toBeNull();
    expect(screen.getByTestId("cli-card-claude-install")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("cli-card-claude-install"));
    expect(onInstall).toHaveBeenCalledTimes(1);
  });

  it("invokes onUninstall when the uninstall button is clicked", () => {
    const onUninstall = vi.fn();
    render(
      <CliCard
        cli="claude"
        status={status(true)}
        profileCount={1}
        onUninstall={onUninstall}
        onViewProfiles={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("cli-card-claude-uninstall"));
    expect(onUninstall).toHaveBeenCalledTimes(1);
  });
});
