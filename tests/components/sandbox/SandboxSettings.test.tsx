import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { SandboxSettings } from "@/components/sandbox/SandboxSettings";
import {
  clearFailures,
  loadScenario,
  resetState,
  sandboxMock,
  setMockDelay,
  setMockScenario,
} from "@/lib/api/mock";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}|${JSON.stringify(opts)}` : key,
    i18n: { language: "en" },
  }),
}));

const toastSuccessMock = vi.fn();
const toastErrorMock = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccessMock(...args),
    error: (...args: unknown[]) => toastErrorMock(...args),
  },
}));

function renderWithClient(ui: ReactNode) {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  });
  return {
    client,
    ...render(<QueryClientProvider client={client}>{ui}</QueryClientProvider>),
  };
}

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("fully-configured");
  setMockScenario("fully-configured");
  toastSuccessMock.mockReset();
  toastErrorMock.mockReset();
});

afterEach(() => {
  resetState();
});

describe("SandboxSettings", () => {
  it("renders L1 rules, L2 redlines, and sandbox level", async () => {
    renderWithClient(<SandboxSettings />);

    await waitFor(() => {
      expect(screen.getByTestId("l1-rule-L1.rm_arbitrary")).toBeTruthy();
    });

    expect(screen.getByTestId("l1-rule-L1.sudo_runas")).toBeTruthy();
    expect(
      screen.getByTestId("l1-rule-L1.claude_skip_permissions"),
    ).toBeTruthy();

    await waitFor(() => {
      expect(screen.getByTestId("l2-redline-disk_wipe.rm_root")).toBeTruthy();
    });

    expect(screen.getByTestId("sandbox-level-strict")).toBeTruthy();
    expect(screen.getByTestId("sandbox-level-medium")).toBeTruthy();
  });

  it("shows a confirm dialog when changing sandbox level and applies the change after confirm", async () => {
    renderWithClient(<SandboxSettings />);

    const strictTrigger = await screen.findByTestId("sandbox-level-strict");
    await waitFor(() => {
      expect(strictTrigger.getAttribute("data-state")).toBe("inactive");
    });

    fireEvent.mouseDown(strictTrigger, { button: 0 });

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("sandbox.level.confirmTitle")).toBeTruthy();

    const confirmBtn = within(dialog).getByRole("button", {
      name: /common\.confirm|sandbox\.confirm/,
    });
    fireEvent.click(confirmBtn);

    await waitFor(async () => {
      const lvl = await sandboxMock.get_sandbox_level();
      expect(lvl).toBe("strict");
    });

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
  });

  it("toggling an L1 switch off calls set_l1_rule and updates state", async () => {
    renderWithClient(<SandboxSettings />);
    const sw = await screen.findByTestId("l1-rule-L1.rm_arbitrary-switch");

    await waitFor(() => {
      expect(sw.getAttribute("aria-checked")).toBe("true");
    });

    fireEvent.click(sw);

    await waitFor(async () => {
      const rules = await sandboxMock.get_l1_rules();
      const r = rules.find((x) => x.id === "L1.rm_arbitrary");
      expect(r?.enabled).toBe(false);
    });
  });

  it("L2 redlines render with permanent-lock badge", async () => {
    renderWithClient(<SandboxSettings />);

    await waitFor(() => {
      expect(screen.getByTestId("l2-redline-disk_wipe.rm_root")).toBeTruthy();
    });

    const badges = screen.getAllByText("sandbox.l2.permanentLockBadge");
    expect(badges.length).toBeGreaterThan(10);
  });

  it("clicking 'unlock this rule' opens the 3-step dialog and unlocks via the mock", async () => {
    renderWithClient(<SandboxSettings />);

    const unlockBtn = await screen.findByTestId("l1-rule-L1.sudo_runas-unlock");
    fireEvent.click(unlockBtn);

    expect(await screen.findByTestId("dangerous-confirm-step-1")).toBeTruthy();
    fireEvent.click(screen.getByTestId("dangerous-confirm-step1-continue"));

    expect(screen.getByTestId("dangerous-confirm-step-2")).toBeTruthy();
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-3"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-step2-next"));

    const input = screen.getByTestId("dangerous-confirm-keyword-input");
    fireEvent.change(input, { target: { value: "I UNDERSTAND" } });

    fireEvent.click(screen.getByTestId("dangerous-confirm-submit"));

    await waitFor(async () => {
      const rules = await sandboxMock.get_l1_rules();
      const r = rules.find((x) => x.id === "L1.sudo_runas");
      expect(r?.unlockedUntil).toBeTruthy();
    });
  });

  it("rejects unlock when keyword doesn't match (submit stays disabled)", async () => {
    renderWithClient(<SandboxSettings />);

    const unlockBtn = await screen.findByTestId("l1-rule-L1.sudo_runas-unlock");
    fireEvent.click(unlockBtn);

    fireEvent.click(
      await screen.findByTestId("dangerous-confirm-step1-continue"),
    );
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-1"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-2"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-risk-3"));
    fireEvent.click(screen.getByTestId("dangerous-confirm-step2-next"));

    fireEvent.change(screen.getByTestId("dangerous-confirm-keyword-input"), {
      target: { value: "NOT THE KEYWORD" },
    });

    expect(
      screen.getByTestId("dangerous-confirm-submit").hasAttribute("disabled"),
    ).toBe(true);
  });

  it("does not render an unlock button for permanently locked rules (unlockable=false)", async () => {
    renderWithClient(<SandboxSettings />);

    await waitFor(() => {
      expect(
        screen.getByTestId("l1-rule-L1.claude_skip_permissions"),
      ).toBeTruthy();
    });

    expect(
      screen.queryByTestId("l1-rule-L1.claude_skip_permissions-unlock"),
    ).toBeNull();
    expect(
      screen.getByTestId("l1-rule-L1.claude_skip_permissions-status-permanent"),
    ).toBeTruthy();
  });

  it("renders a platform alert", async () => {
    renderWithClient(<SandboxSettings />);
    const alert = await screen.findByTestId("sandbox-platform-alert");
    expect(alert).toBeTruthy();
  });
});
