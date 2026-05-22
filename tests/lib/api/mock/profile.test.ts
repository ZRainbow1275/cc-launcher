import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  enableFailure,
  loadScenario,
  profileMock,
  cliStateMock,
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

describe("profileMock.list", () => {
  it("lists only profiles for the given CLI", async () => {
    const claude = await profileMock.list("claude");
    const codex = await profileMock.list("codex");
    expect(claude.length).toBe(3);
    expect(codex.length).toBe(3);
    expect(claude.every((p) => p.target_cli === "claude")).toBe(true);
  });

  it("returns empty array on all-installed-no-profile", async () => {
    loadScenario("all-installed-no-profile");
    const result = await profileMock.list("claude");
    expect(result).toEqual([]);
  });
});

describe("profileMock.get", () => {
  it("returns null when not found", async () => {
    const got = await profileMock.get("does-not-exist", "claude");
    expect(got).toBeNull();
  });

  it("returns the profile when present", async () => {
    const list = await profileMock.list("claude");
    const target = list[0]!;
    const got = await profileMock.get(target.id, "claude");
    expect(got?.id).toBe(target.id);
  });
});

describe("profileMock.create + update + delete", () => {
  it("creates a new profile and persists into list", async () => {
    loadScenario("all-installed-no-profile");
    const created = await profileMock.create({
      target_cli: "claude",
      name: "测试 Profile",
      provider_id: "anthropic-official",
    });
    expect(created.is_builtin).toBe(false);
    expect(created.name).toBe("测试 Profile");
    const list = await profileMock.list("claude");
    expect(list.find((p) => p.id === created.id)).toBeTruthy();
  });

  it("updates a profile and refreshes updated_at", async () => {
    const list = await profileMock.list("claude");
    const editable = list.find((p) => !p.is_builtin)!;
    const updated = await profileMock.update(editable.id, "claude", {
      name: "新名称",
    });
    expect(updated.name).toBe("新名称");
    expect(updated.updated_at).toBeGreaterThanOrEqual(editable.updated_at);
  });

  it("throws on update when profile not found", async () => {
    await expect(
      profileMock.update("does-not-exist", "claude", { name: "X" }),
    ).rejects.toMatchObject({ code: "PROFILE_NOT_FOUND" });
  });

  it("refuses to delete builtin profile", async () => {
    const list = await profileMock.list("claude");
    const builtin = list.find((p) => p.is_builtin)!;
    const res = await profileMock.delete(builtin.id, "claude");
    expect(res.success).toBe(false);
    expect(res.errorCode).toBe("BUILTIN_PROFILE_PROTECTED");
  });

  it("refuses to delete active profile", async () => {
    const active = await cliStateMock.get_active("claude");
    expect(active).not.toBeNull();
    const res = await profileMock.delete(active!, "claude");
    expect(res.success).toBe(false);
    expect(res.errorCode).toBe("PROFILE_IS_ACTIVE");
  });
});

describe("profileMock.activate + cliStateMock", () => {
  it("activates a profile and updates active map", async () => {
    const list = await profileMock.list("claude");
    const target = list.find((p) => !p.is_builtin && p.id !== (list.find((x) => x.is_builtin)?.id))!;
    const res = await profileMock.activate(target.id, "claude");
    expect(res.success).toBe(true);
    const active = await cliStateMock.get_active("claude");
    expect(active).toBe(target.id);
  });

  it("returns error switchResult when profile missing", async () => {
    const res = await profileMock.activate("nope", "claude");
    expect(res.success).toBe(false);
    expect(res.error?.code).toBe("PROFILE_NOT_FOUND");
  });

  it("list_all_active returns both CLIs", async () => {
    const map = await cliStateMock.list_all_active();
    expect(map.claude).not.toBeNull();
    expect(map.codex).not.toBeNull();
  });
});

describe("profileMock failure injection", () => {
  it("rejects list when failure injected", async () => {
    enableFailure("profile", "list");
    await expect(profileMock.list("claude")).rejects.toMatchObject({
      code: "NETWORK_UNREACHABLE",
    });
  });
});
