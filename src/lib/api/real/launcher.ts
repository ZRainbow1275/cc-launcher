import { invoke } from "@tauri-apps/api/core";

import {
  LaunchResult,
  OperationResult,
  TargetCli,
  TerminalCandidate,
} from "../contracts";
import type { SafetySummary } from "../mock/launcher";

export const launcherReal = {
  async list_terminals(): Promise<TerminalCandidate[]> {
    const raw = await invoke<unknown[]>("detect_terminals");
    return raw.map((r) => TerminalCandidate.parse(r));
  },

  async detect_terminals(): Promise<TerminalCandidate[]> {
    const raw = await invoke<unknown[]>("detect_terminals");
    return raw.map((r) => TerminalCandidate.parse(r));
  },

  async open_workdir(profile_id: string): Promise<OperationResult> {
    const raw = await invoke<unknown>("open_workdir", {
      profileId: profile_id,
    });
    return OperationResult.parse(raw);
  },

  async get_safety_summary(args: {
    profile_id: string;
    target_cli: TargetCli;
  }): Promise<SafetySummary> {
    TargetCli.parse(args.target_cli);
    return await invoke<SafetySummary>("get_safety_summary", {
      profileId: args.profile_id,
      targetCli: args.target_cli,
    });
  },

  async start_cli(args: {
    profile_id: string;
    target_cli: TargetCli;
    terminal_id?: string;
    cwd?: string;
  }): Promise<LaunchResult> {
    TargetCli.parse(args.target_cli);
    const raw = await invoke<unknown>("start_cli", {
      opts: {
        profileId: args.profile_id,
        targetCli: args.target_cli,
        terminalId: args.terminal_id ?? null,
        cwd: args.cwd ?? null,
        extraArgs: [],
      },
    });
    return LaunchResult.parse(raw);
  },
};

export type LauncherReal = typeof launcherReal;
