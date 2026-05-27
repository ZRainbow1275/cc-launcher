import {
  CliInstallStatus,
  InstallProgress,
  InstallerOpts,
  NodeStatus,
  OperationResult,
  RegistryName,
  RegistryPickResult,
  RegistryProbe,
  TargetCli,
} from "../contracts";
import { errors, messages } from "./fixtures/i18n";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "installer";
const MOCK_RUNTIME_ROOT = "C:\\Users\\you\\AppData\\Local\\cc-switch\\runtime";
const MOCK_NODE_PATH = `${MOCK_RUNTIME_ROOT}\\node\\node.exe`;

const REGISTRY_DEFS: {
  name: RegistryName;
  url: string;
  baseLatency: number;
}[] = [
  { name: "npmjs", url: "https://registry.npmjs.org", baseLatency: 420 },
  {
    name: "npmmirror",
    url: "https://registry.npmmirror.com",
    baseLatency: 180,
  },
  { name: "tencent", url: "https://mirrors.tencent.com/npm", baseLatency: 220 },
  {
    name: "huawei",
    url: "https://mirrors.huaweicloud.com/repository/npm",
    baseLatency: 340,
  },
];

function configuredRegistryDefs(): {
  name: RegistryName;
  url: string;
  baseLatency: number;
}[] {
  const custom = getState().installerSourceConfig.npmRegistry;
  if (!custom) return REGISTRY_DEFS;
  return [{ name: "custom", url: custom, baseLatency: 120 }, ...REGISTRY_DEFS];
}

function nowIso(): string {
  return new Date().toISOString();
}

async function progress(
  emit: (p: InstallProgress) => void,
  partial: Omit<InstallProgress, "phase"> & { phase: InstallProgress["phase"] },
  delayMs?: number,
): Promise<void> {
  const parsed = InstallProgress.parse(partial);
  emit(parsed);
  if (delayMs && delayMs > 0) await delay(delayMs);
}

export const installerMock = {
  async detect_cli(cli: TargetCli): Promise<CliInstallStatus> {
    TargetCli.parse(cli);
    if (shouldFail(DOMAIN, "detect_cli")) {
      throw errors.networkUnreachable;
    }
    await delay();
    const result = getState().cliStatus[cli];
    return CliInstallStatus.parse({ ...result, lastChecked: nowIso() });
  },

  install_cli(
    cli: TargetCli,
    opts?: InstallerOpts,
  ): AsyncIterable<InstallProgress> {
    TargetCli.parse(cli);
    if (opts !== undefined) InstallerOpts.parse(opts);
    const state = getState();
    const stepDelay = 200;

    async function* gen(): AsyncGenerator<InstallProgress, void, void> {
      const events: InstallProgress[] = [];
      const emit = (p: InstallProgress) => events.push(p);

      const networkOk =
        state.networkAvailable && !shouldFail(DOMAIN, "install_cli");

      await progress(
        emit,
        {
          phase: "probing-registry",
          message: messages.installerProbingRegistry,
          percent: 5,
        },
        stepDelay,
      );
      for (const e of events.splice(0)) yield e;

      if (!networkOk) {
        await progress(
          emit,
          {
            phase: "failed",
            message: messages.installerFailedNetwork,
            percent: 0,
            error: errors.networkUnreachable,
          },
          0,
        );
        for (const e of events.splice(0)) yield e;
        return;
      }

      const chosen =
        opts?.registry ??
        state.installerSourceConfig.npmRegistry ??
        REGISTRY_DEFS[1].url;

      if (!state.nodeStatus.installed && !opts?.skipNodeCheck) {
        await progress(
          emit,
          {
            phase: "installing-node",
            message: messages.installerInstallingNode,
            percent: 25,
            registry: chosen,
          },
          stepDelay,
        );
        for (const e of events.splice(0)) yield e;
      }

      await progress(
        emit,
        {
          phase: "installing-cli",
          message: messages.installerInstallingCli,
          percent: 60,
          registry: chosen,
        },
        stepDelay,
      );
      for (const e of events.splice(0)) yield e;

      await progress(
        emit,
        {
          phase: "validating",
          message: messages.installerValidating,
          percent: 90,
        },
        stepDelay,
      );
      for (const e of events.splice(0)) yield e;

      const version = cli === "claude" ? "2.1.150" : "0.133.0";
      state.cliStatus[cli] = {
        cli,
        installed: true,
        version,
        path:
          cli === "claude"
            ? `${MOCK_RUNTIME_ROOT}\\claude\\claude.cmd`
            : `${MOCK_RUNTIME_ROOT}\\codex\\codex.cmd`,
        lastChecked: nowIso(),
      };

      await progress(
        emit,
        {
          phase: "completed",
          message: messages.installerCompleted,
          percent: 100,
          registry: chosen,
        },
        0,
      );
      for (const e of events.splice(0)) yield e;
    }

    return gen();
  },

  async uninstall_cli(cli: TargetCli): Promise<OperationResult> {
    TargetCli.parse(cli);
    if (shouldFail(DOMAIN, "uninstall_cli")) {
      return OperationResult.parse({
        success: false,
        errorCode: "UNINSTALL_FAILED",
      });
    }
    await delay();
    const state = getState();
    state.cliStatus[cli] = {
      cli,
      installed: false,
      lastChecked: nowIso(),
    };
    return OperationResult.parse({
      success: true,
      message: messages.uninstallSuccess,
    });
  },

  async smart_pick_registry(): Promise<RegistryPickResult> {
    if (shouldFail(DOMAIN, "smart_pick_registry")) {
      throw errors.networkUnreachable;
    }
    await delay();
    const state = getState();
    const defs = configuredRegistryDefs();
    const candidates: RegistryProbe[] = defs.map((r) =>
      RegistryProbe.parse({
        name: r.name,
        url: r.url,
        ok: state.networkAvailable,
        latencyMs: state.networkAvailable ? r.baseLatency : 5000,
        statusCode: state.networkAvailable ? 200 : undefined,
        error: state.networkAvailable ? undefined : "timeout",
      }),
    );

    if (!state.networkAvailable) {
      throw errors.networkUnreachable;
    }

    const custom = candidates.find((c) => c.name === "custom" && c.ok);
    const winner =
      custom ??
      [...candidates]
        .filter((c) => c.ok)
        .sort((a, b) => a.latencyMs - b.latencyMs)[0]!;

    return RegistryPickResult.parse({
      candidates,
      chosen: winner.url,
      chosenName: winner.name,
      chosenAt: nowIso(),
      cached: false,
    });
  },

  async detect_node(): Promise<NodeStatus> {
    if (shouldFail(DOMAIN, "detect_node")) {
      throw errors.networkUnreachable;
    }
    await delay();
    return NodeStatus.parse(getState().nodeStatus);
  },

  install_node(): AsyncIterable<InstallProgress> {
    const state = getState();
    const stepDelay = 200;
    async function* gen(): AsyncGenerator<InstallProgress, void, void> {
      const events: InstallProgress[] = [];
      const emit = (p: InstallProgress) => events.push(p);

      if (!state.networkAvailable || shouldFail(DOMAIN, "install_node")) {
        await progress(
          emit,
          {
            phase: "failed",
            message: messages.installerFailedNetwork,
            error: errors.networkUnreachable,
          },
          0,
        );
        for (const e of events.splice(0)) yield e;
        return;
      }

      await progress(
        emit,
        {
          phase: "probing-registry",
          message: messages.installerProbingRegistry,
          percent: 10,
          registry: state.installerSourceConfig.nodeDistMirror,
        },
        stepDelay,
      );
      for (const e of events.splice(0)) yield e;

      await progress(
        emit,
        {
          phase: "installing-node",
          message: messages.installerInstallingNode,
          percent: 50,
          registry: state.installerSourceConfig.nodeDistMirror,
        },
        stepDelay,
      );
      for (const e of events.splice(0)) yield e;

      await progress(
        emit,
        {
          phase: "validating",
          message: messages.installerValidating,
          percent: 90,
        },
        stepDelay,
      );
      for (const e of events.splice(0)) yield e;

      state.nodeStatus = {
        installed: true,
        version: "v20.11.0",
        path: MOCK_NODE_PATH,
        isPrivateRuntime: true,
        majorVersion: 20,
      };

      await progress(
        emit,
        {
          phase: "completed",
          message: messages.installerCompleted,
          percent: 100,
        },
        0,
      );
      for (const e of events.splice(0)) yield e;
    }
    return gen();
  },

  // install_git removed in D-10 — Git install flows through
  // `systemProbe.apply_fix({ kind: "installGit" })`. See phase-c-parity.md §C.
};

export type InstallerMock = typeof installerMock;
