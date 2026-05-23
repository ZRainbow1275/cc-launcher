import { invoke } from "@tauri-apps/api/core";

import {
  CliInstallStatus,
  InstallProgress,
  InstallerOpts,
  NodeStatus,
  OperationResult,
  RegistryPickResult,
  TargetCli,
} from "../contracts";
import { makeChannelStream } from "./channel-iter";

const isTerminalPhase = (p: InstallProgress): boolean =>
  p.phase === "completed" || p.phase === "failed";

export const installerReal = {
  async detect_cli(cli: TargetCli): Promise<CliInstallStatus> {
    TargetCli.parse(cli);
    const raw = await invoke<unknown>("detect_cli", { cli });
    return CliInstallStatus.parse(raw);
  },

  async detect_node(): Promise<NodeStatus> {
    const raw = await invoke<unknown>("detect_node");
    return NodeStatus.parse(raw);
  },

  install_node(): AsyncIterable<InstallProgress> {
    const handle = makeChannelStream<InstallProgress, NodeStatus>(
      (onProgress) => invoke<NodeStatus>("install_node", { onProgress }),
      isTerminalPhase,
    );
    handle.done.catch(() => {
      /* errors surface through the channel; nothing to do here */
    });
    return handle.iterable;
  },

  install_cli(
    cli: TargetCli,
    opts?: InstallerOpts,
  ): AsyncIterable<InstallProgress> {
    TargetCli.parse(cli);
    if (opts !== undefined) InstallerOpts.parse(opts);
    const handle = makeChannelStream<InstallProgress, CliInstallStatus>(
      (onProgress) =>
        invoke<CliInstallStatus>("install_cli", {
          cli,
          opts: opts ?? null,
          onProgress,
        }),
      isTerminalPhase,
    );
    handle.done.catch(() => {
      /* errors surface through the channel */
    });
    return handle.iterable;
  },

  // install_git removed in D-10 — Git install flows through
  // `systemProbe.apply_fix({ kind: "installGit" })`. See phase-c-parity.md §C.

  async uninstall_cli(cli: TargetCli): Promise<OperationResult> {
    TargetCli.parse(cli);
    const raw = await invoke<unknown>("uninstall_cli", { cli });
    return OperationResult.parse(raw);
  },

  async smart_pick_registry(): Promise<RegistryPickResult> {
    const raw = await invoke<unknown>("smart_pick_registry");
    return RegistryPickResult.parse(raw);
  },
};

export type InstallerReal = typeof installerReal;
