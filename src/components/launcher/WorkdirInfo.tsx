import { useTranslation } from "react-i18next";
import { FolderOpen, Loader2, Lock } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface WorkdirInfoProps {
  cwdDisplay: string;
  cwdAbsolute: string;
  isOpening: boolean;
  onOpen: () => void;
}

export function WorkdirInfo({
  cwdDisplay,
  cwdAbsolute,
  isOpening,
  onOpen,
}: WorkdirInfoProps) {
  const { t } = useTranslation();

  return (
    <TooltipProvider>
      <div
        data-testid="launcher-workdir-info"
        className="space-y-3 rounded-xl border border-border bg-card/30 p-4"
      >
        <div className="space-y-1">
          <h3 className="text-sm font-semibold">
            {t("launcher.workdir.title")}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("launcher.workdir.subtitle")}
          </p>
        </div>

        <div className="space-y-1">
          <Tooltip>
            <TooltipTrigger asChild>
              <code
                data-testid="launcher-workdir-path"
                className="block w-full truncate rounded-md bg-muted px-3 py-2 font-mono text-xs"
              >
                {cwdDisplay}
              </code>
            </TooltipTrigger>
            <TooltipContent side="bottom">
              <div className="space-y-0.5">
                <div className="text-[10px] opacity-70">
                  {t("launcher.workdir.absoluteLabel")}
                </div>
                <code
                  data-testid="launcher-workdir-absolute"
                  className="font-mono text-xs"
                >
                  {cwdAbsolute}
                </code>
              </div>
            </TooltipContent>
          </Tooltip>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={onOpen}
            disabled={isOpening}
            data-testid="launcher-workdir-open"
          >
            {isOpening ? (
              <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
            ) : (
              <FolderOpen className="mr-1.5 h-3.5 w-3.5" />
            )}
            {isOpening
              ? t("launcher.workdir.opening")
              : t("launcher.workdir.open")}
          </Button>

          <Tooltip>
            <TooltipTrigger asChild>
              <span className="inline-flex">
                <Button
                  size="sm"
                  variant="outline"
                  disabled
                  data-testid="launcher-workdir-reset-disabled"
                  aria-disabled="true"
                  className="cursor-not-allowed"
                >
                  <Lock className="mr-1.5 h-3.5 w-3.5" />
                  {t("launcher.workdir.resetDisabled")}
                </Button>
              </span>
            </TooltipTrigger>
            <TooltipContent
              side="bottom"
              data-testid="launcher-workdir-reset-tooltip"
            >
              {t("launcher.workdir.resetTooltip")}
            </TooltipContent>
          </Tooltip>
        </div>
      </div>
    </TooltipProvider>
  );
}
