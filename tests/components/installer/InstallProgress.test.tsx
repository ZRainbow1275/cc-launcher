import { fireEvent, render, screen } from "@testing-library/react";
import i18n from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";

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
  afterEach(async () => {
    await i18n.changeLanguage("zh");
  });

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

  it("renders localized backend progress messages using the active language", async () => {
    await i18n.changeLanguage("en");
    const events: InstallProgressEvent[] = [
      {
        phase: "installing-cli",
        message: {
          zh: "正在安装 CLI...",
          en: "Installing CLI...",
          ja: "CLI をインストール中...",
        },
        percent: 60,
      },
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

    expect(screen.getByText("Installing CLI...")).toBeInTheDocument();
    expect(screen.queryByText("正在安装 CLI...")).toBeNull();
  });

  it("shows the backend error code, cause, and attempted registry on failure", () => {
    const events: InstallProgressEvent[] = [
      {
        phase: "installing-cli",
        message: { zh: "msg", en: "msg", ja: "msg" },
        percent: 60,
        registry: "https://vps.example.com/npm",
      },
      {
        phase: "failed",
        message: { zh: "failed", en: "failed", ja: "failed" },
        percent: 0,
        registry: "https://vps.example.com/npm",
        error: {
          code: "CLI_INSTALL_FAILED",
          message: {
            zh: "CLI 安装失败",
            en: "CLI installation failed",
            ja: "CLI のインストールに失敗しました",
          },
          cause: "npm install failed (exit 1): fetch failed",
          retryable: true,
        },
      },
    ];

    render(
      <InstallProgress
        cli="claude"
        events={events}
        isStreaming={false}
        onCancel={vi.fn()}
        onRetrySameMirror={vi.fn()}
        onRetryDifferentMirror={vi.fn()}
      />,
    );

    expect(
      screen.getByTestId("install-progress-claude-error-code"),
    ).toHaveTextContent("CLI_INSTALL_FAILED");
    expect(
      screen.getByTestId("install-progress-claude-error-cause"),
    ).toHaveTextContent("fetch failed");
    expect(
      screen.getByTestId("install-progress-claude-registry"),
    ).toHaveTextContent("https://vps.example.com/npm");
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
