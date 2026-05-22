import { Lock, ExternalLink } from "lucide-react";
import { useTranslation } from "react-i18next";

import type { L2Redline } from "@/lib/api/contracts";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

export type DetectedOs = "windows" | "macos" | "linux";

export interface L2RedlineCardProps {
  redline: L2Redline;
  os: DetectedOs;
  className?: string;
}

function l2DescriptionKey(id: string): string {
  return `sandbox.l2.${id.replace(/\./g, ".")}`;
}

export function L2RedlineCard({ redline, os, className }: L2RedlineCardProps) {
  const { t } = useTranslation();
  const docKey =
    os === "macos" ? "sandbox.docLink.macos" : "sandbox.docLink.windows";
  const href = `/${t(docKey)}`;

  return (
    <div
      data-testid={`l2-redline-${redline.id}`}
      className={cn(
        "flex items-start justify-between gap-4 rounded-lg border border-destructive/30 bg-destructive/5 p-3",
        className,
      )}
    >
      <div className="flex flex-1 items-start gap-3">
        <div className="mt-0.5 flex h-7 w-7 shrink-0 items-center justify-center rounded-md bg-destructive/15">
          <Lock className="h-4 w-4 text-destructive" aria-hidden="true" />
        </div>
        <div className="space-y-1.5">
          <div className="flex flex-wrap items-center gap-2">
            <Badge
              variant="destructive"
              className="text-[10px] uppercase tracking-wide"
            >
              {t("sandbox.l2.permanentLockBadge")}
            </Badge>
            <Badge variant="outline" className="text-[10px] uppercase">
              {t(`sandbox.l2.matchType.${redline.matchType}`)}
            </Badge>
            <code className="rounded bg-background px-1.5 py-0.5 font-mono text-[11px] text-foreground/80 break-all">
              {redline.pattern}
            </code>
          </div>
          <p className="text-sm font-medium leading-snug">
            {t(l2DescriptionKey(redline.id))}
          </p>
        </div>
      </div>
      <a
        href={href}
        target="_blank"
        rel="noreferrer noopener"
        className="inline-flex shrink-0 items-center gap-1 self-start whitespace-nowrap text-xs font-medium text-muted-foreground hover:text-foreground hover:underline"
      >
        {t("sandbox.l2.whyLink")}
        <ExternalLink className="h-3 w-3" aria-hidden="true" />
      </a>
    </div>
  );
}
