import { invoke } from "@tauri-apps/api/core";

import {
  LaunchResult,
  OperationResult,
  TargetCli,
  TerminalCandidate,
  TerminalKind,
} from "../contracts";
import type { SafetySummary } from "../mock/launcher";

interface BackendTerminalInfo {
  kind: string;
  binaryPath: string;
  displayName: string;
  isDefault: boolean;
}

const KIND_BACK_TO_FRONT: Record<string, TerminalKind> = {
  windowsTerminal: "wt",
  cmd: "cmd",
  powerShell: "powershell",
  macTerminal: "terminal-app",
  iTerm2: "iterm2",
  gnomeTerminal: "gnome-terminal",
  konsole: "konsole",
  xterm: "xterm",
};

function adaptTerminalInfo(raw: BackendTerminalInfo): TerminalCandidate {
  const kind = (KIND_BACK_TO_FRONT[raw.kind] ?? "cmd") as TerminalKind;
  return TerminalCandidate.parse({
    id: raw.kind,
    kind,
    displayName: raw.displayName,
    path: raw.binaryPath,
    installed: true,
    isDefault: raw.isDefault,
  });
}

interface BackendSafetySummary {
  sandboxLevel: string;
  workdir: string;
  flagsApplied: string[];
  envKeysSet: string[];
  redlinesActive: boolean;
}

interface BackendStartCliResult {
  pid: number;
  workdir: string;
  terminal: string;
  safety: BackendSafetySummary;
}

export const launcherReal = {
  async list_terminals(): Promise<TerminalCandidate[]> {
    const raw = await invoke<BackendTerminalInfo[]>("detect_terminals");
    return raw.map(adaptTerminalInfo);
  },

  async detect_terminals(): Promise<TerminalCandidate[]> {
    const raw = await invoke<BackendTerminalInfo[]>("detect_terminals");
    return raw.map(adaptTerminalInfo);
  },

  async open_workdir(profile_id: string): Promise<OperationResult> {
    try {
      await invoke<string>("open_workdir", { profileId: profile_id });
      return OperationResult.parse({ success: true });
    } catch (err) {
      return OperationResult.parse({
        success: false,
        errorCode: typeof err === "string" ? err : "OPEN_WORKDIR_FAILED",
      });
    }
  },

  async get_safety_summary(args: {
    profile_id: string;
    target_cli: TargetCli;
  }): Promise<SafetySummary> {
    TargetCli.parse(args.target_cli);
    const raw = await invoke<BackendSafetySummary>("get_safety_summary", {
      cli: args.target_cli,
      profileId: args.profile_id,
    });
    return {
      profileId: args.profile_id,
      targetCli: args.target_cli,
      flags: raw.flagsApplied,
      cwd: raw.workdir,
      cwdDisplay: raw.workdir,
      l1ActiveCount: raw.envKeysSet.length,
      l2RedlineCount: raw.redlinesActive ? 16 : 0,
    };
  },

  async start_cli(args: {
    profile_id: string;
    target_cli: TargetCli;
    terminal_id?: string;
    cwd?: string;
  }): Promise<LaunchResult> {
    TargetCli.parse(args.target_cli);
    try {
      const raw = await invoke<BackendStartCliResult>("start_cli", {
        opts: {
          cli: args.target_cli,
          profileId: args.profile_id,
          terminal: args.terminal_id ?? null,
          extraArgs: [],
        },
      });
      return LaunchResult.parse({
        success: true,
        profileId: args.profile_id,
        targetCli: args.target_cli,
        terminalId: raw.terminal,
        pid: raw.pid,
        cwd: raw.workdir,
        launchedAt: new Date().toISOString(),
      });
    } catch (err) {
      const code =
        typeof err === "object" && err && "kind" in err
          ? String((err as { kind: string }).kind)
          : "LAUNCH_FAILED";
      return LaunchResult.parse({
        success: false,
        profileId: args.profile_id,
        targetCli: args.target_cli,
        terminalId: args.terminal_id ?? "",
        cwd: args.cwd ?? "",
        launchedAt: new Date().toISOString(),
        error: {
          code,
          message: {
            zh: `启动失败: ${code}`,
            en: `Launch failed: ${code}`,
            ja: `起動に失敗: ${code}`,
          },
          retryable: false,
        },
      });
    }
  },
};

export type LauncherReal = typeof launcherReal;
