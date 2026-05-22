import { describe, expect, it } from "vitest";
import {
  CliInstallStatus,
  FixAction,
  InstallProgress,
  L1Rule,
  Profile,
  ProfileCreatePayload,
  ProfileUpdatePayload,
  SystemProbeReport,
  TargetCli,
} from "@/lib/api/contracts";

describe("TargetCli", () => {
  it("accepts claude and codex only", () => {
    expect(TargetCli.parse("claude")).toBe("claude");
    expect(TargetCli.parse("codex")).toBe("codex");
    expect(() => TargetCli.parse("gemini")).toThrow();
  });
});

describe("Profile schema", () => {
  it("accepts a minimal valid profile and applies defaults", () => {
    const ts = Date.now();
    const profile = Profile.parse({
      id: "p1",
      target_cli: "claude",
      name: "Default",
      provider_id: null,
      settings_json: "{}",
      is_builtin: false,
      mcp_ids: [],
      skill_ids: [],
      created_at: ts,
      updated_at: ts,
    });
    expect(profile.is_builtin).toBe(false);
    expect(profile.mcp_ids).toEqual([]);
  });

  it("rejects unknown extra fields (strict)", () => {
    const ts = Date.now();
    expect(() =>
      Profile.parse({
        id: "p1",
        target_cli: "claude",
        name: "Default",
        provider_id: null,
        settings_json: "{}",
        is_builtin: false,
        mcp_ids: [],
        skill_ids: [],
        created_at: ts,
        updated_at: ts,
        extra: "nope",
      } as never),
    ).toThrow();
  });
});

describe("ProfileCreatePayload + ProfileUpdatePayload", () => {
  it("requires target_cli + name in create payload", () => {
    expect(() => ProfileCreatePayload.parse({ name: "X" } as never)).toThrow();
    const valid = ProfileCreatePayload.parse({
      target_cli: "codex",
      name: "X",
    });
    expect(valid.name).toBe("X");
  });

  it("update payload allows partial fields", () => {
    const p = ProfileUpdatePayload.parse({ name: "renamed" });
    expect(p.name).toBe("renamed");
  });
});

describe("CliInstallStatus", () => {
  it("requires lastChecked ISO datetime", () => {
    const v = CliInstallStatus.parse({
      cli: "claude",
      installed: false,
      lastChecked: new Date().toISOString(),
    });
    expect(v.cli).toBe("claude");
    expect(() =>
      CliInstallStatus.parse({
        cli: "claude",
        installed: false,
        lastChecked: "not-iso",
      }),
    ).toThrow();
  });
});

describe("InstallProgress", () => {
  it("accepts known phases with localized message", () => {
    InstallProgress.parse({
      phase: "installing-cli",
      message: { zh: "z", en: "e", ja: "j" },
      percent: 50,
    });
  });

  it("rejects unknown phases", () => {
    expect(() =>
      InstallProgress.parse({
        phase: "huh" as never,
        message: { zh: "z", en: "e", ja: "j" },
      }),
    ).toThrow();
  });
});

describe("FixAction", () => {
  it("discriminates on kind", () => {
    const a = FixAction.parse({ kind: "installNode", targetLtsMajor: 20 });
    expect(a.kind).toBe("installNode");
    expect(() =>
      FixAction.parse({ kind: "installNode" } as never),
    ).toThrow();
  });
});

describe("L1Rule", () => {
  it("parses a rule with unlockedUntil null", () => {
    const now = new Date().toISOString();
    const rule = L1Rule.parse({
      id: "L1.x",
      category: "DangerousFilesystem",
      pattern: ".*",
      titleKey: "a",
      descriptionKey: "b",
      enabled: true,
      unlockable: true,
      unlockedUntil: null,
      updatedAt: now,
    });
    expect(rule.id).toBe("L1.x");
  });
});

describe("SystemProbeReport", () => {
  it("parses an empty report", () => {
    const r = SystemProbeReport.parse({
      overallStatus: "green",
      items: [],
      generatedAt: new Date().toISOString(),
      probeVersion: 1,
    });
    expect(r.items).toEqual([]);
  });
});
