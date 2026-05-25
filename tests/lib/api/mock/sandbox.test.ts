import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  loadScenario,
  resetState,
  sandboxMock,
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

describe("sandboxMock.get_l1_rules", () => {
  it("returns the default L1 rule set", async () => {
    const rules = await sandboxMock.get_l1_rules();
    expect(rules.length).toBeGreaterThan(0);
    expect(rules.find((r) => r.id === "L1.claude_skip_permissions")).toBeTruthy();
    expect(rules.find((r) => r.id === "L1.codex_yolo")).toBeTruthy();
  });
});

describe("sandboxMock.set_l1_rule", () => {
  it("allows disabling an unlockable rule", async () => {
    const updated = await sandboxMock.set_l1_rule("L1.sudo_runas", false);
    expect(updated.enabled).toBe(false);
  });

  it("throws when trying to disable a non-unlockable rule", async () => {
    await expect(
      sandboxMock.set_l1_rule("L1.network_revshell", false),
    ).rejects.toMatchObject({ code: "L1_RULE_NOT_UNLOCKABLE" });
  });

  it("throws when rule id not found", async () => {
    await expect(sandboxMock.set_l1_rule("nope", true)).rejects.toMatchObject({
      code: "L1_RULE_NOT_FOUND",
    });
  });
});

describe("sandboxMock.unlock_l1_rule", () => {
  it("unlocks with valid keyword and sets unlockedUntil", async () => {
    const updated = await sandboxMock.unlock_l1_rule("L1.sudo_runas", "UNLOCK");
    expect(updated.unlockedUntil).toBeTruthy();
    expect(updated.enabled).toBe(false);
  });

  it("rejects invalid keyword", async () => {
    await expect(
      sandboxMock.unlock_l1_rule("L1.sudo_runas", "wrong"),
    ).rejects.toMatchObject({ code: "INVALID_UNLOCK_KEYWORD" });
  });

  it("refuses to unlock a non-unlockable rule", async () => {
    await expect(
      sandboxMock.unlock_l1_rule("L1.network_revshell", "UNLOCK"),
    ).rejects.toMatchObject({ code: "L1_RULE_NOT_UNLOCKABLE" });
  });
});

describe("sandboxMock.list_l2_redlines", () => {
  it("returns the L2 redline catalog", async () => {
    const list = await sandboxMock.list_l2_redlines();
    expect(list.length).toBeGreaterThan(10);
    expect(list.find((r) => r.id === "disk_wipe.rm_root")).toBeTruthy();
    expect(list.find((r) => r.id === "hosts.windows")).toBeTruthy();
  });
});

describe("sandboxMock.get_sandbox_level + set_sandbox_level", () => {
  it("reads and updates the sandbox level", async () => {
    const initial = await sandboxMock.get_sandbox_level();
    expect(["strict", "medium"]).toContain(initial);
    const res = await sandboxMock.set_sandbox_level("strict");
    expect(res.success).toBe(true);
    const after = await sandboxMock.get_sandbox_level();
    expect(after).toBe("strict");
  });
});
