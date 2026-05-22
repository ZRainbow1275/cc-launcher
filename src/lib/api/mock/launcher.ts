import {
  LaunchResult,
  OperationResult,
  TargetCli,
  TerminalCandidate,
} from "../contracts";
import { errors } from "./fixtures/i18n";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "launcher";

function nowIso(): string {
  return new Date().toISOString();
}

function cwdFor(profileId: string): string {
  return `C:\\Users\\you\\cc-launcher-projects\\${profileId}`;
}

function cwdDisplayFor(profileId: string): string {
  return `~/cc-launcher-projects/${profileId}`;
}

export interface SafetySummary {
  profileId: string;
  targetCli: TargetCli;
  flags: string[];
  cwd: string;
  cwdDisplay: string;
  l1ActiveCount: number;
  l2RedlineCount: number;
}

function buildFlagsForCli(
  target_cli: TargetCli,
  cwd: string,
  profileName: string,
): string[] {
  if (target_cli === "claude") {
    return [
      "--permission-mode default",
      "--strict-mcp-config",
      `--add-dir ${cwd}`,
      "--append-system-prompt-file <safe-prompt-path>",
    ];
  }
  return [`--profile ${profileName}`];
}

export const launcherMock = {
  async list_terminals(): Promise<TerminalCandidate[]> {
    if (shouldFail(DOMAIN, "list_terminals")) throw errors.noTerminal;
    await delay();
    return getState().terminals.map((t) => TerminalCandidate.parse(t));
  },

  async detect_terminals(): Promise<TerminalCandidate[]> {
    if (shouldFail(DOMAIN, "detect_terminals")) throw errors.noTerminal;
    await delay();
    return getState().terminals.map((t) => TerminalCandidate.parse(t));
  },

  async open_workdir(profile_id: string): Promise<OperationResult> {
    if (!profile_id) throw errors.profileNotFound;
    if (shouldFail(DOMAIN, "open_workdir")) throw errors.networkUnreachable;
    await delay();
    return OperationResult.parse({ success: true });
  },

  async get_safety_summary(args: {
    profile_id: string;
    target_cli: TargetCli;
  }): Promise<SafetySummary> {
    TargetCli.parse(args.target_cli);
    if (shouldFail(DOMAIN, "get_safety_summary"))
      throw errors.networkUnreachable;
    await delay();
    const state = getState();
    const profile = state.profiles.find(
      (p) => p.id === args.profile_id && p.target_cli === args.target_cli,
    );
    const cwd = cwdFor(args.profile_id);
    const flags = buildFlagsForCli(
      args.target_cli,
      cwd,
      profile?.name ?? args.profile_id,
    );
    const l1ActiveCount = state.l1Rules.filter((r) => r.enabled).length;
    const l2RedlineCount = 16;
    return {
      profileId: args.profile_id,
      targetCli: args.target_cli,
      flags,
      cwd,
      cwdDisplay: cwdDisplayFor(args.profile_id),
      l1ActiveCount,
      l2RedlineCount,
    };
  },

  async start_cli(args: {
    profile_id: string;
    target_cli: TargetCli;
    terminal_id?: string;
    cwd?: string;
  }): Promise<LaunchResult> {
    TargetCli.parse(args.target_cli);
    const { profile_id, target_cli, terminal_id } = args;
    if (shouldFail(DOMAIN, "start_cli")) {
      return LaunchResult.parse({
        success: false,
        profileId: profile_id,
        targetCli: target_cli,
        terminalId: terminal_id ?? "",
        cwd: args.cwd ?? cwdFor(profile_id),
        launchedAt: nowIso(),
        error: errors.l2RedlineBlocked,
      });
    }
    await delay();
    const state = getState();
    const profile = state.profiles.find(
      (p) => p.id === profile_id && p.target_cli === target_cli,
    );
    if (!profile) {
      return LaunchResult.parse({
        success: false,
        profileId: profile_id,
        targetCli: target_cli,
        terminalId: terminal_id ?? "",
        cwd: args.cwd ?? cwdFor(profile_id),
        launchedAt: nowIso(),
        error: errors.profileNotFound,
      });
    }
    const terminal =
      state.terminals.find((t) =>
        terminal_id ? t.id === terminal_id : t.isDefault,
      ) ?? state.terminals.find((t) => t.installed);
    if (!terminal || !terminal.installed) {
      return LaunchResult.parse({
        success: false,
        profileId: profile_id,
        targetCli: target_cli,
        terminalId: terminal_id ?? "",
        cwd: args.cwd ?? cwdFor(profile_id),
        launchedAt: nowIso(),
        error: errors.noTerminal,
      });
    }
    return LaunchResult.parse({
      success: true,
      profileId: profile_id,
      targetCli: target_cli,
      terminalId: terminal.id,
      pid: 48000 + Math.floor(Math.random() * 1000),
      cwd: args.cwd ?? cwdFor(profile_id),
      launchedAt: nowIso(),
    });
  },
};

export type LauncherMock = typeof launcherMock;
