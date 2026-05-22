import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  loadScenario,
  onboardingMock,
  resetState,
  setMockDelay,
  settingsMock,
} from "@/lib/api/mock";

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("new-user");
});

afterEach(() => {
  resetState();
});

describe("onboardingMock.get_state", () => {
  it("returns not completed on new-user scenario", async () => {
    const state = await onboardingMock.get_state();
    expect(state.completed).toBe(false);
    expect(state.completedAt).toBeNull();
    expect(state.answers).toBeNull();
  });

  it("returns completed state on fully-configured scenario", async () => {
    loadScenario("fully-configured");
    const state = await onboardingMock.get_state();
    expect(state.completed).toBe(true);
    expect(state.answers).not.toBeNull();
  });
});

describe("onboardingMock.complete", () => {
  it("persists answers and updates ui mode + locale", async () => {
    const res = await onboardingMock.complete({
      locale: "en",
      uiMode: "expert",
      enableSandbox: true,
      acceptedRedlines: true,
      preferredCli: "codex",
    });
    expect(res.success).toBe(true);

    const state = await onboardingMock.get_state();
    expect(state.completed).toBe(true);
    expect(state.answers?.locale).toBe("en");
    expect(state.answers?.uiMode).toBe("expert");

    const mode = await settingsMock.get_ui_mode();
    expect(mode).toBe("expert");
    const locale = await settingsMock.get_locale();
    expect(locale).toBe("en");
  });

  it("rejects invalid payload", async () => {
    await expect(
      onboardingMock.complete({
        locale: "fr" as never,
        uiMode: "novice",
        enableSandbox: true,
        acceptedRedlines: true,
      }),
    ).rejects.toBeTruthy();
  });
});
