import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  launcherMock,
  loadScenario,
  profileMock,
  resetState,
  setMockDelay,
} from "@/lib/api/mock";

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("fully-configured");
});

afterEach(() => {
  resetState();
});

describe("launcherMock.list_terminals", () => {
  it("returns terminals on Windows scenario", async () => {
    const list = await launcherMock.list_terminals();
    expect(list.length).toBeGreaterThan(0);
    expect(list.find((t) => t.isDefault)).toBeTruthy();
  });

  it("returns empty list on network-failure scenario", async () => {
    loadScenario("network-failure");
    const list = await launcherMock.list_terminals();
    expect(list).toEqual([]);
  });
});

describe("launcherMock.start_cli", () => {
  it("launches a profile successfully", async () => {
    const profiles = await profileMock.list("claude");
    const target = profiles[0]!;
    const res = await launcherMock.start_cli({
      profile_id: target.id,
      target_cli: "claude",
    });
    expect(res.success).toBe(true);
    expect(res.cwd).toContain(target.id);
    expect(res.pid).toBeGreaterThan(0);
  });

  it("returns error when profile not found", async () => {
    const res = await launcherMock.start_cli({
      profile_id: "nope",
      target_cli: "claude",
    });
    expect(res.success).toBe(false);
    expect(res.error?.code).toBe("PROFILE_NOT_FOUND");
  });

  it("returns no-terminal error when no terminal installed", async () => {
    loadScenario("network-failure");
    const res = await launcherMock.start_cli({
      profile_id: "any",
      target_cli: "claude",
    });
    expect(res.success).toBe(false);
    expect(res.error?.code).toBeDefined();
  });
});
