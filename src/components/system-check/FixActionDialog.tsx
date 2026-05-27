import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import { Loader2, Wrench, XCircle, CircleCheck } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { systemProbe } from "@/lib/api/mock";
import type {
  FixAction,
  FixProgress,
  LocalizedString,
  ProbeItem,
  Locale,
  SystemProbeReport,
} from "@/lib/api/contracts";

import { SYSTEM_PROBE_QUERY_KEY } from "./SystemCheckDashboard";

type Phase = FixProgress["phase"] | "idle";

const REVERSIBLE_KINDS = new Set<FixAction["kind"]>([
  "cleanEnvVar",
  "openHomeDir",
  "injectPathEntries",
]);

function actionTitleKey(action: FixAction): string {
  switch (action.kind) {
    case "installNode":
      return "systemCheck.fix.installNode.title";
    case "installGit":
      return "systemCheck.fix.installGit.title";
    case "cleanEnvVar":
      return "systemCheck.fix.cleanEnvVar.title";
    case "createWorkdir":
      return "systemCheck.fix.createWorkdir.title";
    case "openHomeDir":
      return "systemCheck.fix.openHomeDir.title";
    case "injectPathEntries":
      return "systemCheck.fix.injectPathEntries.title";
    case "externalLink":
      return "systemCheck.fix.externalLink.title";
  }
}

function actionDescriptionKey(action: FixAction): string {
  switch (action.kind) {
    case "installNode":
      return "systemCheck.fix.installNode.description";
    case "installGit":
      return "systemCheck.fix.installGit.description";
    case "cleanEnvVar":
      return "systemCheck.fix.cleanEnvVar.description";
    case "createWorkdir":
      return "systemCheck.fix.createWorkdir.description";
    case "openHomeDir":
      return "systemCheck.fix.openHomeDir.description";
    case "injectPathEntries":
      return "systemCheck.fix.injectPathEntries.description";
    case "externalLink":
      return "systemCheck.fix.externalLink.description";
  }
}

const BACKEND_FIX_MESSAGE_KEYS: Record<string, string> = {
  "fix.starting": "systemCheck.fix.phase.starting",
  "fix.running": "systemCheck.fix.phase.running",
  "fix.validating": "systemCheck.fix.phase.validating",
  "fix.completed": "systemCheck.fix.phase.completed",
  "fix.failed": "systemCheck.fix.phase.failed",
};

type TranslateFn = (key: string, options?: { defaultValue?: string }) => string;

function pickLocalized(
  msg: LocalizedString,
  locale: string,
  translate: TranslateFn,
): string {
  const known: Locale = locale === "en" || locale === "ja" ? locale : "zh";
  const raw = msg[known];
  const mapped = BACKEND_FIX_MESSAGE_KEYS[raw];
  if (mapped) return translate(mapped, { defaultValue: raw });
  if (raw.startsWith("FIX_")) {
    return translate(`systemCheck.fix.errors.${raw}`, { defaultValue: raw });
  }
  return translate(raw, { defaultValue: raw });
}

function isBlockingStatus(status: ProbeItem["status"]): boolean {
  return status === "red" || status === "missing";
}

interface FixActionDialogProps {
  item: ProbeItem;
  action: FixAction;
  batchRemaining: number;
  onClose: () => void;
  onAbortBatch?: () => void;
}

export function FixActionDialog({
  item,
  action,
  batchRemaining,
  onClose,
  onAbortBatch,
}: FixActionDialogProps) {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();
  const [phase, setPhase] = useState<Phase>("idle");
  const [percent, setPercent] = useState<number>(0);
  const [message, setMessage] = useState<string>("");
  const [errorText, setErrorText] = useState<string | null>(null);
  const cancelRef = useRef(false);

  const reversible = REVERSIBLE_KINDS.has(action.kind);
  const completed = phase === "completed";
  const failed = phase === "failed";
  const inFlight =
    phase === "starting" || phase === "running" || phase === "validating";

  const runFix = useCallback(async () => {
    cancelRef.current = false;
    setPhase("starting");
    setPercent(0);
    setMessage("");
    setErrorText(null);
    let lastPhase: Phase = "idle";

    try {
      for await (const event of systemProbe.apply_fix(action)) {
        if (cancelRef.current) break;
        lastPhase = event.phase;
        if (event.phase === "completed") {
          setPhase("validating");
          setPercent(95);
          setMessage(t("systemCheck.fix.phase.validating"));
          continue;
        }
        setPhase(event.phase);
        if (typeof event.percent === "number") {
          setPercent(event.percent);
        }
        setMessage(pickLocalized(event.message, i18n.language, t));
        if (event.phase === "failed" && event.error) {
          setErrorText(pickLocalized(event.error.message, i18n.language, t));
        }
      }
      if (!cancelRef.current && lastPhase === "completed") {
        const report = await queryClient.fetchQuery<SystemProbeReport>({
          queryKey: SYSTEM_PROBE_QUERY_KEY,
          queryFn: () => systemProbe.run(),
        });
        const refreshedItem = report.items.find((it) => it.id === item.id);
        if (refreshedItem && isBlockingStatus(refreshedItem.status)) {
          setPhase("failed");
          setPercent(100);
          setMessage(t("systemCheck.fix.phase.failed"));
          setErrorText(
            t("systemCheck.fix.errors.FIX_VALIDATION_FAILED", {
              defaultValue:
                "The repair ran, but the follow-up check is still red. Rerun the check after the installer or system dialog finishes.",
            }),
          );
          return;
        }
        setPhase("completed");
        setPercent(100);
        setMessage(t("systemCheck.fix.phase.completed"));
      }
    } catch (err) {
      setPhase("failed");
      setErrorText(String(err));
    }
  }, [action, i18n.language, item.id, queryClient, t]);

  useEffect(() => {
    void runFix();
    return () => {
      cancelRef.current = true;
    };
  }, [runFix]);

  useEffect(() => {
    if (phase === "completed") {
      toast.success(t("systemCheck.fix.toast.success"), {
        description: t(item.nameKey, { defaultValue: item.id }),
      });
      void queryClient.invalidateQueries({ queryKey: SYSTEM_PROBE_QUERY_KEY });
    } else if (phase === "failed") {
      toast.error(t("systemCheck.fix.toast.failed"), {
        description: errorText ?? t(item.nameKey, { defaultValue: item.id }),
      });
    }
  }, [phase, errorText, item.nameKey, item.id, queryClient, t]);

  const handleCancel = (): void => {
    cancelRef.current = true;
    onClose();
  };

  const handleClose = (open: boolean): void => {
    if (!open) onClose();
  };

  const titleKey = useMemo(() => actionTitleKey(action), [action]);
  const descriptionKey = useMemo(() => actionDescriptionKey(action), [action]);

  return (
    <Dialog open onOpenChange={handleClose}>
      <DialogContent
        zIndex="nested"
        data-testid="fix-action-dialog"
        className="sm:rounded-lg overflow-hidden"
      >
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            {failed ? (
              <XCircle className="h-5 w-5 text-red-600" />
            ) : completed ? (
              <CircleCheck className="h-5 w-5 text-emerald-600" />
            ) : (
              <Wrench className="h-5 w-5 text-primary" />
            )}
            {t(titleKey, { defaultValue: titleKey })}
          </DialogTitle>
          <DialogDescription>
            {t(descriptionKey, { defaultValue: descriptionKey })}
          </DialogDescription>
        </DialogHeader>
        <div className="px-6 py-4 space-y-3">
          <div className="text-sm">
            <span className="text-muted-foreground">
              {t("systemCheck.fix.target")}:
            </span>{" "}
            <span className="font-medium">
              {t(item.nameKey, { defaultValue: item.id })}
            </span>
          </div>
          {batchRemaining > 0 ? (
            <div className="text-xs text-muted-foreground">
              {t("systemCheck.fix.batchRemaining", { count: batchRemaining })}
            </div>
          ) : null}
          <div
            data-testid="fix-progress-bar"
            className="h-2 w-full rounded-full bg-muted overflow-hidden"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={100}
            aria-valuenow={percent}
          >
            <div
              className={`h-full transition-all duration-300 ${
                failed
                  ? "bg-red-500"
                  : completed
                    ? "bg-emerald-500"
                    : "bg-primary"
              }`}
              style={{ width: `${percent}%` }}
            />
          </div>
          <div
            className="flex items-center gap-2 text-sm"
            data-testid="fix-progress-status"
          >
            {inFlight ? <Loader2 className="h-4 w-4 animate-spin" /> : null}
            <span>
              {message ||
                (phase === "idle"
                  ? t("systemCheck.fix.phase.idle")
                  : t(`systemCheck.fix.phase.${phase}`))}
            </span>
            <span className="ml-auto font-mono text-xs">{percent}%</span>
          </div>
          {failed && errorText ? (
            <p className="text-xs text-red-600">{errorText}</p>
          ) : null}
        </div>
        <DialogFooter>
          {inFlight ? (
            reversible ? (
              <Button variant="outline" onClick={handleCancel}>
                {t("common.cancel")}
              </Button>
            ) : (
              <Button variant="outline" disabled>
                {t("systemCheck.fix.cannotCancel")}
              </Button>
            )
          ) : (
            <>
              {failed ? (
                <Button variant="outline" onClick={runFix}>
                  {t("systemCheck.fix.retry")}
                </Button>
              ) : null}
              {batchRemaining > 0 && onAbortBatch ? (
                <Button
                  variant="outline"
                  onClick={onAbortBatch}
                  data-testid="fix-dialog-cancel-batch"
                >
                  {t("systemCheck.fix.cancelBatch", {
                    defaultValue: t("common.cancel"),
                  })}
                </Button>
              ) : null}
              <Button onClick={onClose} data-testid="fix-dialog-action">
                {batchRemaining > 0
                  ? t("systemCheck.fix.next")
                  : t("common.close")}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
