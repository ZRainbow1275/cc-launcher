import { invoke } from "@tauri-apps/api/core";

import { FixAction, FixProgress, SystemProbeReport } from "../contracts";
import { makeChannelStream } from "./channel-iter";

export const systemProbeReal = {
  async run(): Promise<SystemProbeReport> {
    const raw = await invoke<unknown>("probe_system");
    return SystemProbeReport.parse(raw);
  },

  apply_fix(fix_action: FixAction): AsyncIterable<FixProgress> {
    const action = FixAction.parse(fix_action);
    const handle = makeChannelStream<FixProgress, void>(
      (channel) =>
        invoke<void>("apply_probe_fix", {
          action,
          channel,
        }),
      (raw) => raw.phase === "completed" || raw.phase === "failed",
    );
    handle.done.catch(() => {
      /* surface via channel */
    });
    async function* gen(): AsyncGenerator<FixProgress, void, void> {
      for await (const raw of handle.iterable) {
        yield FixProgress.parse(raw);
      }
    }
    return gen();
  },
};

export type SystemProbeReal = typeof systemProbeReal;
