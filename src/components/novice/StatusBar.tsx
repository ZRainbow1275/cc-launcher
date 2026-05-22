import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { SystemCheckSummary } from "@/components/system-check";
import {
  cliState,
  profile as profileApi,
  sandbox as sandboxApi,
} from "@/lib/api/mock";
import type { Profile, SandboxLevel, TargetCli } from "@/lib/api/contracts";

const CLI_LIST: TargetCli[] = ["claude", "codex"];

export interface NoviceActiveProfile {
  cli: TargetCli;
  profile: Profile | null;
}

async function loadActiveProfile(): Promise<NoviceActiveProfile> {
  for (const cli of CLI_LIST) {
    const activeId = await cliState.get_active(cli);
    if (activeId) {
      const list = await profileApi.list(cli);
      const found = list.find((p) => p.id === activeId);
      if (found) {
        return { cli, profile: found };
      }
    }
  }
  return { cli: "claude", profile: null };
}

export function StatusBar() {
  const { t } = useTranslation();

  const profileQuery = useQuery<NoviceActiveProfile>({
    queryKey: ["novice", "active_profile"],
    queryFn: loadActiveProfile,
  });

  const sandboxQuery = useQuery<SandboxLevel>({
    queryKey: ["novice", "sandbox_level"],
    queryFn: () => sandboxApi.get_sandbox_level(),
  });

  const activeName =
    profileQuery.data?.profile?.name ?? t("novice.statusBar.noProfile");

  const sandboxLabel = sandboxQuery.data
    ? t(`novice.statusBar.sandboxLevel.${sandboxQuery.data}`)
    : "—";

  return (
    <div
      data-testid="novice-status-bar"
      className="flex items-center gap-4 border-t border-border-default bg-muted/30 px-4 py-2"
    >
      <div
        data-testid="novice-status-profile"
        className="flex items-center gap-1.5 text-xs"
      >
        <span className="text-muted-foreground">
          {t("novice.statusBar.profile")}:
        </span>
        <span className="font-medium">{activeName}</span>
      </div>
      <div
        data-testid="novice-status-sandbox"
        className="flex items-center gap-1.5 text-xs"
      >
        <span className="text-muted-foreground">
          {t("novice.statusBar.sandbox")}:
        </span>
        <span className="font-medium">{sandboxLabel}</span>
      </div>
      <div className="ml-auto flex items-center gap-1.5 text-xs">
        <span className="text-muted-foreground">
          {t("novice.statusBar.system")}:
        </span>
        <SystemCheckSummary variant="inline" />
      </div>
    </div>
  );
}
