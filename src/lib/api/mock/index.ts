import type { MockController, ScenarioId } from "../contracts";
import { installerMock } from "./installer";
import { launcherMock } from "./launcher";
import { onboardingMock } from "./onboarding";
import { cliStateMock, profileMock } from "./profile";
import { sandboxMock } from "./sandbox";
import {
  clearFailures,
  disableFailure,
  enableFailure,
  getMockDelay,
  resetScenario as resetRuntimeScenario,
  setMockDelay,
  setScenario as setRuntimeScenario,
} from "./runtime";
import { getCurrentScenarioId, loadScenario, resetState } from "./scenarios";
import { settingsMock } from "./settings";
import { systemProbeMock } from "./system-probe";

function isMockMode(): boolean {
  try {
    return import.meta.env?.VITE_MOCK_IPC === "1";
  } catch {
    return false;
  }
}

const notImplemented = (name: string) => () => {
  throw new Error(
    `Real Tauri IPC not implemented yet (Phase B): ${name}. Set VITE_MOCK_IPC=1 to use mock.`,
  );
};

function buildRealStub<T extends Record<string, unknown>>(
  prefix: string,
  shape: T,
): T {
  const out: Record<string, unknown> = {};
  for (const key of Object.keys(shape)) {
    out[key] = notImplemented(`${prefix}.${key}`);
  }
  return out as T;
}

const MOCK = isMockMode();

export const installer = MOCK
  ? installerMock
  : buildRealStub("installer", installerMock);
export const profile = MOCK
  ? profileMock
  : buildRealStub("profile", profileMock);
export const cliState = MOCK
  ? cliStateMock
  : buildRealStub("cliState", cliStateMock);
export const launcher = MOCK
  ? launcherMock
  : buildRealStub("launcher", launcherMock);
export const systemProbe = MOCK
  ? systemProbeMock
  : buildRealStub("systemProbe", systemProbeMock);
export const sandbox = MOCK
  ? sandboxMock
  : buildRealStub("sandbox", sandboxMock);
export const onboarding = MOCK
  ? onboardingMock
  : buildRealStub("onboarding", onboardingMock);
export const settings = MOCK
  ? settingsMock
  : buildRealStub("settings", settingsMock);

export const mockController: MockController = {
  setScenario(id: ScenarioId): void {
    loadScenario(id);
    setRuntimeScenario(id);
  },
  resetScenario(): void {
    resetRuntimeScenario();
    resetState();
  },
  getScenario(): ScenarioId {
    return getCurrentScenarioId();
  },
  setMockDelay,
  getMockDelay,
  enableFailure,
  disableFailure,
  clearFailures,
};

if (MOCK && typeof window !== "undefined") {
  (
    window as unknown as { __ccMockController?: MockController }
  ).__ccMockController = mockController;
}

export const MOCK_MODE = MOCK;

export {
  installerMock,
  launcherMock,
  onboardingMock,
  profileMock,
  cliStateMock,
  sandboxMock,
  settingsMock,
  systemProbeMock,
};
export {
  loadScenario,
  getCurrentScenarioId,
  resetState,
  type ScenarioState,
} from "./scenarios";
export {
  enableFailure,
  disableFailure,
  clearFailures,
  setMockDelay,
  getMockDelay,
  setScenario as setMockScenario,
  resetScenario as resetMockScenario,
} from "./runtime";

export * from "../contracts";
export {
  renderWithMockIPC,
  setupMockIPC,
  teardownMockIPC,
  collectAsyncIterable,
} from "./test-helpers";
