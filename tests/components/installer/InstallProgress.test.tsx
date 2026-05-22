import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { InstallProgress } from "@/components/installer/InstallProgress";
import type { InstallProgress as InstallProgressEvent } from "@/lib/api/contracts";

function evt(
  phase: InstallProgressEvent["phase"],
  percent?: number,
): InstallProgressEvent {
  return {
    phase,
    message: { zh: "msg", en: "msg", ja: "msg" },
    percent,
  };
}

describe("InstallProgress", () => {
  it("renders streaming phases in order and shows the latest phase + bar", () => {
    const events: InstallProgressEvent[] = [
      evt("probing-registry", 5),
      evt("installing-cli", 60),
      evt("validating", 90),
    ];

    render(
      <InstallProgress
        cli="claude"
        events={events}
        isStreaming={true}
        onCancel={vi.fn()}
        onRetrySameMirror={vi.fn()}
        onRetryDifferentMirror={vi.fn()}
      />,
    );

    const phase = screen.getByTestId("install-progress-claude-phase");
    expect(phase.getAttribute("data-phase")).toBe("validating");
    expect(screen.getByTestId("install-progress-claude-bar")).toHaveStyle({
      width: "90%",
    });
    expect(
      screen.getByTestId("install-progress-claude-bytes"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("install-progress-claude-speed"),
    ).toBeInTheDocument();
  });

  it("invokes onCancel when the cancel button is clicked", () => {
    const onCancel = vi.fn();
    const events: InstallProgressEvent[] = [evt("installing-cli", 50)];

    render(
      <InstallProgress
        cli="codex"
        events={events}
        isStreaming={true}
        onCancel={onCancel}
        onRetrySameMirror={vi.fn()}
        onRetryDifferentMirror={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("install-progress-codex-cancel"));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("shows cleaned-up note plus same-mirror and different-mirror retry buttons on error phase", () => {
    const onRetrySame = vi.fn();
    const onRetryDifferent = vi.fn();
    const events: InstallProgressEvent[] = [
      evt("installing-cli", 40),
      evt("failed", 40),
    ];

    render(
      <InstallProgress
        cli="claude"
        events={events}
        isStreaming={false}
        onCancel={vi.fn()}
        onRetrySameMirror={onRetrySame}
        onRetryDifferentMirror={onRetryDifferent}
      />,
    );

    expect(
      screen.getByTestId("install-progress-claude-error"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("install-progress-claude-cleaned"),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("install-progress-claude-retry-same"));
    expect(onRetrySame).toHaveBeenCalledTimes(1);

    fireEvent.click(
      screen.getByTestId("install-progress-claude-retry-different"),
    );
    expect(onRetryDifferent).toHaveBeenCalledTimes(1);
  });

  it("shows a success block with version when phase is completed", () => {
    const events: InstallProgressEvent[] = [
      evt("installing-cli", 60),
      evt("completed", 100),
    ];

    render(
      <InstallProgress
        cli="codex"
        events={events}
        isStreaming={false}
        installedVersion="0.133.0"
        onCancel={vi.fn()}
        onRetrySameMirror={vi.fn()}
        onRetryDifferentMirror={vi.fn()}
      />,
    );

    expect(
      screen.getByTestId("install-progress-codex-success"),
    ).toBeInTheDocument();
  });
});
