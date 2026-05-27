import { z } from "zod";

export const TargetCli = z.enum(["claude", "codex"]);
export type TargetCli = z.infer<typeof TargetCli>;

export const LocalizedString = z
  .object({
    zh: z.string(),
    en: z.string(),
    ja: z.string(),
  })
  .strict();
export type LocalizedString = z.infer<typeof LocalizedString>;

export const OperationResult = z
  .object({
    success: z.boolean(),
    message: LocalizedString.optional(),
    errorCode: z.string().optional(),
  })
  .strict();
export type OperationResult = z.infer<typeof OperationResult>;

export const TypedError = z
  .object({
    code: z.string(),
    message: LocalizedString,
    cause: z.string().optional(),
    retryable: z.boolean().default(false),
  })
  .strict();
export type TypedError = z.infer<typeof TypedError>;

export const CliInstallStatus = z
  .object({
    cli: TargetCli,
    installed: z.boolean(),
    version: z.string().optional(),
    path: z.string().optional(),
    lastChecked: z.string().datetime(),
  })
  .strict();
export type CliInstallStatus = z.infer<typeof CliInstallStatus>;

export const InstallPhase = z.enum([
  "probing-registry",
  "installing-node",
  "installing-cli",
  "validating",
  "completed",
  "failed",
]);
export type InstallPhase = z.infer<typeof InstallPhase>;

export const InstallProgress = z
  .object({
    phase: InstallPhase,
    message: LocalizedString,
    percent: z.number().min(0).max(100).optional(),
    registry: z.string().optional(),
    error: TypedError.optional(),
  })
  .strict();
export type InstallProgress = z.infer<typeof InstallProgress>;

export const InstallerSourceConfig = z
  .object({
    npmRegistry: z.string().url().optional(),
    nodeDistMirror: z.string().url().optional(),
    gitForWindowsMirror: z.string().url().optional(),
  })
  .strict();
export type InstallerSourceConfig = z.infer<typeof InstallerSourceConfig>;

export const RegistryName = z.string().min(1);
export type RegistryName = z.infer<typeof RegistryName>;

export const RegistryProbe = z
  .object({
    name: RegistryName,
    url: z.string().url(),
    ok: z.boolean(),
    latencyMs: z.number().int().nonnegative(),
    statusCode: z.number().int().optional(),
    error: z.string().optional(),
  })
  .strict();
export type RegistryProbe = z.infer<typeof RegistryProbe>;

export const RegistryPickResult = z
  .object({
    candidates: z.array(RegistryProbe),
    chosen: z.string().url(),
    chosenName: RegistryName,
    chosenAt: z.string().datetime(),
    cached: z.boolean(),
  })
  .strict();
export type RegistryPickResult = z.infer<typeof RegistryPickResult>;

export const NodeStatus = z
  .object({
    installed: z.boolean(),
    version: z.string().optional(),
    path: z.string().optional(),
    isPrivateRuntime: z.boolean(),
    majorVersion: z.number().int().optional(),
  })
  .strict();
export type NodeStatus = z.infer<typeof NodeStatus>;

export const GitStatus = z
  .object({
    installed: z.boolean(),
    version: z.string().optional(),
    path: z.string().optional(),
  })
  .strict();
export type GitStatus = z.infer<typeof GitStatus>;

export const ProfileBase = z.object({
  id: z.string().min(1),
  target_cli: TargetCli,
  name: z.string().min(1),
  description: z.string().optional(),
  icon: z.string().optional(),
  icon_color: z.string().optional(),
  provider_id: z.string().nullable(),
  settings_json: z.string().default("{}"),
  sort_index: z.number().int().optional(),
  is_builtin: z.boolean().default(false),
  mcp_ids: z.array(z.string()).default([]),
  skill_ids: z.array(z.string()).default([]),
  created_at: z.number().int(),
  updated_at: z.number().int(),
});

export const Profile = ProfileBase.strict();
export type Profile = z.infer<typeof Profile>;

export const ProfileCreatePayload = z
  .object({
    target_cli: TargetCli,
    name: z.string().min(1),
    description: z.string().optional(),
    icon: z.string().optional(),
    icon_color: z.string().optional(),
    provider_id: z.string().nullable().optional(),
    settings_json: z.string().optional(),
    mcp_ids: z.array(z.string()).optional(),
    skill_ids: z.array(z.string()).optional(),
  })
  .strict();
export type ProfileCreatePayload = z.infer<typeof ProfileCreatePayload>;

export const ProfileUpdatePayload = z
  .object({
    name: z.string().min(1).optional(),
    description: z.string().optional(),
    icon: z.string().optional(),
    icon_color: z.string().optional(),
    provider_id: z.string().nullable().optional(),
    settings_json: z.string().optional(),
    sort_index: z.number().int().optional(),
    mcp_ids: z.array(z.string()).optional(),
    skill_ids: z.array(z.string()).optional(),
  })
  .strict();
export type ProfileUpdatePayload = z.infer<typeof ProfileUpdatePayload>;

export const SwitchResult = z
  .object({
    success: z.boolean(),
    profileId: z.string(),
    targetCli: TargetCli,
    backupDir: z.string().optional(),
    error: TypedError.optional(),
    switchedAt: z.string().datetime(),
  })
  .strict();
export type SwitchResult = z.infer<typeof SwitchResult>;

export const TerminalKind = z.enum([
  "wt",
  "cmd",
  "powershell",
  "terminal-app",
  "iterm2",
  "gnome-terminal",
  "konsole",
  "xterm",
]);
export type TerminalKind = z.infer<typeof TerminalKind>;

export const TerminalCandidate = z
  .object({
    id: z.string(),
    kind: TerminalKind,
    displayName: z.string(),
    path: z.string().optional(),
    installed: z.boolean(),
    isDefault: z.boolean(),
  })
  .strict();
export type TerminalCandidate = z.infer<typeof TerminalCandidate>;

export const LaunchResult = z
  .object({
    success: z.boolean(),
    profileId: z.string(),
    targetCli: TargetCli,
    terminalId: z.string(),
    pid: z.number().int().optional(),
    cwd: z.string(),
    launchedAt: z.string().datetime(),
    error: TypedError.optional(),
  })
  .strict();
export type LaunchResult = z.infer<typeof LaunchResult>;

export const ProbeStatus = z.enum([
  "green",
  "yellow",
  "red",
  "missing",
  "unknown",
]);
export type ProbeStatus = z.infer<typeof ProbeStatus>;

export const FixAction = z.discriminatedUnion("kind", [
  z
    .object({
      kind: z.literal("installNode"),
      targetLtsMajor: z.number().int(),
    })
    .strict(),
  z.object({ kind: z.literal("installGit") }).strict(),
  z.object({ kind: z.literal("cleanEnvVar"), varName: z.string() }).strict(),
  z.object({ kind: z.literal("createWorkdir"), path: z.string() }).strict(),
  z.object({ kind: z.literal("openHomeDir") }).strict(),
  z
    .object({
      kind: z.literal("injectPathEntries"),
      entries: z.array(z.string()),
    })
    .strict(),
  z
    .object({
      kind: z.literal("externalLink"),
      url: z.string().url(),
      labelKey: z.string(),
    })
    .strict(),
]);
export type FixAction = z.infer<typeof FixAction>;

export const ProbeItem = z
  .object({
    id: z.string(),
    nameKey: z.string(),
    status: ProbeStatus,
    value: z.unknown(),
    messageKey: z.string(),
    fixAction: FixAction.nullable(),
    elapsedMs: z.number().int().nonnegative(),
    group: z.enum(["system", "runtime", "env", "network", "workdir"]),
  })
  .strict();
export type ProbeItem = z.infer<typeof ProbeItem>;

export const SystemProbeReport = z
  .object({
    overallStatus: ProbeStatus,
    items: z.array(ProbeItem),
    generatedAt: z.string().datetime(),
    probeVersion: z.number().int(),
  })
  .strict();
export type SystemProbeReport = z.infer<typeof SystemProbeReport>;

export const FixProgress = z
  .object({
    fixId: z.string(),
    phase: z.enum(["starting", "running", "validating", "completed", "failed"]),
    message: LocalizedString,
    percent: z.number().min(0).max(100).optional(),
    error: TypedError.optional(),
  })
  .strict();
export type FixProgress = z.infer<typeof FixProgress>;

export const L1RuleCategory = z.enum([
  "DangerousFilesystem",
  "PrivilegeEscalation",
  "NetworkExposure",
  "SuspiciousCommand",
  "CliRiskyFlag",
]);
export type L1RuleCategory = z.infer<typeof L1RuleCategory>;

export const L1Rule = z
  .object({
    id: z.string(),
    category: L1RuleCategory,
    pattern: z.string(),
    titleKey: z.string(),
    descriptionKey: z.string(),
    enabled: z.boolean(),
    unlockable: z.boolean(),
    unlockedUntil: z.string().datetime().nullable(),
    updatedAt: z.string().datetime(),
  })
  .strict();
export type L1Rule = z.infer<typeof L1Rule>;

export const L2Redline = z
  .object({
    id: z.string(),
    category: z.enum([
      "DiskWipe",
      "BootCritical",
      "HostsFile",
      "LauncherSelf",
      "ReverseShell",
      "Ransomware",
      "SudoDestructive",
      "CwdSystemRoot",
    ]),
    pattern: z.string(),
    descriptionKey: z.string(),
    matchType: z.enum(["regex", "substring"]),
  })
  .strict();
export type L2Redline = z.infer<typeof L2Redline>;

export const SandboxLevel = z.enum(["strict", "medium"]);
export type SandboxLevel = z.infer<typeof SandboxLevel>;

export const UnlockRequest = z
  .object({
    ruleId: z.string(),
    keyword: z.string(),
  })
  .strict();
export type UnlockRequest = z.infer<typeof UnlockRequest>;

export const OnboardingAnswers = z
  .object({
    locale: z.enum(["zh", "en", "ja"]),
    uiMode: z.enum(["novice", "expert"]),
    enableSandbox: z.boolean(),
    acceptedRedlines: z.boolean(),
    preferredCli: TargetCli.optional(),
  })
  .strict();
export type OnboardingAnswers = z.infer<typeof OnboardingAnswers>;

export const OnboardingState = z
  .object({
    completed: z.boolean(),
    completedAt: z.string().datetime().nullable(),
    answers: OnboardingAnswers.nullable(),
  })
  .strict();
export type OnboardingState = z.infer<typeof OnboardingState>;

export const UiMode = z.enum(["novice", "expert"]);
export type UiMode = z.infer<typeof UiMode>;

export const Locale = z.enum(["zh", "en", "ja"]);
export type Locale = z.infer<typeof Locale>;

export const ActiveProfileMap = z.object({
  claude: z.string().nullable(),
  codex: z.string().nullable(),
});
export type ActiveProfileMap = z.infer<typeof ActiveProfileMap>;

export const InstallerOpts = z
  .object({
    registry: z.string().url().optional(),
    skipNodeCheck: z.boolean().optional(),
  })
  .strict()
  .optional();
export type InstallerOpts = z.infer<typeof InstallerOpts>;

export const ScenarioId = z.enum([
  "new-user",
  "claude-installed-codex-missing",
  "all-installed-no-profile",
  "fully-configured",
  "network-failure",
]);
export type ScenarioId = z.infer<typeof ScenarioId>;

export interface MockController {
  setScenario(id: ScenarioId): void;
  resetScenario(): void;
  getScenario(): ScenarioId;
  setMockDelay(ms: number): void;
  getMockDelay(): number;
  enableFailure(domain: string, command: string): void;
  disableFailure(domain: string, command: string): void;
  clearFailures(): void;
}
