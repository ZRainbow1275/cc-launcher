import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  collectAsyncIterable,
  enableFailure,
  loadScenario,
  resetState,
  setMockDelay,
  systemProbeMock,
} from "@/lib/api/mock";

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("new-user");
});

afterEach(() => {
  resetState();
});

describe("systemProbeMock.run", () => {
  it("returns a report with 17 items in new-user scenario", async () => {
    const report = await systemProbeMock.run();
    expect(report.items).toHaveLength(17);
    expect(report.overallStatus).toBe("red");
  });

  it("returns green on fully-configured scenario", async () => {
    loadScenario("fully-configured");
    const report = await systemProbeMock.run();
    expect(report.overallStatus).toBe("green");
  });

  it("throws on failure injection", async () => {
    enableFailure("system_probe", "run");
    await expect(systemProbeMock.run()).rejects.toMatchObject({
      code: "NETWORK_UNREACHABLE",
    });
  });
});

describe("systemProbeMock.apply_fix", () => {
  it("emits progress events and updates node status", async () => {
    const events = await collectAsyncIterable(
      systemProbeMock.apply_fix({ kind: "installNode", targetLtsMajor: 20 }),
    );
    expect(events.length).toBeGreaterThanOrEqual(3);
    expect(events[events.length - 1]!.phase).toBe("completed");

    const report = await systemProbeMock.run();
    const node = report.items.find((i) => i.id === "node")!;
    const npm = report.items.find((i) => i.id === "npm")!;
    const path = report.items.find((i) => i.id === "path")!;
    expect(node.status).toBe("green");
    expect(npm.status).toBe("green");
    expect(path.value).toMatchObject({
      coveredByPrivateRuntime: ["node", "npm"],
      unresolved: ["git"],
    });
  });

  it("emits failed phase on failure injection", async () => {
    enableFailure("system_probe", "apply_fix");
    const events = await collectAsyncIterable(
      systemProbeMock.apply_fix({ kind: "installGit" }),
    );
    expect(events[events.length - 1]!.phase).toBe("failed");
  });

  it("creates the mock workdir and clears both workdir probes", async () => {
    const events = await collectAsyncIterable(
      systemProbeMock.apply_fix({
        kind: "createWorkdir",
        path: "C:\\Users\\you\\cc-launcher-projects",
      }),
    );
    expect(events[events.length - 1]!.phase).toBe("completed");

    const report = await systemProbeMock.run();
    expect(report.items.find((i) => i.id === "workdirExists")?.status).toBe(
      "green",
    );
    expect(report.items.find((i) => i.id === "workdirWritable")?.status).toBe(
      "green",
    );
  });
});
