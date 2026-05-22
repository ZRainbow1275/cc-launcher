import { useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  CheckCircle2,
  Loader2,
  MonitorSmartphone,
  XCircle,
} from "lucide-react";
import type { TerminalCandidate } from "@/lib/api/contracts";
import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";

interface TerminalPickerProps {
  terminals: TerminalCandidate[];
  selectedId: string | null;
  isLoading: boolean;
  onSelect: (id: string) => void;
}

function isRecommended(t: TerminalCandidate): boolean {
  return t.installed && t.isDefault;
}

export function TerminalPicker({
  terminals,
  selectedId,
  isLoading,
  onSelect,
}: TerminalPickerProps) {
  const { t } = useTranslation();

  const recommended = useMemo(
    () => terminals.find(isRecommended) ?? null,
    [terminals],
  );

  useEffect(() => {
    if (selectedId) return;
    const fallback = recommended ?? terminals.find((x) => x.installed) ?? null;
    if (fallback) onSelect(fallback.id);
  }, [recommended, terminals, selectedId, onSelect]);

  return (
    <div
      data-testid="launcher-terminal-picker"
      className="space-y-3 rounded-xl border border-border bg-card/30 p-4"
    >
      <div className="space-y-1">
        <h3 className="text-sm font-semibold">
          {t("launcher.terminal.title")}
        </h3>
        <p className="text-xs text-muted-foreground">
          {t("launcher.terminal.subtitle")}
        </p>
      </div>

      {isLoading ? (
        <div
          className="flex items-center gap-2 text-xs text-muted-foreground"
          data-testid="launcher-terminal-loading"
        >
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
          {t("launcher.safety.loading")}
        </div>
      ) : terminals.length === 0 ? (
        <p
          className="text-xs text-muted-foreground"
          data-testid="launcher-terminal-empty"
        >
          {t("launcher.terminal.empty")}
        </p>
      ) : (
        <ul className="space-y-2" data-testid="launcher-terminal-list">
          {terminals.map((term) => {
            const isSelected = selectedId === term.id;
            const isAvailable = term.installed;
            return (
              <li key={term.id}>
                <button
                  type="button"
                  data-testid={`launcher-terminal-option-${term.id}`}
                  disabled={!isAvailable}
                  onClick={() => isAvailable && onSelect(term.id)}
                  className={cn(
                    "flex w-full items-center justify-between gap-3 rounded-lg border px-3 py-2 text-left transition-colors",
                    isAvailable
                      ? "border-border bg-background hover:border-blue-400"
                      : "border-dashed border-border bg-muted/40 cursor-not-allowed opacity-60",
                    isSelected &&
                      isAvailable &&
                      "border-blue-500 bg-blue-500/5 ring-1 ring-blue-500",
                  )}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <MonitorSmartphone className="h-4 w-4 shrink-0 text-muted-foreground" />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium truncate">
                          {term.displayName}
                        </span>
                        {isRecommended(term) ? (
                          <Badge
                            variant="default"
                            data-testid={`launcher-terminal-recommended-${term.id}`}
                            className="text-[10px] uppercase"
                          >
                            {t("launcher.terminal.recommended")}
                          </Badge>
                        ) : null}
                      </div>
                      {term.path ? (
                        <div className="mt-0.5 truncate text-[11px] text-muted-foreground">
                          <span className="opacity-70">
                            {t("launcher.terminal.pathLabel")}:
                          </span>{" "}
                          <code className="font-mono">{term.path}</code>
                        </div>
                      ) : null}
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    {isAvailable ? (
                      <Badge
                        variant="secondary"
                        data-testid={`launcher-terminal-available-${term.id}`}
                        className="text-[10px]"
                      >
                        <CheckCircle2 className="mr-1 h-3 w-3" />
                        {t("launcher.terminal.available")}
                      </Badge>
                    ) : (
                      <Badge
                        variant="outline"
                        data-testid={`launcher-terminal-unavailable-${term.id}`}
                        className="text-[10px] text-muted-foreground"
                      >
                        <XCircle className="mr-1 h-3 w-3" />
                        {t("launcher.terminal.unavailable")}
                      </Badge>
                    )}
                  </div>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
