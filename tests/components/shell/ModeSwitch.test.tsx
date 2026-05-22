import {
  render,
  screen,
  waitFor,
  within,
  fireEvent,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { ModeSwitch } from "@/components/shell/ModeSwitch";
import {
  clearFailures,
  loadScenario,
  resetState,
  setMockDelay,
  setMockScenario,
  settingsMock,
} from "@/lib/api/mock";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) =>
      options?.defaultValue ?? key,
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
  loadScenario("new-user");
  setMockScenario("new-user");
  toastSuccessMock.mockReset();
  toastErrorMock.mockReset();
});

afterEach(() => {
  resetState();
});

describe("ModeSwitch", () => {
  it("renders with the current mode reflected by the switch", async () => {
    renderWithClient(<ModeSwitch />);
    const toggle = await screen.findByRole("switch");
    await waitFor(() =>
      expect(toggle.getAttribute("aria-checked")).toBe("false"),
    );
  });

  it("shows confirmation dialog when toggling novice -> expert and confirms switch", async () => {
    const user = userEvent.setup();
    renderWithClient(<ModeSwitch />);
    const toggle = await screen.findByRole("switch");
    await waitFor(() =>
      expect(toggle.getAttribute("aria-checked")).toBe("false"),
    );

    await user.click(toggle);

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByText(/进入专家模式将解锁所有高级面板/),
    ).toBeTruthy();

    const confirmBtn = within(dialog).getByRole("button", { name: /确定/ });
    await user.click(confirmBtn);

    await waitFor(async () => {
      const persisted = await settingsMock.get_ui_mode();
      expect(persisted).toBe("expert");
    });

    await waitFor(() => expect(toastSuccessMock).toHaveBeenCalled());
  });

  it("cancels switch when the user clicks cancel", async () => {
    const user = userEvent.setup();
    renderWithClient(<ModeSwitch />);
    const toggle = await screen.findByRole("switch");
    await waitFor(() =>
      expect(toggle.getAttribute("aria-checked")).toBe("false"),
    );

    await user.click(toggle);

    const dialog = await screen.findByRole("dialog");
    const cancelBtn = within(dialog).getByRole("button", { name: /取消/ });
    await user.click(cancelBtn);

    await waitFor(() => expect(screen.queryByRole("dialog")).toBeNull());

    const persisted = await settingsMock.get_ui_mode();
    expect(persisted).toBe("novice");
    expect(toastSuccessMock).not.toHaveBeenCalled();
  });

  it("uses the expert -> novice copy when switching back", async () => {
    // Pre-seed expert mode
    await settingsMock.set_ui_mode("expert");

    renderWithClient(<ModeSwitch />);
    const toggle = await screen.findByRole("switch");
    await waitFor(() =>
      expect(toggle.getAttribute("aria-checked")).toBe("true"),
    );

    fireEvent.click(toggle);

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/切回小白模式将隐藏专家面板/)).toBeTruthy();
  });
});
