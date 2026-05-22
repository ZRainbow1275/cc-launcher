import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { Package, RefreshCcw, Play } from "lucide-react";

import { installer, cliState, onboarding } from "@/lib/api/mock";
import type {
  ActiveProfileMap,
  CliInstallStatus,
  OnboardingState,
  TargetCli,
} from "@/lib/api/contracts";

import { BigButton } from "./BigButton";
import { StatusBar } from "./StatusBar";
import { OnboardingDialog } from "./OnboardingDialog";

const CLI_LIST: TargetCli[] = ["claude", "codex"];

export type NoviceRoute = "installer" | "profile" | "launcher";

export interface NoviceHomeProps {
  onNavigate?: (route: NoviceRoute) => void;
}

async function loadCliSummary(): Promise<{
  statuses: CliInstallStatus[];
  hasAnyCli: boolean;
}> {
  const statuses = await Promise.all(
    CLI_LIST.map((cli) => installer.detect_cli(cli)),
  );
  return { statuses, hasAnyCli: statuses.some((s) => s.installed) };
}

function navigateFallback(route: NoviceRoute): void {
  // eslint-disable-next-line no-console
  console.log(`[NoviceHome] navigate placeholder: ${route}`);
}

export function NoviceHome({ onNavigate }: NoviceHomeProps = {}) {
  const { t } = useTranslation();

  const onboardingQuery = useQuery<OnboardingState>({
    queryKey: ["onboarding", "state"],
    queryFn: () => onboarding.get_state(),
  });

  const cliQuery = useQuery({
    queryKey: ["novice", "cli_summary"],
    queryFn: loadCliSummary,
  });

  const activeQuery = useQuery<ActiveProfileMap>({
    queryKey: ["novice", "active_profile_map"],
    queryFn: () => cliState.list_all_active(),
  });

  const hasAnyCli = cliQuery.data?.hasAnyCli ?? false;
  const hasActiveProfile = useMemo(() => {
    const map = activeQuery.data;
    if (!map) return false;
    return Boolean(map.claude) || Boolean(map.codex);
  }, [activeQuery.data]);

  const handleNavigate = (route: NoviceRoute): void => {
    if (onNavigate) {
      onNavigate(route);
    } else {
      navigateFallback(route);
    }
  };

  const onboardingOpen =
    onboardingQuery.isSuccess && onboardingQuery.data?.completed === false;

  return (
    <div className="flex min-h-screen w-full flex-col bg-background">
      <header className="flex items-center gap-3 px-8 py-6 border-b border-border-default">
        <div
          className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10 text-primary text-lg font-bold"
          aria-hidden
        >
          CC
        </div>
        <div>
          <h1 className="text-xl font-semibold leading-tight">
            {t("novice.title")}
          </h1>
          <p className="text-xs text-muted-foreground">
            {t("app.description")}
          </p>
        </div>
      </header>

      <main
        data-testid="novice-home-main"
        className="flex-1 flex items-center justify-center px-8 py-10"
      >
        <div className="grid w-full max-w-4xl grid-cols-1 gap-6 sm:grid-cols-3">
          <BigButton
            testId="novice-button-install"
            icon={<Package className="h-9 w-9" />}
            title={t("novice.buttons.install")}
            subtitle={t("novice.buttons.installSubtitle")}
            onClick={() => handleNavigate("installer")}
          />
          <BigButton
            testId="novice-button-switch-profile"
            icon={<RefreshCcw className="h-9 w-9" />}
            title={t("novice.buttons.switchProfile")}
            subtitle={t("novice.buttons.switchProfileSubtitle")}
            onClick={() => handleNavigate("profile")}
            disabled={!hasAnyCli}
            disabledReason={t("novice.buttons.switchProfileDisabledReason")}
          />
          <BigButton
            testId="novice-button-launch"
            icon={<Play className="h-9 w-9" />}
            title={t("novice.buttons.launch")}
            subtitle={t("novice.buttons.launchSubtitle")}
            onClick={() => handleNavigate("launcher")}
            disabled={!hasAnyCli || !hasActiveProfile}
            disabledReason={
              !hasAnyCli
                ? t("novice.buttons.launchDisabledNoCli")
                : t("novice.buttons.launchDisabledNoProfile")
            }
          />
        </div>
      </main>

      <StatusBar />

      <OnboardingDialog open={onboardingOpen} />
    </div>
  );
}

export default NoviceHome;
