import { render, screen, fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SafetySummary } from "@/components/launcher/SafetySummary";
import type { SafetySummary as SafetySummaryData } from "@/lib/api/mock/launcher";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}|${JSON.stringify(opts)}` : key,
    i18n: { language: "en" },
  }),
}));

beforeEach(() => {});

afterEach(() => {
  vi.clearAllMocks();
});

const claudeData: SafetySummaryData = {
  profileId: "claude-x",
  targetCli: "claude",
  flags: [
    "--permission-mode default",
    "--strict-mcp-config",
    "--add-dir C:\\Users\\you\\cc-launcher-projects\\claude-x",
    "--append-system-prompt-file <safe-prompt-path>",
  ],
  cwd: "C:\\Users\\you\\cc-launcher-projects\\claude-x",
  cwdDisplay: "~/cc-launcher-projects/claude-x",
  l1ActiveCount: 7,
  l2RedlineCount: 16,
};

const codexData: SafetySummaryData = {
  profileId: "codex-x",
  targetCli: "codex",
  flags: ["--profile work-codex"],
  cwd: "C:\\Users\\you\\cc-launcher-projects\\codex-x",
  cwdDisplay: "~/cc-launcher-projects/codex-x",
  l1ActiveCount: 5,
  l2RedlineCount: 16,
};

describe("SafetySummary (Claude)", () => {
  it("renders Claude flags including --strict-mcp-config / --permission-mode default / --add-dir", () => {
    render(
      <SafetySummary
        targetCli="claude"
        data={claudeData}
        isLoading={false}
        hasActiveProfile={true}
      />,
    );
    // open the collapsible first
    fireEvent.click(screen.getByTestId("launcher-safety-toggle"));
    const flags = screen.getByTestId("launcher-safety-flags");
    expect(flags.textContent).toContain("--permission-mode default");
    expect(flags.textContent).toContain("--strict-mcp-config");
    expect(flags.textContent).toContain("--add-dir");
  });

  it("renders L1 and L2 counts numerically", () => {
    render(
      <SafetySummary
        targetCli="claude"
        data={claudeData}
        isLoading={false}
        hasActiveProfile={true}
      />,
    );
    fireEvent.click(screen.getByTestId("launcher-safety-toggle"));
    expect(
      screen.getByTestId("launcher-safety-l1-count").textContent,
    ).toContain('"count":7');
    expect(
      screen.getByTestId("launcher-safety-l2-count").textContent,
    ).toContain('"count":16');
  });

  it("defaults to collapsed (content data-state=closed)", () => {
    render(
      <SafetySummary
        targetCli="claude"
        data={claudeData}
        isLoading={false}
        hasActiveProfile={true}
      />,
    );
    const trigger = screen.getByTestId("launcher-safety-toggle");
    expect(trigger.getAttribute("data-state")).toBe("closed");
  });

  it("expands when clicked", () => {
    render(
      <SafetySummary
        targetCli="claude"
        data={claudeData}
        isLoading={false}
        hasActiveProfile={true}
      />,
    );
    const trigger = screen.getByTestId("launcher-safety-toggle");
    fireEvent.click(trigger);
    expect(trigger.getAttribute("data-state")).toBe("open");
  });
});

describe("SafetySummary (Codex)", () => {
  it("renders Codex flags including --profile <name>", () => {
    render(
      <SafetySummary
        targetCli="codex"
        data={codexData}
        isLoading={false}
        hasActiveProfile={true}
      />,
    );
    fireEvent.click(screen.getByTestId("launcher-safety-toggle"));
    expect(screen.getByTestId("launcher-safety-flags").textContent).toContain(
      "--profile work-codex",
    );
  });
});
