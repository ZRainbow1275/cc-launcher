import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ChevronDown, Info, ShieldAlert, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { sandbox, systemProbe } from "@/lib/api/mock";
import type { L1Rule, L2Redline, SandboxLevel } from "@/lib/api/contracts";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { cn } from "@/lib/utils";

import {
  DangerousConfirm,
  type DangerousConfirmLocale,
  type DangerousConfirmRuleInfo,
} from "./DangerousConfirm";
import { L1RuleEditor } from "./L1RuleEditor";
import { L2RedlineCard, type DetectedOs } from "./L2RedlineCard";

const QUERY_KEYS = {
  l1Rules: ["sandbox", "l1_rules"] as const,
  l2Redlines: ["sandbox", "l2_redlines"] as const,
  level: ["sandbox", "level"] as const,
};

function getRuleNamespace(ruleId: string): string {
  return ruleId.startsWith("L1.") ? ruleId.slice(3) : ruleId;
}

function buildRuleInfo(rule: L1Rule): DangerousConfirmRuleInfo {
  const ns = getRuleNamespace(rule.id);
  return {
    ruleId: rule.id,
    titleKey: rule.titleKey ?? `sandbox.l1.${ns}.title`,
    descriptionKey: rule.descriptionKey ?? `sandbox.l1.${ns}.desc`,
    riskKeys: [
      `sandbox.l1.${ns}.risk1`,
      `sandbox.l1.${ns}.risk2`,
      `sandbox.l1.${ns}.risk3`,
    ],
  };
}

function detectOsFromUserAgent(): DetectedOs {
  if (typeof navigator === "undefined") return "windows";
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac")) return "macos";
  if (ua.includes("linux")) return "linux";
  return "windows";
}

function normalizeOs(value: unknown): DetectedOs | null {
  if (!value || typeof value !== "object") return null;
  const name = (value as { name?: unknown }).name;
  if (typeof name !== "string") return null;
  const lower = name.toLowerCase();
  if (lower.includes("windows") || lower === "win32" || lower === "win")
    return "windows";
  if (lower.includes("mac") || lower === "darwin") return "macos";
  if (lower.includes("linux")) return "linux";
  return null;
}

function localeKeyFromI18n(language: string): DangerousConfirmLocale {
  if (language.startsWith("ja")) return "ja";
  if (language.startsWith("en")) return "en";
  return "zh";
}

function groupRedlines(
  list: L2Redline[],
): Array<{ category: L2Redline["category"]; items: L2Redline[] }> {
  const map = new Map<L2Redline["category"], L2Redline[]>();
  for (const r of list) {
    const arr = map.get(r.category) ?? [];
    arr.push(r);
    map.set(r.category, arr);
  }
  return Array.from(map.entries()).map(([category, items]) => ({
    category,
    items,
  }));
}

function errorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error) return err.message;
  if (err && typeof err === "object" && "message" in err) {
    const m = (err as { message: unknown }).message;
    if (typeof m === "string") return m;
    if (m && typeof m === "object" && "zh" in m) {
      const zh = (m as { zh?: unknown }).zh;
      if (typeof zh === "string") return zh;
    }
  }
  return fallback;
}

export interface SandboxSettingsProps {
  className?: string;
}

export function SandboxSettings({ className }: SandboxSettingsProps) {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();

  const [unlockRule, setUnlockRule] = useState<L1Rule | null>(null);
  const [pendingLevel, setPendingLevel] = useState<SandboxLevel | null>(null);
  const [detectedOs, setDetectedOs] = useState<DetectedOs>(
    detectOsFromUserAgent(),
  );

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const report = await systemProbe.run();
        const osItem = report.items.find((it) => it.id === "os");
        const os = osItem ? normalizeOs(osItem.value) : null;
        if (os && !cancelled) setDetectedOs(os);
      } catch {
        // keep userAgent fallback
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  const l1Query = useQuery({
    queryKey: QUERY_KEYS.l1Rules,
    queryFn: () => sandbox.get_l1_rules(),
    refetchInterval: 60 * 1000,
  });

  const l2Query = useQuery({
    queryKey: QUERY_KEYS.l2Redlines,
    queryFn: () => sandbox.list_l2_redlines(),
    staleTime: 5 * 60 * 1000,
  });

  const levelQuery = useQuery({
    queryKey: QUERY_KEYS.level,
    queryFn: () => sandbox.get_sandbox_level(),
  });

  const toggleMutation = useMutation({
    mutationFn: (vars: { ruleId: string; enabled: boolean }) =>
      sandbox.set_l1_rule(vars.ruleId, vars.enabled),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.l1Rules });
      toast.success(t("sandbox.l1.toggleSuccess"));
    },
    onError: (err) => {
      toast.error(
        t("sandbox.l1.toggleFailed", {
          error: errorMessage(err, t("common.error")),
        }),
      );
    },
  });

  const unlockMutation = useMutation({
    mutationFn: (vars: { ruleId: string; keyword: string }) =>
      sandbox.unlock_l1_rule(vars.ruleId, vars.keyword),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.l1Rules });
      toast.success(t("sandbox.confirm.unlocked"));
      setUnlockRule(null);
    },
    onError: (err) => {
      toast.error(
        t("sandbox.confirm.unlockFailed", {
          error: errorMessage(err, t("common.error")),
        }),
      );
    },
  });

  const relockMutation = useMutation({
    mutationFn: (vars: { ruleId: string }) =>
      sandbox.set_l1_rule(vars.ruleId, true),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.l1Rules });
      toast.success(t("sandbox.l1.relockSuccess"));
    },
    onError: (err) => {
      toast.error(
        t("sandbox.l1.toggleFailed", {
          error: errorMessage(err, t("common.error")),
        }),
      );
    },
  });

  const levelMutation = useMutation({
    mutationFn: (level: SandboxLevel) => sandbox.set_sandbox_level(level),
    onSuccess: (_data, level) => {
      void queryClient.invalidateQueries({ queryKey: QUERY_KEYS.level });
      toast.success(
        t("sandbox.level.changed", {
          level: t(`sandbox.level.${level}`),
        }),
      );
      setPendingLevel(null);
    },
    onError: (err) => {
      toast.error(
        t("sandbox.level.changeFailed", {
          error: errorMessage(err, t("common.error")),
        }),
      );
    },
  });

  const l1Rules = l1Query.data ?? [];
  const l2Grouped = useMemo(
    () => groupRedlines(l2Query.data ?? []),
    [l2Query.data],
  );

  const currentLevel: SandboxLevel = levelQuery.data ?? "strict";

  const handleLevelChange = (value: string) => {
    if (value !== "strict" && value !== "medium") return;
    if (value === currentLevel) return;
    setPendingLevel(value);
  };

  const handleLevelConfirm = () => {
    if (!pendingLevel) return;
    levelMutation.mutate(pendingLevel);
  };

  const platformAlertKey =
    detectedOs === "macos"
      ? "sandbox.platform.macos"
      : detectedOs === "linux"
        ? "sandbox.platform.linux"
        : "sandbox.platform.windows";

  const dangerousConfirmInfo: DangerousConfirmRuleInfo | null = unlockRule
    ? buildRuleInfo(unlockRule)
    : null;

  return (
    <div className={cn("space-y-6", className)} data-testid="sandbox-settings">
      <header className="space-y-1">
        <h2 className="text-xl font-semibold">{t("sandbox.title")}</h2>
        <p className="text-sm text-muted-foreground">{t("sandbox.subtitle")}</p>
      </header>

      <section
        data-testid="sandbox-level-section"
        className="space-y-3 rounded-xl border border-border bg-card/30 p-4"
      >
        <div className="space-y-1">
          <h3 className="text-sm font-semibold">{t("sandbox.level.title")}</h3>
          <p className="text-xs text-muted-foreground">
            {t("sandbox.level.description")}
          </p>
        </div>
        <Tabs
          value={currentLevel}
          onValueChange={handleLevelChange}
          activationMode="manual"
          data-testid="sandbox-level-tabs"
        >
          <TabsList>
            <TabsTrigger value="strict" data-testid="sandbox-level-strict">
              <ShieldCheck className="mr-1.5 h-3.5 w-3.5" />
              {t("sandbox.level.strict")}
            </TabsTrigger>
            <TabsTrigger value="medium" data-testid="sandbox-level-medium">
              <ShieldAlert className="mr-1.5 h-3.5 w-3.5" />
              {t("sandbox.level.medium")}
            </TabsTrigger>
          </TabsList>
        </Tabs>
        <p className="text-xs text-muted-foreground">
          {currentLevel === "strict"
            ? t("sandbox.level.strictDesc")
            : t("sandbox.level.mediumDesc")}
        </p>
      </section>

      <Alert data-testid="sandbox-platform-alert">
        <Info className="h-4 w-4" />
        <AlertDescription>{t(platformAlertKey)}</AlertDescription>
      </Alert>

      <section
        data-testid="sandbox-l1-section"
        className="space-y-3 rounded-xl border border-border bg-card/30 p-4"
      >
        <div className="space-y-1">
          <h3 className="text-sm font-semibold">{t("sandbox.l1.title")}</h3>
          <p className="text-xs text-muted-foreground">
            {t("sandbox.l1.description")}
          </p>
        </div>

        {l1Query.isLoading && (
          <p className="text-xs text-muted-foreground">
            {t("sandbox.l1.loading")}
          </p>
        )}
        {l1Query.isError && (
          <Alert variant="destructive">
            <AlertDescription>
              {t("sandbox.l1.loadError", {
                error: errorMessage(l1Query.error, t("common.error")),
              })}
            </AlertDescription>
          </Alert>
        )}
        {!l1Query.isLoading && l1Rules.length === 0 && !l1Query.isError && (
          <p className="text-xs text-muted-foreground">
            {t("sandbox.l1.empty")}
          </p>
        )}

        <div className="space-y-2">
          {l1Rules.map((rule) => (
            <L1RuleEditor
              key={rule.id}
              rule={rule}
              busy={
                toggleMutation.isPending ||
                relockMutation.isPending ||
                unlockMutation.isPending
              }
              onToggle={(r, nextEnabled) =>
                toggleMutation.mutate({ ruleId: r.id, enabled: nextEnabled })
              }
              onRequestUnlock={(r) => setUnlockRule(r)}
              onRelock={(r) => relockMutation.mutate({ ruleId: r.id })}
            />
          ))}
        </div>
      </section>

      <section
        data-testid="sandbox-l2-section"
        className="space-y-3 rounded-xl border border-border bg-card/30 p-4"
      >
        <div className="space-y-1">
          <h3 className="text-sm font-semibold">{t("sandbox.l2.title")}</h3>
          <p className="text-xs text-muted-foreground">
            {t("sandbox.l2.description")}
          </p>
        </div>

        {l2Query.isLoading && (
          <p className="text-xs text-muted-foreground">
            {t("sandbox.l2.loading")}
          </p>
        )}
        {l2Query.isError && (
          <Alert variant="destructive">
            <AlertDescription>
              {t("sandbox.l2.loadError", {
                error: errorMessage(l2Query.error, t("common.error")),
              })}
            </AlertDescription>
          </Alert>
        )}

        <div className="space-y-3">
          {l2Grouped.map((group) => (
            <Collapsible
              key={group.category}
              defaultOpen
              data-testid={`l2-group-${group.category}`}
            >
              <CollapsibleTrigger className="group flex w-full items-center justify-between rounded-md bg-muted/30 px-3 py-2 text-left text-xs font-semibold uppercase tracking-wide text-muted-foreground hover:bg-muted/60">
                <span className="flex items-center gap-2">
                  {t(`sandbox.l2.category.${group.category}`)}
                  <Badge variant="outline" className="text-[10px]">
                    {group.items.length}
                  </Badge>
                </span>
                <ChevronDown className="h-3.5 w-3.5 transition-transform group-data-[state=closed]:rotate-[-90deg]" />
              </CollapsibleTrigger>
              <CollapsibleContent className="mt-2 space-y-2">
                {group.items.map((item) => (
                  <L2RedlineCard key={item.id} redline={item} os={detectedOs} />
                ))}
              </CollapsibleContent>
            </Collapsible>
          ))}
        </div>
      </section>

      <ConfirmDialog
        isOpen={pendingLevel !== null}
        title={t("sandbox.level.confirmTitle")}
        message={t("sandbox.level.confirmMessage")}
        variant="destructive"
        zIndex="alert"
        onCancel={() => setPendingLevel(null)}
        onConfirm={handleLevelConfirm}
      />

      <DangerousConfirm
        isOpen={unlockRule !== null}
        rule={dangerousConfirmInfo}
        locale={localeKeyFromI18n(i18n.language)}
        submitting={unlockMutation.isPending}
        onClose={() => {
          if (!unlockMutation.isPending) setUnlockRule(null);
        }}
        onConfirm={(keyword) => {
          if (!unlockRule) return;
          unlockMutation.mutate({ ruleId: unlockRule.id, keyword });
        }}
      />
    </div>
  );
}
