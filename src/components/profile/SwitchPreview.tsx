import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { AlertCircle, ArrowRight, Minus, Plus } from "lucide-react";
import type { Profile, SwitchResult } from "@/lib/api/contracts";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface SwitchPreviewProps {
  open: boolean;
  current: Profile | null;
  next: Profile | null;
  isSwitching: boolean;
  failure: SwitchResult | null;
  onConfirm: () => void;
  onRetry: () => void;
  onClose: () => void;
}

interface DiffSet {
  added: string[];
  removed: string[];
  kept: string[];
}

function diffArrays(curr: string[], nxt: string[]): DiffSet {
  const a = new Set(curr);
  const b = new Set(nxt);
  return {
    added: nxt.filter((x) => !a.has(x)),
    removed: curr.filter((x) => !b.has(x)),
    kept: nxt.filter((x) => a.has(x)),
  };
}

function compactJson(input: string): string {
  try {
    return JSON.stringify(JSON.parse(input || "{}"));
  } catch {
    return input || "{}";
  }
}

export function SwitchPreview({
  open,
  current,
  next,
  isSwitching,
  failure,
  onConfirm,
  onRetry,
  onClose,
}: SwitchPreviewProps) {
  const { t } = useTranslation();

  const mcpDiff = useMemo<DiffSet>(
    () => diffArrays(current?.mcp_ids ?? [], next?.mcp_ids ?? []),
    [current, next],
  );
  const skillsDiff = useMemo<DiffSet>(
    () => diffArrays(current?.skill_ids ?? [], next?.skill_ids ?? []),
    [current, next],
  );

  const providerChanged =
    (current?.provider_id ?? null) !== (next?.provider_id ?? null);

  const currentSettings = compactJson(current?.settings_json ?? "{}");
  const nextSettings = compactJson(next?.settings_json ?? "{}");
  const settingsChanged = currentSettings !== nextSettings;

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) onClose();
      }}
    >
      <DialogContent
        className="max-w-2xl"
        data-testid="profile-switch-preview-dialog"
      >
        <DialogHeader>
          <DialogTitle>{t("profile.switch.title")}</DialogTitle>
          <DialogDescription>
            {t("profile.switch.subtitle", {
              from: current?.name ?? t("profile.switch.none"),
              to: next?.name ?? "",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 text-sm">
          <Section title={t("profile.switch.providerSection")}>
            {providerChanged ? (
              <div
                className="flex items-center gap-2"
                data-testid="diff-provider"
              >
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs">
                  {current?.provider_id ?? t("profile.switch.none")}
                </code>
                <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
                <code className="rounded bg-muted px-1.5 py-0.5 text-xs">
                  {next?.provider_id ?? t("profile.switch.none")}
                </code>
              </div>
            ) : (
              <p
                className="text-xs text-muted-foreground"
                data-testid="diff-provider-unchanged"
              >
                {t("profile.switch.unchanged")}
              </p>
            )}
          </Section>

          <Section title={t("profile.switch.mcpSection")}>
            <DiffBlock
              added={mcpDiff.added}
              removed={mcpDiff.removed}
              testId="diff-mcp"
              emptyMessage={t("profile.switch.unchanged")}
            />
          </Section>

          <Section title={t("profile.switch.skillsSection")}>
            <DiffBlock
              added={skillsDiff.added}
              removed={skillsDiff.removed}
              testId="diff-skills"
              emptyMessage={t("profile.switch.unchanged")}
            />
          </Section>

          <Section title={t("profile.switch.settingsSection")}>
            {settingsChanged ? (
              <pre
                className="max-h-32 overflow-auto whitespace-pre-wrap break-all rounded bg-muted px-2 py-1 text-[11px]"
                data-testid="diff-settings"
              >
                <span className="text-rose-600 dark:text-rose-400">
                  - {currentSettings}
                </span>
                {"\n"}
                <span className="text-emerald-600 dark:text-emerald-400">
                  + {nextSettings}
                </span>
              </pre>
            ) : (
              <p
                className="text-xs text-muted-foreground"
                data-testid="diff-settings-unchanged"
              >
                {t("profile.switch.unchanged")}
              </p>
            )}
          </Section>

          {failure && (
            <div
              className="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-xs"
              data-testid="switch-failure"
            >
              <div className="flex items-center gap-2 font-medium text-destructive">
                <AlertCircle className="h-4 w-4" />
                {t("profile.switch.failureTitle")}
              </div>
              <p className="mt-1 text-destructive/90">
                {failure.error?.message?.zh ??
                  failure.error?.code ??
                  t("profile.switch.failureUnknown")}
              </p>
              {failure.backupDir && (
                <p
                  className="mt-2 break-all text-muted-foreground"
                  data-testid="switch-backup-dir"
                >
                  {t("profile.switch.backupDir")}:{" "}
                  <code className="rounded bg-background px-1 py-0.5">
                    {failure.backupDir}
                  </code>
                </p>
              )}
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isSwitching}>
            {t("common.cancel")}
          </Button>
          {failure ? (
            <Button
              onClick={onRetry}
              disabled={isSwitching}
              data-testid="switch-retry"
            >
              {t("profile.switch.retry")}
            </Button>
          ) : (
            <Button
              onClick={onConfirm}
              disabled={isSwitching || !next}
              data-testid="switch-confirm"
            >
              {isSwitching
                ? t("profile.switch.switching")
                : t("profile.switch.confirm")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function Section({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        {title}
      </h4>
      {children}
    </div>
  );
}

interface DiffBlockProps {
  added: string[];
  removed: string[];
  testId: string;
  emptyMessage: string;
}

function DiffBlock({ added, removed, testId, emptyMessage }: DiffBlockProps) {
  if (added.length === 0 && removed.length === 0) {
    return (
      <p
        className="text-xs text-muted-foreground"
        data-testid={`${testId}-unchanged`}
      >
        {emptyMessage}
      </p>
    );
  }
  return (
    <div className="flex flex-wrap gap-1.5" data-testid={testId}>
      {added.map((id) => (
        <Badge
          key={`add-${id}`}
          variant="outline"
          className={cn(
            "border-emerald-500/40 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
          )}
          data-testid={`${testId}-add-${id}`}
        >
          <Plus className="mr-1 h-3 w-3" />
          {id}
        </Badge>
      ))}
      {removed.map((id) => (
        <Badge
          key={`rm-${id}`}
          variant="outline"
          className={cn(
            "border-rose-500/40 bg-rose-500/10 text-rose-700 dark:text-rose-300",
          )}
          data-testid={`${testId}-remove-${id}`}
        >
          <Minus className="mr-1 h-3 w-3" />
          {id}
        </Badge>
      ))}
    </div>
  );
}
