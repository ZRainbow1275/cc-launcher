import type { ScenarioId } from "../contracts";

interface RuntimeState {
  scenario: ScenarioId;
  delay: number;
  failures: Set<string>;
}

const DEFAULT_DELAY = 300;
const DEFAULT_SCENARIO: ScenarioId = "new-user";

const state: RuntimeState = {
  scenario: DEFAULT_SCENARIO,
  delay: DEFAULT_DELAY,
  failures: new Set<string>(),
};

type Listener = (scenario: ScenarioId) => void;
const listeners = new Set<Listener>();

export function getScenario(): ScenarioId {
  return state.scenario;
}

export function setScenario(id: ScenarioId): void {
  state.scenario = id;
  for (const l of listeners) l(id);
}

export function resetScenario(): void {
  state.scenario = DEFAULT_SCENARIO;
  state.delay = DEFAULT_DELAY;
  state.failures.clear();
  for (const l of listeners) l(state.scenario);
}

export function subscribeScenario(l: Listener): () => void {
  listeners.add(l);
  return () => listeners.delete(l);
}

export function setMockDelay(ms: number): void {
  if (ms < 0) throw new Error("delay must be >= 0");
  state.delay = ms;
}

export function getMockDelay(): number {
  return state.delay;
}

export async function delay(ms?: number): Promise<void> {
  const ts = ms ?? state.delay;
  if (ts <= 0) return;
  await new Promise<void>((resolve) => setTimeout(resolve, ts));
}

function failureKey(domain: string, command: string): string {
  return `${domain}.${command}`;
}

export function enableFailure(domain: string, command: string): void {
  state.failures.add(failureKey(domain, command));
}

export function disableFailure(domain: string, command: string): void {
  state.failures.delete(failureKey(domain, command));
}

export function clearFailures(): void {
  state.failures.clear();
}

export function shouldFail(domain: string, command: string): boolean {
  return state.failures.has(failureKey(domain, command));
}
