import type {
  ActiveProfileMap,
  CliInstallStatus,
  InstallerSourceConfig,
  L1Rule,
  Locale,
  NodeStatus,
  OnboardingState,
  Profile,
  SandboxLevel,
  ScenarioId,
  SystemProbeReport,
  TargetCli,
  TerminalCandidate,
  UiMode,
} from "../contracts";
import {
  defaultProfile,
  expProfile,
  resetIdCounter,
  workProfile,
} from "./fixtures/profiles";
import {
  probeReportFullyConfigured,
  probeReportNetworkFailure,
  probeReportNewUser,
} from "./fixtures/probe";
import { defaultL1Rules } from "./fixtures/sandbox";
import { windowsTerminals, noTerminals } from "./fixtures/terminals";

const MOCK_RUNTIME_ROOT = "C:\\Users\\you\\AppData\\Local\\cc-switch\\runtime";
const MOCK_NODE_PATH = `${MOCK_RUNTIME_ROOT}\\node\\node.exe`;
const CLAUDE_VERSION = "2.1.150";
const CODEX_VERSION = "0.133.0";

export interface ScenarioState {
  cliStatus: Record<TargetCli, CliInstallStatus>;
  nodeStatus: NodeStatus;
  gitInstalled: boolean;
  workdirReady: boolean;
  workdirPath: string;
  profiles: Profile[];
  activeProfiles: ActiveProfileMap;
  terminals: TerminalCandidate[];
  probeReport: SystemProbeReport;
  l1Rules: L1Rule[];
  sandboxLevel: SandboxLevel;
  onboarding: OnboardingState;
  uiMode: UiMode;
  locale: Locale;
  networkAvailable: boolean;
  installerSourceConfig: InstallerSourceConfig;
}

function emptyCliStatus(cli: TargetCli, lastChecked: string): CliInstallStatus {
  return { cli, installed: false, lastChecked };
}

function installedCliStatus(
  cli: TargetCli,
  version: string,
  lastChecked: string,
): CliInstallStatus {
  return {
    cli,
    installed: true,
    version,
    path:
      cli === "claude"
        ? `${MOCK_RUNTIME_ROOT}\\claude\\claude.cmd`
        : `${MOCK_RUNTIME_ROOT}\\codex\\codex.cmd`,
    lastChecked,
  };
}

const generatedAt = "2026-05-22T10:00:00.000Z";

function defaultInstallerSourceConfig(): InstallerSourceConfig {
  return {};
}

function buildNewUser(): ScenarioState {
  resetIdCounter();
  return {
    cliStatus: {
      claude: emptyCliStatus("claude", generatedAt),
      codex: emptyCliStatus("codex", generatedAt),
    },
    nodeStatus: { installed: false, isPrivateRuntime: true },
    gitInstalled: false,
    workdirReady: false,
    workdirPath: "C:\\Users\\you\\cc-launcher-projects",
    profiles: [],
    activeProfiles: { claude: null, codex: null },
    terminals: windowsTerminals(),
    probeReport: probeReportNewUser(),
    l1Rules: defaultL1Rules(),
    sandboxLevel: "strict",
    onboarding: { completed: false, completedAt: null, answers: null },
    uiMode: "novice",
    locale: "zh",
    networkAvailable: true,
    installerSourceConfig: defaultInstallerSourceConfig(),
  };
}

function buildClaudeInstalledCodexMissing(): ScenarioState {
  resetIdCounter();
  const claudeDefault = defaultProfile("claude");
  const codexDefault = defaultProfile("codex");
  return {
    cliStatus: {
      claude: installedCliStatus("claude", CLAUDE_VERSION, generatedAt),
      codex: emptyCliStatus("codex", generatedAt),
    },
    nodeStatus: {
      installed: true,
      version: "v20.11.0",
      path: MOCK_NODE_PATH,
      isPrivateRuntime: true,
      majorVersion: 20,
    },
    gitInstalled: true,
    workdirReady: true,
    workdirPath: "C:\\Users\\you\\cc-launcher-projects",
    profiles: [claudeDefault, codexDefault],
    activeProfiles: {
      claude: claudeDefault.id,
      codex: codexDefault.id,
    },
    terminals: windowsTerminals(),
    probeReport: probeReportFullyConfigured(),
    l1Rules: defaultL1Rules(),
    sandboxLevel: "strict",
    onboarding: {
      completed: true,
      completedAt: generatedAt,
      answers: {
        locale: "zh",
        uiMode: "novice",
        enableSandbox: true,
        acceptedRedlines: true,
        preferredCli: "claude",
      },
    },
    uiMode: "novice",
    locale: "zh",
    networkAvailable: true,
    installerSourceConfig: defaultInstallerSourceConfig(),
  };
}

function buildAllInstalledNoProfile(): ScenarioState {
  resetIdCounter();
  return {
    cliStatus: {
      claude: installedCliStatus("claude", CLAUDE_VERSION, generatedAt),
      codex: installedCliStatus("codex", CODEX_VERSION, generatedAt),
    },
    nodeStatus: {
      installed: true,
      version: "v20.11.0",
      path: MOCK_NODE_PATH,
      isPrivateRuntime: true,
      majorVersion: 20,
    },
    gitInstalled: true,
    workdirReady: true,
    workdirPath: "C:\\Users\\you\\cc-launcher-projects",
    profiles: [],
    activeProfiles: { claude: null, codex: null },
    terminals: windowsTerminals(),
    probeReport: probeReportFullyConfigured(),
    l1Rules: defaultL1Rules(),
    sandboxLevel: "strict",
    onboarding: { completed: false, completedAt: null, answers: null },
    uiMode: "novice",
    locale: "zh",
    networkAvailable: true,
    installerSourceConfig: defaultInstallerSourceConfig(),
  };
}

function buildFullyConfigured(): ScenarioState {
  resetIdCounter();
  const claudeDefault = defaultProfile("claude");
  const claudeWork = workProfile("claude", "工作");
  const claudeExp = expProfile("claude");
  const codexDefault = defaultProfile("codex");
  const codexWork = workProfile("codex", "工作");
  const codexExp = expProfile("codex");
  return {
    cliStatus: {
      claude: installedCliStatus("claude", CLAUDE_VERSION, generatedAt),
      codex: installedCliStatus("codex", CODEX_VERSION, generatedAt),
    },
    nodeStatus: {
      installed: true,
      version: "v20.11.0",
      path: MOCK_NODE_PATH,
      isPrivateRuntime: true,
      majorVersion: 20,
    },
    gitInstalled: true,
    workdirReady: true,
    workdirPath: "C:\\Users\\you\\cc-launcher-projects",
    profiles: [
      claudeDefault,
      claudeWork,
      claudeExp,
      codexDefault,
      codexWork,
      codexExp,
    ],
    activeProfiles: {
      claude: claudeWork.id,
      codex: codexDefault.id,
    },
    terminals: windowsTerminals(),
    probeReport: probeReportFullyConfigured(),
    l1Rules: defaultL1Rules(),
    sandboxLevel: "medium",
    onboarding: {
      completed: true,
      completedAt: generatedAt,
      answers: {
        locale: "zh",
        uiMode: "expert",
        enableSandbox: true,
        acceptedRedlines: true,
        preferredCli: "claude",
      },
    },
    uiMode: "expert",
    locale: "zh",
    networkAvailable: true,
    installerSourceConfig: defaultInstallerSourceConfig(),
  };
}

function buildNetworkFailure(): ScenarioState {
  resetIdCounter();
  return {
    cliStatus: {
      claude: emptyCliStatus("claude", generatedAt),
      codex: emptyCliStatus("codex", generatedAt),
    },
    nodeStatus: { installed: false, isPrivateRuntime: true },
    gitInstalled: false,
    workdirReady: false,
    workdirPath: "C:\\Users\\you\\cc-launcher-projects",
    profiles: [],
    activeProfiles: { claude: null, codex: null },
    terminals: noTerminals(),
    probeReport: probeReportNetworkFailure(),
    l1Rules: defaultL1Rules(),
    sandboxLevel: "strict",
    onboarding: { completed: false, completedAt: null, answers: null },
    uiMode: "novice",
    locale: "zh",
    networkAvailable: false,
    installerSourceConfig: defaultInstallerSourceConfig(),
  };
}

const builders: Record<ScenarioId, () => ScenarioState> = {
  "new-user": buildNewUser,
  "claude-installed-codex-missing": buildClaudeInstalledCodexMissing,
  "all-installed-no-profile": buildAllInstalledNoProfile,
  "fully-configured": buildFullyConfigured,
  "network-failure": buildNetworkFailure,
};

let currentState: ScenarioState = builders["new-user"]();
let currentScenario: ScenarioId = "new-user";

export function loadScenario(id: ScenarioId): ScenarioState {
  currentState = builders[id]();
  currentScenario = id;
  return currentState;
}

export function getState(): ScenarioState {
  return currentState;
}

export function getCurrentScenarioId(): ScenarioId {
  return currentScenario;
}

export function resetState(): void {
  loadScenario(currentScenario);
}
