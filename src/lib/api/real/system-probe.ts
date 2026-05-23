import { invoke } from "@tauri-apps/api/core";

import { FixAction, FixProgress, SystemProbeReport } from "../contracts";
import { makeChannelStream } from "./channel-iter";

interface BackendFixProgress {
  fixId: string;
  phase: "starting" | "running" | "validating" | "completed" | "failed";
  messageKey: string;
  percent?: number;
  errorCode?: string;
}

function adaptBackendProgress(raw: BackendFixProgress): FixProgress {
  return FixProgress.parse({
    fixId: raw.fixId,
    phase: raw.phase,
    message: {
      zh: raw.messageKey,
      en: raw.messageKey,
      ja: raw.messageKey,
    },
    percent: raw.percent,
    error: raw.errorCode
      ? {
          code: raw.errorCode,
          message: {
            zh: raw.errorCode,
            en: raw.errorCode,
            ja: raw.errorCode,
          },
          retryable: false,
        }
      : undefined,
  });
}

export const systemProbeReal = {
  async run(): Promise<SystemProbeReport> {
    const raw = await invoke<unknown>("probe_system");
    return SystemProbeReport.parse(raw);
  },

  apply_fix(fix_action: FixAction): AsyncIterable<FixProgress> {
    const action = FixAction.parse(fix_action);
    const handle = makeChannelStream<BackendFixProgress, void>(
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
        yield adaptBackendProgress(raw);
      }
    }
    return gen();
  },
};

export type SystemProbeReal = typeof systemProbeReal;
