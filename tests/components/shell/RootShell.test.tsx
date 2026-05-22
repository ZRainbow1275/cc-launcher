import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderWithMockIPC, teardownMockIPC } from "@/lib/api/mock";

// jsdom does not implement matchMedia; the ThemeProvider in RootShell uses it
// to subscribe to OS color-scheme preference changes. Stub it before any
// component renders.
if (typeof window !== "undefined" && !window.matchMedia) {
  Object.defineProperty(window, "matchMedia", {
    writable: true,
    configurable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addListener: () => {},
      removeListener: () => {},
      addEventListener: () => {},
      removeEventListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}

// Mock the lazy cc-switch App tree. The real App is the whole multi-Provider
// daily-driver tree that pulls in heavy Tauri/window globals; the only thing
// we want to assert at the RootShell layer is that it lazily mounts App
// under its provider stack.
vi.mock("@/App", () => ({
  __esModule: true,
  default: () => (
    <div data-testid="cc-switch-app-root">cc-switch app mounted</div>
  ),
}));

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() },
  Toaster: () => null,
}));

// Avoid pulling in the real UpdateProvider's Tauri event subscriptions.
vi.mock("@/contexts/UpdateContext", () => ({
  UpdateProvider: ({ children }: { children: React.ReactNode }) => (
    <>{children}</>
  ),
}));

import { RootShell } from "@/components/shell/RootShell";

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("RootShell provider stack", () => {
  it("lazy-mounts the App tree under the QueryClient / Theme / Update provider stack", async () => {
    renderWithMockIPC("new-user", <RootShell />);

    // Lazy chunk resolves on the next microtask; assert end state.
    await waitFor(() => {
      expect(screen.getByTestId("cc-switch-app-root")).toBeInTheDocument();
    });
  });
});
