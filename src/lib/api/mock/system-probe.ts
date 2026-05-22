import { FixAction, FixProgress, SystemProbeReport } from "../contracts";
import { errors, messages } from "./fixtures/i18n";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "system_probe";

function nowIso(): string {
  return new Date().toISOString();
}

function fixIdFor(action: FixAction): string {
  switch (action.kind) {
    case "installNode":
      return `fix-installNode-${action.targetLtsMajor}`;
    case "installGit":
      return "fix-installGit";
    case "cleanEnvVar":
      return `fix-cleanEnvVar-${action.varName}`;
    case "openHomeDir":
      return "fix-openHomeDir";
    case "injectPathEntries":
      return "fix-injectPathEntries";
    case "externalLink":
      return `fix-externalLink-${action.labelKey}`;
  }
}

function reprobe(): void {
  const state = getState();
  state.probeReport = {
    ...state.probeReport,
    items: state.probeReport.items.map((it) => {
      if (it.id === "node" && state.nodeStatus.installed) {
        return {
          ...it,
          status: "green",
          value: {
            version: state.nodeStatus.version,
            path: state.nodeStatus.path,
          },
          fixAction: null,
          messageKey: "probe.node.green",
        };
      }
      if (it.id === "git" && state.gitInstalled) {
        return {
          ...it,
          status: "green",
          value: {
            version: "2.43.0",
            path: "C:\\Program Files\\Git\\bin\\git.exe",
          },
          fixAction: null,
          messageKey: "probe.git.green",
        };
      }
      return it;
    }),
    generatedAt: nowIso(),
  };
  const hasRed = state.probeReport.items.some(
    (i) => i.status === "red" || i.status === "missing",
  );
  state.probeReport.overallStatus = hasRed
    ? "red"
    : state.probeReport.items.some((i) => i.status === "yellow")
      ? "yellow"
      : "green";
}

export const systemProbeMock = {
  async run(): Promise<SystemProbeReport> {
    if (shouldFail(DOMAIN, "run")) throw errors.networkUnreachable;
    await delay();
    const report = getState().probeReport;
    return SystemProbeReport.parse({ ...report, generatedAt: nowIso() });
  },

  apply_fix(fix_action: FixAction): AsyncIterable<FixProgress> {
    const action = FixAction.parse(fix_action);
    const fixId = fixIdFor(action);
    const stepDelay = 200;
    const state = getState();

    async function* gen(): AsyncGenerator<FixProgress, void, void> {
      const events: FixProgress[] = [];
      const emit = (p: FixProgress) => events.push(FixProgress.parse(p));

      if (shouldFail(DOMAIN, "apply_fix")) {
        emit({
          fixId,
          phase: "failed",
          message: messages.installerFailedNetwork,
          error: errors.networkUnreachable,
        });
        for (const e of events.splice(0)) yield e;
        return;
      }

      emit({
        fixId,
        phase: "starting",
        message: messages.fixStarting,
        percent: 5,
      });
      for (const e of events.splice(0)) yield e;
      await delay(stepDelay);

      emit({
        fixId,
        phase: "running",
        message: messages.fixRunning,
        percent: 50,
      });
      for (const e of events.splice(0)) yield e;
      await delay(stepDelay);

      switch (action.kind) {
        case "installNode":
          state.nodeStatus = {
            installed: true,
            version: "v20.11.0",
            path: "C:\\Users\\you\\.cc-switch\\runtime\\node\\node.exe",
            isPrivateRuntime: true,
            majorVersion: action.targetLtsMajor,
          };
          break;
        case "installGit":
          state.gitInstalled = true;
          break;
        case "cleanEnvVar":
        case "openHomeDir":
        case "injectPathEntries":
        case "externalLink":
          break;
      }

      emit({
        fixId,
        phase: "validating",
        message: messages.fixValidating,
        percent: 90,
      });
      for (const e of events.splice(0)) yield e;
      await delay(stepDelay);

      reprobe();

      emit({
        fixId,
        phase: "completed",
        message: messages.fixCompleted,
        percent: 100,
      });
      for (const e of events.splice(0)) yield e;
    }

    return gen();
  },
};

export type SystemProbeMock = typeof systemProbeMock;
