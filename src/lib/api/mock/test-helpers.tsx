import {
  render,
  type RenderOptions,
  type RenderResult,
} from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { type ReactElement, type ReactNode } from "react";
import type { ScenarioId } from "../contracts";
import { loadScenario } from "./scenarios";
import {
  clearFailures,
  enableFailure,
  resetScenario,
  setMockDelay,
  setScenario,
} from "./runtime";

export interface MockIPCSetupOptions {
  scenario: ScenarioId;
  delayMs?: number;
  failures?: { domain: string; command: string }[];
}

export function setupMockIPC(opts: MockIPCSetupOptions): void {
  resetScenario();
  clearFailures();
  loadScenario(opts.scenario);
  setScenario(opts.scenario);
  setMockDelay(opts.delayMs ?? 0);
  if (opts.failures) {
    for (const f of opts.failures) {
      enableFailure(f.domain, f.command);
    }
  }
}

export function teardownMockIPC(): void {
  clearFailures();
  resetScenario();
}

export interface RenderWithMockIPCOptions extends MockIPCSetupOptions {
  renderOptions?: Omit<RenderOptions, "wrapper">;
  queryClient?: QueryClient;
}

function makeQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: { retry: false, gcTime: 0, staleTime: 0 },
      mutations: { retry: false },
    },
  });
}

export function renderWithMockIPC(
  scenarioId: ScenarioId,
  ui: ReactElement,
  options: Partial<Omit<RenderWithMockIPCOptions, "scenario">> = {},
): RenderResult & { queryClient: QueryClient } {
  setupMockIPC({
    scenario: scenarioId,
    delayMs: options.delayMs ?? 0,
    failures: options.failures,
  });
  const queryClient = options.queryClient ?? makeQueryClient();
  const Wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
  );
  const result = render(ui, { wrapper: Wrapper, ...options.renderOptions });
  return { ...result, queryClient };
}

export async function collectAsyncIterable<T>(
  iter: AsyncIterable<T>,
): Promise<T[]> {
  const out: T[] = [];
  for await (const x of iter) out.push(x);
  return out;
}
