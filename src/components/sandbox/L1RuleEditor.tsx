import { useEffect, useState } from "react";
import { Lock, ShieldAlert, ShieldCheck, Unlock } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { L1Rule } from "@/lib/api/contracts";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";

export interface L1RuleEditorProps {
  rule: L1Rule;
  busy?: boolean;
  onToggle: (rule: L1Rule, nextEnabled: boolean) => void;
  onRequestUnlock: (rule: L1Rule) => void;
  onRelock: (rule: L1Rule) => void;
}

function ruleNamespace(ruleId: string): string {
  return ruleId.startsWith("L1.") ? ruleId.slice(3) : ruleId;
}

function ruleTitleKey(rule: L1Rule): string {
  return rule.titleKey ?? `sandbox.l1.${ruleNamespace(rule.id)}.title`;
}

function ruleDescriptionKey(rule: L1Rule): string {
  return rule.descriptionKey ?? `sandbox.l1.${ruleNamespace(rule.id)}.desc`;
}

function computeRemainingHours(unlockedUntil: string | null): number | null {
  if (!unlockedUntil) return null;
  const until = Date.parse(unlockedUntil);
  if (Number.isNaN(until)) return null;
  const diff = until - Date.now();
  if (diff <= 0) return null;
  return Math.ceil(diff / (60 * 60 * 1000));
}

export function L1RuleEditor({
  rule,
  busy = false,
  onToggle,
  onRequestUnlock,
  onRelock,
}: L1RuleEditorProps) {
  const { t } = useTranslation();
  const [remainingHours, setRemainingHours] = useState<number | null>(() =>
    computeRemainingHours(rule.unlockedUntil),
  );

  useEffect(() => {
    setRemainingHours(computeRemainingHours(rule.unlockedUntil));
    if (!rule.unlockedUntil) return;
    const interval = setInterval(() => {
      setRemainingHours(computeRemainingHours(rule.unlockedUntil));
    }, 60 * 1000);
    return () => clearInterval(interval);
  }, [rule.unlockedUntil]);

  const isUnlocked = remainingHours !== null;
  const isPermanent = !rule.unlockable;

  let statusBadge: React.ReactNode;
  if (isPermanent) {
    statusBadge = (
      <Badge
        variant="destructive"
        className="gap-1 border-transparent bg-red-700 text-[10px] text-white hover:bg-red-700/90"
        data-testid={`l1-rule-${rule.id}-status-permanent`}
      >
        <Lock className="h-3 w-3" aria-hidden="true" />
        {t("sandbox.l1.permanentLock")}
      </Badge>
    );
  } else if (isUnlocked) {
    statusBadge = (
      <Badge
        variant="outline"
        className="gap-1 border-amber-500/60 text-[10px] text-amber-700 dark:text-amber-400"
        data-testid={`l1-rule-${rule.id}-status-unlocked`}
      >
        {remainingHours && remainingHours > 0
          ? t("sandbox.l1.statusUnlocked", { hours: remainingHours })
          : t("sandbox.l1.statusUnlockedLessThanHour")}
      </Badge>
    );
  } else if (rule.enabled) {
    statusBadge = (
      <Badge
        variant="secondary"
        className="gap-1 text-[10px]"
        data-testid={`l1-rule-${rule.id}-status-enabled`}
      >
        <ShieldCheck className="h-3 w-3" aria-hidden="true" />
        {t("sandbox.l1.statusEnabled")}
      </Badge>
    );
  } else {
    statusBadge = (
      <Badge
        variant="outline"
        className="gap-1 text-[10px]"
        data-testid={`l1-rule-${rule.id}-status-disabled`}
      >
        <ShieldAlert
          className="h-3 w-3 text-muted-foreground"
          aria-hidden="true"
        />
        {t("sandbox.l1.statusDisabled")}
      </Badge>
    );
  }

  return (
    <div
      data-testid={`l1-rule-${rule.id}`}
      className={cn(
        "flex flex-col gap-3 rounded-xl border border-border bg-card/50 p-4 transition-colors sm:flex-row sm:items-start sm:justify-between",
        isUnlocked && "border-amber-500/50 bg-amber-500/5",
        isPermanent && "border-destructive/30 bg-destructive/5",
      )}
    >
      <div className="flex-1 space-y-2">
        <div className="flex flex-wrap items-center gap-2">
          <p className="text-sm font-medium leading-none">
            {t(ruleTitleKey(rule))}
          </p>
          {statusBadge}
        </div>
        <p className="text-xs leading-relaxed text-muted-foreground">
          {t(ruleDescriptionKey(rule))}
        </p>
      </div>

      <div className="flex items-center gap-3 sm:flex-col sm:items-end">
        <Switch
          checked={rule.enabled}
          onCheckedChange={(value) => {
            if (!value && rule.enabled && !isUnlocked && !isPermanent) {
              onRequestUnlock(rule);
              return;
            }
            onToggle(rule, value);
          }}
          disabled={busy || isPermanent || isUnlocked}
          aria-label={t(ruleTitleKey(rule))}
          data-testid={`l1-rule-${rule.id}-switch`}
        />
        {!isPermanent && !isUnlocked && rule.enabled && (
          <Button
            variant="outline"
            size="sm"
            onClick={() => onRequestUnlock(rule)}
            disabled={busy}
            className="gap-1"
            data-testid={`l1-rule-${rule.id}-unlock`}
          >
            <Unlock className="h-3.5 w-3.5" aria-hidden="true" />
            {t("sandbox.l1.unlockButton")}
          </Button>
        )}
        {!isPermanent && isUnlocked && (
          <Button
            variant="default"
            size="sm"
            onClick={() => onRelock(rule)}
            disabled={busy}
            className="gap-1"
            data-testid={`l1-rule-${rule.id}-relock`}
          >
            <Lock className="h-3.5 w-3.5" aria-hidden="true" />
            {t("sandbox.l1.relockButton")}
          </Button>
        )}
      </div>
    </div>
  );
}
