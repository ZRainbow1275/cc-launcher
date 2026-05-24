import { useTranslation } from "react-i18next";
import { ChevronDown, Loader2, ShieldCheck } from "lucide-react";
import type { TargetCli } from "@/lib/api/contracts";
import type { SafetySummary as SafetySummaryData } from "@/lib/api/mock/launcher";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { Badge } from "@/components/ui/badge";

interface SafetySummaryProps {
  targetCli: TargetCli;
  data: SafetySummaryData | undefined;
  isLoading: boolean;
  hasActiveProfile: boolean;
}

export function SafetySummary({
  targetCli,
  data,
  isLoading,
  hasActiveProfile,
}: SafetySummaryProps) {
  const { t } = useTranslation();

  return (
    <Collapsible
      data-testid="launcher-safety-summary"
      className="rounded-xl border border-border bg-card/30"
    >
      <CollapsibleTrigger
        data-testid="launcher-safety-toggle"
        className="group flex w-full items-center justify-between gap-2 rounded-xl px-4 py-3 text-left hover:bg-muted/40"
      >
        <div className="flex items-center gap-2">
          <ShieldCheck className="h-4 w-4 text-emerald-500" />
          <div>
            <div className="text-sm font-semibold">
              {t("launcher.safety.title")}
            </div>
            <div className="text-xs text-muted-foreground">
              {t("launcher.safety.subtitle")}
            </div>
          </div>
        </div>
        <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform group-data-[state=open]:rotate-180" />
      </CollapsibleTrigger>

      <CollapsibleContent
        data-testid="launcher-safety-content"
        className="space-y-4 px-4 pb-4"
      >
        {!hasActiveProfile ? (
          <div
            data-testid="launcher-safety-no-profile"
            className="p-4 text-sm text-muted-foreground"
          >
            {t("launcher.safety.noActiveProfile")}
          </div>
        ) : isLoading || !data ? (
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
            {t("launcher.safety.loading")}
          </div>
        ) : (
          <>
            <section
              data-testid="launcher-safety-flags-section"
              className="space-y-2"
            >
              <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {t("launcher.safety.flags")}
              </div>
              <pre
                data-testid="launcher-safety-flags"
                className="overflow-x-auto whitespace-pre-wrap break-all rounded-md bg-muted p-3 font-mono text-xs"
              >
                {data.flags.join(" \\\n  ")}
              </pre>
              <p className="text-[11px] text-muted-foreground">
                {targetCli === "claude"
                  ? t("launcher.safety.flagsHintClaude")
                  : t("launcher.safety.flagsHintCodex")}
              </p>
            </section>

            <section
              data-testid="launcher-safety-cwd-section"
              className="space-y-1"
            >
              <div className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                {t("launcher.safety.cwd")}
              </div>
              <code
                data-testid="launcher-safety-cwd"
                className="block w-full truncate rounded-md bg-muted px-3 py-2 font-mono text-xs"
              >
                {data.cwd}
              </code>
            </section>

            <section
              data-testid="launcher-safety-counts-section"
              className="flex flex-wrap items-center gap-2"
            >
              <Badge
                variant="secondary"
                data-testid="launcher-safety-l1-count"
                className="text-xs"
              >
                {t("launcher.safety.l1Count", { count: data.l1ActiveCount })}
              </Badge>
              <Badge
                variant="destructive"
                data-testid="launcher-safety-l2-count"
                className="text-xs"
              >
                {t("launcher.safety.l2Count", { count: data.l2RedlineCount })}
              </Badge>
            </section>
          </>
        )}
      </CollapsibleContent>
    </Collapsible>
  );
}
