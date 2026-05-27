import { afterEach, describe, expect, it } from "vitest";
import {
  cliStateMock,
  enableFailure,
  getCurrentScenarioId,
  installerMock,
  loadScenario,
  profileMock,
  resetMockScenario,
  resetState,
  setMockDelay,
  setupMockIPC,
  teardownMockIPC,
} from "@/lib/api/mock";

afterEach(() => {
  teardownMockIPC();
  resetState();
});

describe("scenario switching", () => {
  it("switches between scenarios and replaces in-memory state", async () => {
    setMockDelay(0);
    loadScenario("new-user");
    expect(getCurrentScenarioId()).toBe("new-user");
    let claude = await installerMock.detect_cli("claude");
    expect(claude.installed).toBe(false);

    loadScenario("claude-installed-codex-missing");
    expect(getCurrentScenarioId()).toBe("claude-installed-codex-missing");
    claude = await installerMock.detect_cli("claude");
    expect(claude.installed).toBe(true);
    expect(claude.version).toBe("2.1.150");
  });

  it("resetMockScenario clears delay and failures", async () => {
    setMockDelay(0);
    loadScenario("fully-configured");
    enableFailure("profile", "list");
    await expect(profileMock.list("claude")).rejects.toBeTruthy();
    resetMockScenario();
    setMockDelay(0);
    // failures cleared, but scenario reset to default "new-user", reload to test
    loadScenario("fully-configured");
    const list = await profileMock.list("claude");
    expect(list.length).toBeGreaterThan(0);
  });

  it("all-installed-no-profile yields no profiles but installed CLIs", async () => {
    setMockDelay(0);
    loadScenario("all-installed-no-profile");
    const list = await profileMock.list("claude");
    expect(list).toEqual([]);
    const active = await cliStateMock.list_all_active();
    expect(active.claude).toBeNull();
    expect(active.codex).toBeNull();
  });
});

describe("setupMockIPC helper", () => {
  it("applies scenario + failures from options", async () => {
    setupMockIPC({
      scenario: "fully-configured",
      delayMs: 0,
      failures: [{ domain: "profile", command: "list" }],
    });
    await expect(profileMock.list("claude")).rejects.toMatchObject({
      code: "NETWORK_UNREACHABLE",
    });
  });
});
