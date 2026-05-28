import { useTranslation } from "react-i18next";
import { CheckCircle2, Loader2, RefreshCw, XCircle } from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import type {
  InstallPhase,
  InstallProgress as InstallProgressEvent,
  LocalizedString,
  TargetCli,
} from "@/lib/api/contracts";

interface InstallProgressProps {
  cli: TargetCli;
  events: InstallProgressEvent[];
  isStreaming: boolean;
  installedVersion?: string;
  onCancel: () => void;
  onRetrySameMirror: () => void;
  onRetryDifferentMirror: () => void;
}

const PHASE_PERCENT_FALLBACK: Record<InstallPhase, number> = {
  "probing-registry": 10,
  "installing-node": 30,
  "installing-cli": 60,
  validating: 90,
  completed: 100,
  failed: 0,
};

function bestEffortPercent(events: InstallProgressEvent[]): number {
  for (let i = events.length - 1; i >= 0; i -= 1) {
    const evt = events[i];
    if (evt && typeof evt.percent === "number") return evt.percent;
    if (evt) return PHASE_PERCENT_FALLBACK[evt.phase];
  }
  return 0;
}

function synthesizedBytes(percent: number): { loaded: number; total: number } {
  const total = 24_000;
  const loaded = Math.round((percent / 100) * total);
  return { loaded, total };
}

function synthesizedSpeedKbps(percent: number): number {
  if (percent <= 0 || percent >= 100) return 0;
  return 480 + Math.round((percent % 10) * 18);
}

function localeKey(language: string): keyof LocalizedString {
  if (language.startsWith("zh")) return "zh";
  if (language.startsWith("ja")) return "ja";
  return "en";
}

function localizedText(
  message: LocalizedString | undefined,
  language: string,
): string {
  if (!message) return "";
  return message[localeKey(language)] || message.en || message.zh || message.ja;
}

function latestRegistry(events: InstallProgressEvent[]): string | undefined {
  for (let i = events.length - 1; i >= 0; i -= 1) {
    const registry = events[i]?.registry;
    if (registry) return registry;
  }
  return undefined;
}

export function InstallProgress({
  cli,
  events,
  isStreaming,
  installedVersion,
  onCancel,
  onRetrySameMirror,
  onRetryDifferentMirror,
}: InstallProgressProps) {
  const { t, i18n } = useTranslation();
  const last = events[events.length - 1];
  const failed = last?.phase === "failed";
  const completed = last?.phase === "completed";
  const percent = bestEffortPercent(events);
  const bytes = synthesizedBytes(percent);
  const speed = synthesizedSpeedKbps(percent);
  const statusMessage = localizedText(last?.message, i18n.language);
  const errorMessage = localizedText(last?.error?.message, i18n.language);
  const errorCause = last?.error?.cause?.trim();
  const registry = latestRegistry(events);

  return (
    <Card data-testid={`install-progress-${cli}`}>
      <CardContent className="pt-6 space-y-4">
        <div className="flex items-center justify-between gap-3">
          <p
            className="text-sm font-medium"
            data-testid={`install-progress-${cli}-current`}
          >
            {t("installer.step4.progress.current", { cli })}
          </p>
          {isStreaming && !completed && !failed ? (
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={onCancel}
              data-testid={`install-progress-${cli}-cancel`}
            >
              {t("installer.step4.progress.cancel")}
            </Button>
          ) : null}
        </div>

        {last ? (
          <div className="space-y-1">
            <div className="flex items-center justify-between text-xs">
              <span className="flex items-center gap-2">
                {completed ? (
                  <CheckCircle2 className="h-4 w-4 text-emerald-600" />
                ) : failed ? (
                  <XCircle className="h-4 w-4 text-red-600" />
                ) : (
                  <Loader2 className="h-4 w-4 animate-spin text-blue-500" />
                )}
                <span
                  className="font-medium"
                  data-testid={`install-progress-${cli}-phase`}
                  data-phase={last.phase}
                >
                  {last.phase}
                </span>
                <span className="text-muted-foreground">{statusMessage}</span>
              </span>
              <span className="font-mono text-muted-foreground">
                {percent}%
              </span>
            </div>
            {registry ? (
              <p
                className="text-[11px] text-muted-foreground font-mono"
                data-testid={`install-progress-${cli}-registry`}
              >
                {t("installer.step4.progress.source")} {registry}
              </p>
            ) : null}
            <div className="h-2 w-full bg-muted rounded-full overflow-hidden">
              <div
                data-testid={`install-progress-${cli}-bar`}
                className={`h-full transition-all ${
                  failed
                    ? "bg-red-500"
                    : completed
                      ? "bg-emerald-500"
                      : "bg-blue-500"
                }`}
                style={{ width: `${percent}%` }}
              />
            </div>
            <div className="flex items-center justify-between text-[11px] text-muted-foreground font-mono">
              <span data-testid={`install-progress-${cli}-bytes`}>
                {t("installer.step4.progress.bytes", {
                  loaded: bytes.loaded,
                  total: bytes.total,
                })}
              </span>
              <span data-testid={`install-progress-${cli}-speed`}>
                {t("installer.step4.progress.speed", { kbps: speed })}
              </span>
            </div>
          </div>
        ) : null}

        {completed && installedVersion ? (
          <Alert
            variant="default"
            data-testid={`install-progress-${cli}-success`}
          >
            <CheckCircle2 className="h-4 w-4" />
            <AlertTitle>
              {t("installer.step4.progress.success", {
                cli,
                version: installedVersion,
              })}
            </AlertTitle>
          </Alert>
        ) : null}

        {failed ? (
          <Alert
            variant="destructive"
            data-testid={`install-progress-${cli}-error`}
          >
            <XCircle className="h-4 w-4" />
            <AlertTitle>
              {t("installer.step4.progress.error", { cli })}
            </AlertTitle>
            <AlertDescription className="space-y-3">
              <p
                className="text-xs"
                data-testid={`install-progress-${cli}-cleaned`}
              >
                {t("installer.step4.progress.cleaned")}
              </p>
              {last.error ? (
                <div
                  className="space-y-2 rounded-md border border-red-200 bg-red-950/5 p-3 text-xs dark:border-red-900/70 dark:bg-red-950/30"
                  data-testid={`install-progress-${cli}-error-detail`}
                >
                  <div className="grid gap-1 sm:grid-cols-[120px_1fr]">
                    <span className="font-medium">
                      {t("installer.step4.progress.errorCode")}
                    </span>
                    <span
                      className="font-mono break-all"
                      data-testid={`install-progress-${cli}-error-code`}
                    >
                      {last.error.code}
                    </span>
                  </div>
                  {errorMessage ? (
                    <div className="grid gap-1 sm:grid-cols-[120px_1fr]">
                      <span className="font-medium">
                        {t("installer.step4.progress.errorMessage")}
                      </span>
                      <span
                        data-testid={`install-progress-${cli}-error-message`}
                      >
                        {errorMessage}
                      </span>
                    </div>
                  ) : null}
                  {registry ? (
                    <div className="grid gap-1 sm:grid-cols-[120px_1fr]">
                      <span className="font-medium">
                        {t("installer.step4.progress.attemptedSource")}
                      </span>
                      <span className="font-mono break-all">{registry}</span>
                    </div>
                  ) : null}
                  {errorCause ? (
                    <div className="space-y-1">
                      <span className="font-medium">
                        {t("installer.step4.progress.errorCause")}
                      </span>
                      <pre
                        className="max-h-40 overflow-auto whitespace-pre-wrap break-words rounded bg-background/80 p-2 font-mono text-[11px]"
                        data-testid={`install-progress-${cli}-error-cause`}
                      >
                        {errorCause}
                      </pre>
                    </div>
                  ) : null}
                </div>
              ) : null}
              <div className="flex flex-wrap gap-2">
                <Button
                  type="button"
                  size="sm"
                  variant="outline"
                  onClick={onRetrySameMirror}
                  data-testid={`install-progress-${cli}-retry-same`}
                >
                  <RefreshCw className="h-4 w-4 mr-1" />
                  {t("installer.step4.progress.retry")}
                </Button>
                <Button
                  type="button"
                  size="sm"
                  variant="default"
                  onClick={onRetryDifferentMirror}
                  data-testid={`install-progress-${cli}-retry-different`}
                >
                  {t("installer.step4.progress.retryDifferent")}
                </Button>
              </div>
            </AlertDescription>
          </Alert>
        ) : null}
      </CardContent>
    </Card>
  );
}
