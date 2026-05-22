import { useTranslation } from "react-i18next";
import {
  AlertTriangle,
  CircleCheck,
  CircleDashed,
  CircleHelp,
  ExternalLink,
  Wrench,
  XCircle,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type { FixAction, ProbeItem, ProbeStatus } from "@/lib/api/contracts";

const INFORMATIONAL_IDS = new Set([
  "admin",
  "psPolicy",
  "systemProxy",
  "rosetta",
  "defender",
]);

function statusDot(status: ProbeStatus): React.ReactNode {
  const common = "h-4 w-4 flex-shrink-0";
  switch (status) {
    case "green":
      return (
        <CircleCheck
          aria-label="green"
          className={`${common} text-emerald-600`}
        />
      );
    case "yellow":
      return (
        <AlertTriangle
          aria-label="yellow"
          className={`${common} text-yellow-600`}
        />
      );
    case "red":
      return <XCircle aria-label="red" className={`${common} text-red-600`} />;
    case "missing":
      return (
        <CircleHelp aria-label="missing" className={`${common} text-red-600`} />
      );
    case "unknown":
      return (
        <CircleDashed
          aria-label="unknown"
          className={`${common} text-muted-foreground`}
        />
      );
  }
}

function toneClasses(status: ProbeStatus): string {
  switch (status) {
    case "green":
      return "border-emerald-200 dark:border-emerald-900 bg-white dark:bg-gray-900";
    case "yellow":
      return "border-yellow-200 dark:border-yellow-900 bg-yellow-50/40 dark:bg-yellow-950/30";
    case "red":
    case "missing":
      return "border-red-200 dark:border-red-900 bg-red-50/40 dark:bg-red-950/30";
    case "unknown":
      return "border-border-default bg-muted/30";
  }
}

function formatValue(item: ProbeItem): string | null {
  const v = item.value as unknown;
  if (v === null || v === undefined) return null;
  if (typeof v === "string") return v;
  if (typeof v === "number" || typeof v === "boolean") return String(v);
  if (typeof v !== "object") return null;
  const obj = v as Record<string, unknown>;
  if (typeof obj.version === "string") {
    return obj.version;
  }
  if (typeof obj.totalGb === "number") {
    return `${obj.totalGb} GiB`;
  }
  if (typeof obj.availableGb === "number" && typeof obj.mount === "string") {
    return `${obj.availableGb.toFixed(1)} GiB (${obj.mount})`;
  }
  if (typeof obj.availableGb === "number") {
    return `${obj.availableGb.toFixed(1)} GiB`;
  }
  if (typeof obj.physicalCores === "number") {
    return `${obj.physicalCores} cores${obj.brand ? ` (${String(obj.brand)})` : ""}`;
  }
  if (typeof obj.arch === "string") {
    return `${obj.arch}${typeof obj.bits === "number" ? ` / ${obj.bits}bit` : ""}`;
  }
  if (typeof obj.longVersion === "string") {
    return String(obj.longVersion);
  }
  if (typeof obj.policy === "string") {
    return String(obj.policy);
  }
  if (typeof obj.isAdmin === "boolean") {
    return obj.isAdmin ? "admin" : "user";
  }
  if (typeof obj.path === "string" && typeof obj.writable === "boolean") {
    return obj.path;
  }
  if (Array.isArray(v)) {
    const arr = v as Array<{ name?: string; ok?: boolean; latencyMs?: number }>;
    return arr
      .map((entry) =>
        entry.ok
          ? `${entry.name}:${entry.latencyMs ?? "?"}ms`
          : `${entry.name}:✗`,
      )
      .join(" · ");
  }
  return null;
}

function detectOs(): "windows" | "macos" | "linux" {
  if (typeof navigator === "undefined") return "windows";
  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("mac")) return "macos";
  if (ua.includes("linux")) return "linux";
  return "windows";
}

function docLinkForFactor(item: ProbeItem): string {
  const os = detectOs();
  const slug = os === "macos" ? "macos" : "windows";
  return `docs/env-setup-${slug}-zh.md#section-3-${item.id}`;
}

interface ProbeItemCardProps {
  item: ProbeItem;
  onRequestFix: (item: ProbeItem) => void;
}

export function ProbeItemCard({ item, onRequestFix }: ProbeItemCardProps) {
  const { t, i18n } = useTranslation();
  const name = t(item.nameKey, { defaultValue: item.id });
  const message = t(item.messageKey, { defaultValue: item.messageKey });
  const value = formatValue(item);
  const action: FixAction | null = item.fixAction;
  const informational =
    action?.kind === "externalLink" || INFORMATIONAL_IDS.has(item.id);

  return (
    <div
      data-testid={`probe-item-${item.id}`}
      data-status={item.status}
      className={`flex items-start gap-3 rounded-md border p-3 ${toneClasses(item.status)}`}
    >
      <div className="mt-0.5">{statusDot(item.status)}</div>
      <div className="flex-1 min-w-0">
        <div className="flex flex-wrap items-baseline gap-x-2">
          <span className="text-sm font-semibold">{name}</span>
          {value ? (
            <span className="text-xs text-muted-foreground font-mono truncate">
              {value}
            </span>
          ) : null}
        </div>
        <p className="text-xs text-muted-foreground mt-1">{message}</p>
      </div>
      <div className="flex-shrink-0 self-center">
        {action && action.kind === "externalLink" ? (
          <Button
            variant="outline"
            size="sm"
            asChild
            data-testid={`probe-item-${item.id}-link`}
          >
            <a
              href={action.url}
              target="_blank"
              rel="noreferrer"
              aria-label={t(action.labelKey, {
                defaultValue: t("systemCheck.actions.learnMore"),
              })}
            >
              <ExternalLink className="h-4 w-4 mr-1" />
              {t("systemCheck.actions.learnMore")}
            </a>
          </Button>
        ) : informational ? (
          <Button
            variant="outline"
            size="sm"
            asChild
            data-testid={`probe-item-${item.id}-doc`}
          >
            <a
              href={docLinkForFactor(item)}
              target="_blank"
              rel="noreferrer"
              hrefLang={i18n.language}
            >
              <ExternalLink className="h-4 w-4 mr-1" />
              {t("systemCheck.actions.learnMore")}
            </a>
          </Button>
        ) : action ? (
          <Button
            variant="default"
            size="sm"
            onClick={() => onRequestFix(item)}
            data-testid={`probe-item-${item.id}-fix`}
          >
            <Wrench className="h-4 w-4 mr-1" />
            {t("systemCheck.actions.fix")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
