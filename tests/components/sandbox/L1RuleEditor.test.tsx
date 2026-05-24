import { render, screen, fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { L1RuleEditor } from "@/components/sandbox/L1RuleEditor";
import type { L1Rule } from "@/lib/api/contracts";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) => {
      if (opts && "hours" in opts) {
        return `${key}|hours=${String(opts.hours)}`;
      }
      return key;
    },
  }),
}));

function baseRule(overrides: Partial<L1Rule> = {}): L1Rule {
  return {
    id: "L1.rm_arbitrary",
    category: "DangerousFilesystem",
    pattern: "rm -rf",
    titleKey: "sandbox.l1.rm_arbitrary.title",
    descriptionKey: "sandbox.l1.rm_arbitrary.desc",
    enabled: true,
    unlockable: true,
    unlockedUntil: null,
    updatedAt: new Date().toISOString(),
    ...overrides,
  };
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-05-22T10:00:00.000Z"));
});

afterEach(() => {
  vi.useRealTimers();
});

describe("L1RuleEditor", () => {
  it("renders title and description for an enabled unlockable rule", () => {
    render(
      <L1RuleEditor
        rule={baseRule()}
        onToggle={vi.fn()}
        onRequestUnlock={vi.fn()}
        onRelock={vi.fn()}
      />,
    );

    expect(screen.getByText("sandbox.l1.rm_arbitrary.title")).toBeTruthy();
    expect(screen.getByText("sandbox.l1.rm_arbitrary.desc")).toBeTruthy();
    expect(
      screen.getByTestId("l1-rule-L1.rm_arbitrary-status-enabled"),
    ).toBeTruthy();
    expect(screen.getByTestId("l1-rule-L1.rm_arbitrary-unlock")).toBeTruthy();
  });

  it("routes a switch click on a locked-on rule through onRequestUnlock (A1 fix)", () => {
    const onToggle = vi.fn();
    const onRequestUnlock = vi.fn();
    render(
      <L1RuleEditor
        rule={baseRule()}
        onToggle={onToggle}
        onRequestUnlock={onRequestUnlock}
        onRelock={vi.fn()}
      />,
    );

    const sw = screen.getByTestId("l1-rule-L1.rm_arbitrary-switch");
    fireEvent.click(sw);
    expect(onToggle).not.toHaveBeenCalled();
    expect(onRequestUnlock).toHaveBeenCalledTimes(1);
  });

  it("invokes onRequestUnlock when the unlock button is clicked", () => {
    const onRequestUnlock = vi.fn();
    render(
      <L1RuleEditor
        rule={baseRule()}
        onToggle={vi.fn()}
        onRequestUnlock={onRequestUnlock}
        onRelock={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByTestId("l1-rule-L1.rm_arbitrary-unlock"));
    expect(onRequestUnlock).toHaveBeenCalledTimes(1);
  });

  it("shows permanent-lock badge and hides toggle interactivity when unlockable=false", () => {
    render(
      <L1RuleEditor
        rule={baseRule({
          id: "L1.claude_skip_permissions",
          unlockable: false,
          enabled: true,
        })}
        onToggle={vi.fn()}
        onRequestUnlock={vi.fn()}
        onRelock={vi.fn()}
      />,
    );

    expect(
      screen.getByTestId("l1-rule-L1.claude_skip_permissions-status-permanent"),
    ).toBeTruthy();
    const sw = screen.getByTestId("l1-rule-L1.claude_skip_permissions-switch");
    expect(sw.getAttribute("data-disabled")).not.toBeNull();
    expect(
      screen.queryByTestId("l1-rule-L1.claude_skip_permissions-unlock"),
    ).toBeNull();
  });

  it("renders unlocked status with remaining hours when unlockedUntil is in the future", () => {
    const until = new Date(Date.now() + 23 * 60 * 60 * 1000).toISOString();
    render(
      <L1RuleEditor
        rule={baseRule({ enabled: false, unlockedUntil: until })}
        onToggle={vi.fn()}
        onRequestUnlock={vi.fn()}
        onRelock={vi.fn()}
      />,
    );

    const badge = screen.getByTestId("l1-rule-L1.rm_arbitrary-status-unlocked");
    expect(badge.textContent).toContain("hours=23");
    expect(screen.getByTestId("l1-rule-L1.rm_arbitrary-relock")).toBeTruthy();
  });

  it("auto-locks once unlockedUntil has elapsed (countdown via setInterval)", () => {
    const until = new Date(Date.now() + 60 * 1000).toISOString();
    const { rerender } = render(
      <L1RuleEditor
        rule={baseRule({ enabled: false, unlockedUntil: until })}
        onToggle={vi.fn()}
        onRequestUnlock={vi.fn()}
        onRelock={vi.fn()}
      />,
    );

    expect(
      screen.getByTestId("l1-rule-L1.rm_arbitrary-status-unlocked"),
    ).toBeTruthy();

    vi.setSystemTime(new Date(Date.now() + 2 * 60 * 1000));
    vi.advanceTimersByTime(60 * 1000);

    rerender(
      <L1RuleEditor
        rule={baseRule({ enabled: false, unlockedUntil: until })}
        onToggle={vi.fn()}
        onRequestUnlock={vi.fn()}
        onRelock={vi.fn()}
      />,
    );

    expect(
      screen.queryByTestId("l1-rule-L1.rm_arbitrary-status-unlocked"),
    ).toBeNull();
    expect(
      screen.getByTestId("l1-rule-L1.rm_arbitrary-status-disabled"),
    ).toBeTruthy();
  });
});
