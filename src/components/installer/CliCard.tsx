import { useTranslation } from "react-i18next";
import { CheckCircle2, Download, Loader2, Trash2 } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import type { CliInstallStatus, TargetCli } from "@/lib/api/contracts";

interface CliCardProps {
  cli: TargetCli;
  status: CliInstallStatus;
  profileCount: number;
  isInstalling?: boolean;
  isUninstalling?: boolean;
  onUninstall: () => void;
  onViewProfiles: () => void;
  onInstall?: () => void;
}

const CLI_DISPLAY_NAME: Record<TargetCli, string> = {
  claude: "Claude Code",
  codex: "Codex CLI",
};

export function CliCard({
  cli,
  status,
  profileCount,
  isInstalling = false,
  isUninstalling = false,
  onUninstall,
  onViewProfiles,
  onInstall,
}: CliCardProps) {
  const { t } = useTranslation();
  const installed = status.installed;

  return (
    <Card data-testid={`cli-card-${cli}`} data-installed={installed.toString()}>
      <CardContent className="pt-6 space-y-3">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-3">
            <div
              className={`h-10 w-10 rounded-md flex items-center justify-center text-white text-sm font-semibold ${
                cli === "claude" ? "bg-orange-500" : "bg-emerald-600"
              }`}
            >
              {cli === "claude" ? "CC" : "CX"}
            </div>
            <div>
              <p className="font-medium text-sm">{CLI_DISPLAY_NAME[cli]}</p>
              <p
                className="text-xs text-muted-foreground"
                data-testid={`cli-card-${cli}-status-label`}
              >
                {isInstalling
                  ? t("installer.cardView.installing")
                  : installed
                    ? t("installer.cardView.installed")
                    : t("installer.cardView.notInstalled")}
              </p>
            </div>
          </div>
          {isInstalling ? (
            <Badge variant="outline">
              <Loader2 className="h-3 w-3 animate-spin mr-1" />
              {t("installer.cardView.installing")}
            </Badge>
          ) : installed ? (
            <Badge
              variant="default"
              data-testid={`cli-card-${cli}-installed-badge`}
              className="bg-emerald-500 hover:bg-emerald-600"
            >
              <CheckCircle2 className="h-3 w-3 mr-1" />
              {t("installer.cardView.installed")}
            </Badge>
          ) : (
            <Badge variant="outline">
              {t("installer.cardView.notInstalled")}
            </Badge>
          )}
        </div>

        {installed ? (
          <div className="grid grid-cols-2 gap-2 text-xs">
            <div className="flex flex-col">
              <span className="text-muted-foreground">
                {t("installer.cardView.version")}
              </span>
              <span
                className="font-mono"
                data-testid={`cli-card-${cli}-version`}
              >
                {status.version ?? "-"}
              </span>
            </div>
            <div className="flex flex-col">
              <span className="text-muted-foreground">Profile</span>
              <span data-testid={`cli-card-${cli}-profile-count`}>
                {profileCount > 0
                  ? t("installer.cardView.profileCount", {
                      count: profileCount,
                    })
                  : t("installer.cardView.noProfile")}
              </span>
            </div>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">
            {CLI_DISPLAY_NAME[cli]} {t("installer.cardView.notInstalled")}
          </p>
        )}

        <div className="flex flex-wrap gap-2 pt-1">
          {installed ? (
            <>
              <Button
                type="button"
                size="sm"
                variant="outline"
                onClick={onViewProfiles}
                data-testid={`cli-card-${cli}-view-profiles`}
              >
                {t("installer.cardView.viewProfiles")}
              </Button>
              <Button
                type="button"
                size="sm"
                variant="destructive"
                disabled={isUninstalling}
                onClick={onUninstall}
                data-testid={`cli-card-${cli}-uninstall`}
              >
                {isUninstalling ? (
                  <Loader2 className="h-3.5 w-3.5 mr-1 animate-spin" />
                ) : (
                  <Trash2 className="h-3.5 w-3.5 mr-1" />
                )}
                {t("installer.cardView.uninstall")}
              </Button>
            </>
          ) : onInstall ? (
            <Button
              type="button"
              size="sm"
              variant="default"
              onClick={onInstall}
              data-testid={`cli-card-${cli}-install`}
            >
              <Download className="h-3.5 w-3.5 mr-1" />
              {t("installer.cardView.installNow")}
            </Button>
          ) : null}
        </div>
      </CardContent>
    </Card>
  );
}
