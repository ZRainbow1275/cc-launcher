import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  CircleCheck,
  CircleDashed,
  CircleHelp,
  Loader2,
  RefreshCw,
  Wrench,
  XCircle,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { systemProbe } from "@/lib/api/mock";
import type {
  FixAction,
  ProbeItem,
  ProbeStatus,
  SystemProbeReport,
} from "@/lib/api/contracts";

import { FixActionDialog } from "./FixActionDialog";
import { ProbeCardGroup } from "./ProbeCardGroup";

export const SYSTEM_PROBE_QUERY_KEY = ["system_probe", "run"] as const;

type ProbeGroup = ProbeItem["group"];

const GROUP_ORDER: ProbeGroup[] = [
  "system",
  "runtime",
  "env",
  "network",
  "workdir",
];

const GROUP_TITLE_KEY: Record<ProbeGroup, string> = {
  system: "systemCheck.groups.system",
  runtime: "systemCheck.groups.runtime",
  env: "systemCheck.groups.env",
  network: "systemCheck.groups.network",
  workdir: "systemCheck.groups.workdir",
};

function fixActionKey(action: FixAction): string {
  switch (action.kind) {
    case "installNode":
      return `installNode:${action.targetLtsMajor}`;
    case "installGit":
      return "installGit";
    case "cleanEnvVar":
      return `cleanEnvVar:${action.varName}`;
    case "createWorkdir":
      return `createWorkdir:${action.path}`;
    case "openHomeDir":
      return "openHomeDir";
    case "injectPathEntries":
      return "injectPathEntries";
    case "externalLink":
      return `externalLink:${action.labelKey}`;
  }
}

function isAutoFixable(action: FixAction): boolean {
  return !["externalLink", "openHomeDir", "injectPathEntries"].includes(
    action.kind,
  );
}

function statusIcon(status: ProbeStatus): React.ReactNode {
  const common = "h-5 w-5";
  switch (status) {
    case "green":
      return <CircleCheck className={`${common} text-emerald-600`} />;
    case "yellow":
      return <AlertTriangle className={`${common} text-yellow-600`} />;
    case "red":
      return <XCircle className={`${common} text-red-600`} />;
    case "missing":
      return <CircleHelp className={`${common} text-red-600`} />;
    case "unknown":
      return <CircleDashed className={`${common} text-muted-foreground`} />;
  }
}

function statusToneClasses(status: ProbeStatus): string {
  switch (status) {
    case "green":
      return "bg-emerald-50 dark:bg-emerald-950 border-emerald-200 dark:border-emerald-900";
    case "yellow":
      return "bg-yellow-50 dark:bg-yellow-950 border-yellow-200 dark:border-yellow-900";
    case "red":
    case "missing":
      return "bg-red-50 dark:bg-red-950 border-red-200 dark:border-red-900";
    case "unknown":
      return "bg-muted border-border-default";
  }
}

interface CountSummary {
  green: number;
  yellow: number;
  red: number;
  unknown: number;
}

function countByStatus(items: ProbeItem[]): CountSummary {
  return items.reduce<CountSummary>(
    (acc, item) => {
      if (item.status === "green") acc.green += 1;
      else if (item.status === "yellow") acc.yellow += 1;
      else if (item.status === "red" || item.status === "missing") acc.red += 1;
      else acc.unknown += 1;
      return acc;
    },
    { green: 0, yellow: 0, red: 0, unknown: 0 },
  );
}

interface SystemCheckDashboardProps {
  className?: string;
  embedded?: boolean;
}

export function SystemCheckDashboard({
  className,
  embedded = false,
}: SystemCheckDashboardProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [activeFix, setActiveFix] = useState<{
    item: ProbeItem;
    action: FixAction;
  } | null>(null);
  const [batchQueue, setBatchQueue] = useState<
    { item: ProbeItem; action: FixAction }[]
  >([]);

  const query = useQuery<SystemProbeReport>({
    queryKey: SYSTEM_PROBE_QUERY_KEY,
    queryFn: () => systemProbe.run(),
  });

  const groupedItems = useMemo(() => {
    const items = query.data?.items ?? [];
    const map = new Map<ProbeGroup, ProbeItem[]>();
    for (const group of GROUP_ORDER) {
      map.set(group, []);
    }
    for (const item of items) {
      const bucket = map.get(item.group);
      if (bucket) bucket.push(item);
    }
    return map;
  }, [query.data]);

  const counts = useMemo(
    () => countByStatus(query.data?.items ?? []),
    [query.data],
  );

  const fixableItems = useMemo<{ item: ProbeItem; action: FixAction }[]>(() => {
    const items = query.data?.items ?? [];
    const seen = new Set<string>();
    return items.reduce<{ item: ProbeItem; action: FixAction }[]>((acc, it) => {
      if (!it.fixAction || !isAutoFixable(it.fixAction)) return acc;
      const key = fixActionKey(it.fixAction);
      if (seen.has(key)) return acc;
      seen.add(key);
      acc.push({ item: it, action: it.fixAction });
      return acc;
    }, []);
  }, [query.data]);

  const handleReprobe = (): void => {
    void query.refetch();
  };

  const handleFixOne = (item: ProbeItem): void => {
    if (!item.fixAction || !isAutoFixable(item.fixAction)) return;
    setActiveFix({ item, action: item.fixAction });
  };

  const handleFixAll = (): void => {
    if (fixableItems.length === 0) return;
    const [head, ...rest] = fixableItems;
    if (!head) return;
    setBatchQueue(rest);
    setActiveFix(head);
  };

  const handleFixDialogClose = (): void => {
    const next = batchQueue[0];
    if (next) {
      setBatchQueue((q) => q.slice(1));
      setActiveFix(next);
      return;
    }
    setActiveFix(null);
    void queryClient.invalidateQueries({ queryKey: SYSTEM_PROBE_QUERY_KEY });
  };

  const handleFixBatchAbort = (): void => {
    setBatchQueue([]);
    setActiveFix(null);
    void queryClient.invalidateQueries({ queryKey: SYSTEM_PROBE_QUERY_KEY });
  };

  const overall = query.data?.overallStatus ?? "unknown";

  return (
    <div
      data-testid="system-check-dashboard"
      className={`${embedded ? "" : "p-6"} space-y-4 ${className ?? ""}`}
    >
      <Card className={statusToneClasses(overall)}>
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-start justify-between gap-3">
            <div className="flex items-center gap-3">
              <span aria-hidden>{statusIcon(overall)}</span>
              <div>
                <CardTitle className="text-lg">
                  {t("systemCheck.title")}
                </CardTitle>
                <CardDescription>
                  {t(`systemCheck.overall.${overall}`)}
                </CardDescription>
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Button
                size="sm"
                variant="outline"
                onClick={handleReprobe}
                disabled={query.isFetching}
                data-testid="system-check-reprobe"
              >
                {query.isFetching ? (
                  <Loader2 className="h-4 w-4 mr-1 animate-spin" />
                ) : (
                  <RefreshCw className="h-4 w-4 mr-1" />
                )}
                {t("systemCheck.actions.reprobe")}
              </Button>
              <Button
                size="sm"
                variant="default"
                onClick={handleFixAll}
                disabled={fixableItems.length === 0 || activeFix !== null}
                data-testid="system-check-fix-all"
              >
                <Wrench className="h-4 w-4 mr-1" />
                {t("systemCheck.actions.fixAll", {
                  count: fixableItems.length,
                })}
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="flex flex-wrap items-center gap-4 text-sm">
            <CountChip
              tone="green"
              label={t("systemCheck.counts.green")}
              value={counts.green}
            />
            <CountChip
              tone="yellow"
              label={t("systemCheck.counts.yellow")}
              value={counts.yellow}
            />
            <CountChip
              tone="red"
              label={t("systemCheck.counts.red")}
              value={counts.red}
            />
            {counts.unknown > 0 ? (
              <CountChip
                tone="unknown"
                label={t("systemCheck.counts.unknown")}
                value={counts.unknown}
              />
            ) : null}
            {query.data ? (
              <span className="ml-auto text-xs text-muted-foreground">
                {t("systemCheck.lastUpdated", {
                  time: new Date(query.data.generatedAt).toLocaleTimeString(),
                })}
              </span>
            ) : null}
          </div>
        </CardContent>
      </Card>

      {query.isError ? (
        <Card className="border-red-200 dark:border-red-900 bg-red-50 dark:bg-red-950">
          <CardContent className="pt-6">
            <p className="text-sm text-red-900 dark:text-red-100">
              {t("systemCheck.error.fetch")}
            </p>
          </CardContent>
        </Card>
      ) : null}

      <div className="space-y-3" data-testid="system-check-groups">
        {GROUP_ORDER.map((group) => {
          const items = groupedItems.get(group) ?? [];
          if (items.length === 0) return null;
          return (
            <ProbeCardGroup
              key={group}
              title={t(GROUP_TITLE_KEY[group])}
              items={items}
              onRequestFix={handleFixOne}
            />
          );
        })}
      </div>

      {activeFix ? (
        <FixActionDialog
          key={fixActionKey(activeFix.action)}
          item={activeFix.item}
          action={activeFix.action}
          batchRemaining={batchQueue.length}
          onClose={handleFixDialogClose}
          onAbortBatch={handleFixBatchAbort}
        />
      ) : null}
    </div>
  );
}

interface CountChipProps {
  tone: "green" | "yellow" | "red" | "unknown";
  label: string;
  value: number;
}

function CountChip({ tone, label, value }: CountChipProps) {
  const toneClass =
    tone === "green"
      ? "bg-emerald-100 text-emerald-900 dark:bg-emerald-900/40 dark:text-emerald-100"
      : tone === "yellow"
        ? "bg-yellow-100 text-yellow-900 dark:bg-yellow-900/40 dark:text-yellow-100"
        : tone === "red"
          ? "bg-red-100 text-red-900 dark:bg-red-900/40 dark:text-red-100"
          : "bg-muted text-muted-foreground";
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-semibold ${toneClass}`}
      data-testid={`system-check-count-${tone}`}
    >
      <span className="font-mono">{value}</span>
      <span>{label}</span>
    </span>
  );
}
