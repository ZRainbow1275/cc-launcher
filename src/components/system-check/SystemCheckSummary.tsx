import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import {
  AlertTriangle,
  CircleCheck,
  CircleDashed,
  XCircle,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { systemProbe } from "@/lib/api/mock";
import type {
  ProbeItem,
  ProbeStatus,
  SystemProbeReport,
} from "@/lib/api/contracts";

import {
  SystemCheckDashboard,
  SYSTEM_PROBE_QUERY_KEY,
} from "./SystemCheckDashboard";

interface Counts {
  green: number;
  yellow: number;
  red: number;
}

function tally(items: ProbeItem[]): Counts {
  return items.reduce<Counts>(
    (acc, it) => {
      if (it.status === "green") acc.green += 1;
      else if (it.status === "yellow") acc.yellow += 1;
      else if (it.status === "red" || it.status === "missing") acc.red += 1;
      return acc;
    },
    { green: 0, yellow: 0, red: 0 },
  );
}

function overallIcon(status: ProbeStatus): React.ReactNode {
  switch (status) {
    case "green":
      return <CircleCheck className="h-4 w-4 text-emerald-600" />;
    case "yellow":
      return <AlertTriangle className="h-4 w-4 text-yellow-600" />;
    case "red":
    case "missing":
      return <XCircle className="h-4 w-4 text-red-600" />;
    case "unknown":
      return <CircleDashed className="h-4 w-4 text-muted-foreground" />;
  }
}

interface SystemCheckSummaryProps {
  className?: string;
  variant?: "inline" | "row";
  onOpenDashboard?: () => void;
}

export function SystemCheckSummary({
  className,
  variant = "inline",
  onOpenDashboard,
}: SystemCheckSummaryProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  const query = useQuery<SystemProbeReport>({
    queryKey: SYSTEM_PROBE_QUERY_KEY,
    queryFn: () => systemProbe.run(),
  });

  const counts = useMemo(() => tally(query.data?.items ?? []), [query.data]);
  const status: ProbeStatus = query.data?.overallStatus ?? "unknown";

  const handleOpen = (): void => {
    if (onOpenDashboard) {
      onOpenDashboard();
    } else {
      setOpen(true);
    }
  };

  const label = t("systemCheck.summary.label", {
    green: counts.green,
    yellow: counts.yellow,
    red: counts.red,
  });

  const baseClass =
    variant === "row"
      ? "inline-flex items-center justify-between gap-3 w-full px-3 py-2 rounded-md border bg-card hover:bg-muted/50 transition-colors"
      : "inline-flex items-center gap-2 px-2.5 py-1.5 rounded-md hover:bg-muted/50 transition-colors";

  return (
    <>
      <button
        type="button"
        data-testid="system-check-summary"
        data-status={status}
        onClick={handleOpen}
        className={`${baseClass} text-left ${className ?? ""}`}
      >
        <span className="flex items-center gap-2">
          {overallIcon(status)}
          <span className="text-sm font-medium">
            {t("systemCheck.summary.title")}
          </span>
        </span>
        <span className="flex items-center gap-1.5 text-xs">
          <span className="rounded-full bg-emerald-100 dark:bg-emerald-900/40 text-emerald-900 dark:text-emerald-100 px-2 py-0.5 font-mono">
            {counts.green}
          </span>
          <span className="rounded-full bg-yellow-100 dark:bg-yellow-900/40 text-yellow-900 dark:text-yellow-100 px-2 py-0.5 font-mono">
            {counts.yellow}
          </span>
          <span className="rounded-full bg-red-100 dark:bg-red-900/40 text-red-900 dark:text-red-100 px-2 py-0.5 font-mono">
            {counts.red}
          </span>
          <span className="sr-only">{label}</span>
        </span>
      </button>

      {onOpenDashboard ? null : (
        <Dialog open={open} onOpenChange={setOpen}>
          <DialogContent
            variant="fullscreen"
            zIndex="base"
            data-testid="system-check-summary-dialog"
            className="overflow-hidden"
          >
            <DialogHeader className="px-6 py-4">
              <DialogTitle>{t("systemCheck.title")}</DialogTitle>
              <DialogDescription>{t("systemCheck.subtitle")}</DialogDescription>
            </DialogHeader>
            <div className="flex-1 overflow-y-auto">
              <SystemCheckDashboard embedded />
            </div>
            <div className="px-6 py-3 border-t border-border-default flex justify-end bg-muted/20">
              <Button variant="outline" onClick={() => setOpen(false)}>
                {t("common.close")}
              </Button>
            </div>
          </DialogContent>
        </Dialog>
      )}
    </>
  );
}
