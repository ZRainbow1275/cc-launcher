import { renderHook, act, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { ReactNode } from "react";
import { useUiMode } from "@/hooks/useUiMode";
import {
  clearFailures,
  enableFailure,
  loadScenario,
  resetState,
  setMockDelay,
  setMockScenario,
} from "@/lib/api/mock";

function makeWrapper() {
  const client = new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  });
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  return { Wrapper, client };
}

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("new-user");
  setMockScenario("new-user");
});

afterEach(() => {
  resetState();
  clearFailures();
});

describe("useUiMode", () => {
  it("returns 'novice' as the default mode during initial load", async () => {
    const { Wrapper } = makeWrapper();
    const { result } = renderHook(() => useUiMode(), { wrapper: Wrapper });

    expect(result.current.mode).toBe("novice");
    expect(result.current.isLoading).toBe(true);

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.mode).toBe("novice");
  });

  it("setMode persists the new mode and re-reads on query invalidation", async () => {
    const { Wrapper } = makeWrapper();
    const { result } = renderHook(() => useUiMode(), { wrapper: Wrapper });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.mode).toBe("novice");

    await act(async () => {
      await result.current.setMode("expert");
    });

    await waitFor(() => expect(result.current.mode).toBe("expert"));
  });

  it("exposes isError when the IPC call fails", async () => {
    enableFailure("settings", "get_ui_mode");
    const { Wrapper } = makeWrapper();
    const { result } = renderHook(() => useUiMode(), { wrapper: Wrapper });

    await waitFor(() => expect(result.current.isLoading).toBe(false));
    expect(result.current.isError).toBe(true);
    // Falls back to default mode when query fails
    expect(result.current.mode).toBe("novice");
  });
});
