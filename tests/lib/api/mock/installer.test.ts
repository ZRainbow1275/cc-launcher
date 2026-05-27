import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  collectAsyncIterable,
  enableFailure,
  installerMock,
  loadScenario,
  resetState,
  setMockDelay,
} from "@/lib/api/mock";

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("new-user");
});

afterEach(() => {
  resetState();
});

describe("installerMock.detect_cli", () => {
  it("returns not installed on new-user scenario", async () => {
    const status = await installerMock.detect_cli("claude");
    expect(status.installed).toBe(false);
    expect(status.cli).toBe("claude");
  });

  it("returns installed on claude-installed-codex-missing scenario", async () => {
    loadScenario("claude-installed-codex-missing");
    const claude = await installerMock.detect_cli("claude");
    const codex = await installerMock.detect_cli("codex");
    expect(claude.installed).toBe(true);
    expect(claude.version).toBe("2.1.150");
    expect(codex.installed).toBe(false);
  });

  it("rejects when failure injected", async () => {
    enableFailure("installer", "detect_cli");
    await expect(installerMock.detect_cli("claude")).rejects.toMatchObject({
      code: "NETWORK_UNREACHABLE",
    });
  });
});

describe("installerMock.install_cli", () => {
  it("emits progress events and marks installed at completion", async () => {
    const events = await collectAsyncIterable(
      installerMock.install_cli("claude"),
    );
    expect(events.length).toBeGreaterThanOrEqual(3);
    expect(events[events.length - 1]!.phase).toBe("completed");

    const status = await installerMock.detect_cli("claude");
    expect(status.installed).toBe(true);
    expect(status.version).toBe("2.1.150");
  });

  it("emits failed phase on network-failure scenario", async () => {
    loadScenario("network-failure");
    const events = await collectAsyncIterable(
      installerMock.install_cli("claude"),
    );
    const last = events[events.length - 1]!;
    expect(last.phase).toBe("failed");
    expect(last.error?.code).toBe("NETWORK_UNREACHABLE");
  });

  it("uses configured npm registry when install opts omit registry", async () => {
    const { settingsMock } = await import("@/lib/api/mock");
    await settingsMock.set_installer_source_config({
      npmRegistry: "https://vps.example.com/npm",
    });

    const events = await collectAsyncIterable(
      installerMock.install_cli("claude"),
    );
    const installCli = events.find((event) => event.phase === "installing-cli");

    expect(installCli?.registry).toBe("https://vps.example.com/npm");
    expect(events[events.length - 1]?.registry).toBe(
      "https://vps.example.com/npm",
    );
  });
});

describe("installerMock.smart_pick_registry", () => {
  it("returns 4 candidates and picks the fastest", async () => {
    const result = await installerMock.smart_pick_registry();
    expect(result.candidates).toHaveLength(4);
    expect(result.candidates.every((c) => c.ok)).toBe(true);
    expect(result.chosen).toMatch(/^https:\/\//);
  });

  it("throws on network-failure scenario", async () => {
    loadScenario("network-failure");
    await expect(installerMock.smart_pick_registry()).rejects.toMatchObject({
      code: "NETWORK_UNREACHABLE",
    });
  });

  it("prefers configured custom registry when present", async () => {
    loadScenario("fully-configured");
    const { settingsMock } = await import("@/lib/api/mock");
    await settingsMock.set_installer_source_config({
      npmRegistry: "https://vps.example.com/npm",
    });

    const result = await installerMock.smart_pick_registry();
    expect(result.chosenName).toBe("custom");
    expect(result.chosen).toBe("https://vps.example.com/npm");
    expect(result.candidates[0]?.name).toBe("custom");
  });
});

describe("installerMock.uninstall_cli", () => {
  it("marks cli as uninstalled", async () => {
    loadScenario("claude-installed-codex-missing");
    const result = await installerMock.uninstall_cli("claude");
    expect(result.success).toBe(true);
    const status = await installerMock.detect_cli("claude");
    expect(status.installed).toBe(false);
  });
});

describe("installerMock.detect_node + install_node", () => {
  it("reports missing node in new-user scenario", async () => {
    const status = await installerMock.detect_node();
    expect(status.installed).toBe(false);
  });

  it("install_node updates state", async () => {
    const events = await collectAsyncIterable(installerMock.install_node());
    expect(events[events.length - 1]!.phase).toBe("completed");
    const status = await installerMock.detect_node();
    expect(status.installed).toBe(true);
    expect(status.majorVersion).toBe(20);
  });

  it("uses the configured Node mirror when one is saved", async () => {
    loadScenario("new-user");
    const { settingsMock } = await import("@/lib/api/mock");
    await settingsMock.set_installer_source_config({
      nodeDistMirror: "https://vps.example.com/node",
    });

    const events = await collectAsyncIterable(installerMock.install_node());
    expect(events[0]?.registry).toBe("https://vps.example.com/node");
    expect(events[events.length - 1]!.phase).toBe("completed");
    const status = await installerMock.detect_node();
    expect(status.installed).toBe(true);
  });
});
