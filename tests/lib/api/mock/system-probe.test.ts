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
  it("returns a report with 16+ items in new-user scenario", async () => {
    const report = await systemProbeMock.run();
    expect(report.items.length).toBeGreaterThanOrEqual(16);
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
    expect(node.status).toBe("green");
  });

  it("emits failed phase on failure injection", async () => {
    enableFailure("system_probe", "apply_fix");
    const events = await collectAsyncIterable(
      systemProbeMock.apply_fix({ kind: "installGit" }),
    );
    expect(events[events.length - 1]!.phase).toBe("failed");
  });
});
