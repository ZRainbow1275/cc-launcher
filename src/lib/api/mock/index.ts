import { isTauri } from "@tauri-apps/api/core";

import type { MockController, ScenarioId } from "../contracts";
import {
  cliStateReal,
  installerReal,
  launcherReal,
  onboardingReal,
  profileReal,
  sandboxReal,
  settingsReal,
  systemProbeReal,
} from "../real";
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
    const flag = import.meta.env?.VITE_MOCK_IPC;
    if (flag === "1") return true;
    if (flag === "0") return false;
    // Use Tauri's official runtime detection. When the webview hosts the
    // app, the Tauri runtime sets `globalThis.isTauri = true` synchronously
    // before any user JS runs, so this is safe at module-init time.
    if (isTauri()) return false;
    return import.meta.env?.DEV === true;
  } catch {
    return true;
  }
}

const MOCK = isMockMode();

export const installer = MOCK ? installerMock : installerReal;
export const profile = MOCK ? profileMock : profileReal;
export const cliState = MOCK ? cliStateMock : cliStateReal;
export const launcher = MOCK ? launcherMock : launcherReal;
export const systemProbe = MOCK ? systemProbeMock : systemProbeReal;
export const sandbox = MOCK ? sandboxMock : sandboxReal;
export const onboarding = MOCK ? onboardingMock : onboardingReal;
export const settings = MOCK ? settingsMock : settingsReal;

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
