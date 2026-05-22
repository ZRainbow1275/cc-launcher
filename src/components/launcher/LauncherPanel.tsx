import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery } from "@tanstack/react-query";
import { toast } from "sonner";
import { AlertTriangle, Loader2, PlayCircle } from "lucide-react";

import {
  cliState,
  launcher,
  profile as profileApi,
  sandbox,
} from "@/lib/api/mock";
import type {
  LaunchResult,
  Profile,
  TargetCli,
  TypedError,
} from "@/lib/api/contracts";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

import { TerminalPicker } from "./TerminalPicker";
import { WorkdirInfo } from "./WorkdirInfo";
import { SafetySummary } from "./SafetySummary";

type ErrorKind =
  | "node_missing"
  | "cli_missing"
  | "profile_invalid"
  | "terminal_not_found"
  | "unknown";

interface ErrorState {
  kind: ErrorKind;
  detail: string;
  cliName?: string;
}

export interface LauncherPanelProps {
  initialCli?: TargetCli;
  onNavigateProfileManager?: () => void;
  onNavigateInstaller?: () => void;
}

function classifyError(err: TypedError | undefined): ErrorState {
  if (!err) return { kind: "unknown", detail: "" };
  switch (err.code) {
    case "NODE_MISSING":
      return { kind: "node_missing", detail: err.message.zh };
    case "CLI_MISSING":
      return { kind: "cli_missing", detail: err.message.zh };
    case "PROFILE_NOT_FOUND":
      return { kind: "profile_invalid", detail: err.message.zh };
    case "NO_TERMINAL_AVAILABLE":
      return { kind: "terminal_not_found", detail: err.message.zh };
    default:
      return { kind: "unknown", detail: err.message.zh };
  }
}

function cwdAbsoluteFor(profileId: string): string {
  return `C:\\Users\\you\\cc-launcher-projects\\${profileId}`;
}

function cwdDisplayFor(profileId: string): string {
  return `~/cc-launcher-projects/${profileId}`;
}

export function LauncherPanel({
  initialCli = "claude",
  onNavigateProfileManager,
  onNavigateInstaller,
}: LauncherPanelProps) {
  const { t } = useTranslation();
  const [cli, setCli] = useState<TargetCli>(initialCli);
  const [selectedTerminalId, setSelectedTerminalId] = useState<string | null>(
    null,
  );
  const [error, setError] = useState<ErrorState | null>(null);
  const [isOpeningWorkdir, setIsOpeningWorkdir] = useState(false);

  const activeQuery = useQuery({
    queryKey: ["launcher", "active", cli],
    queryFn: () => cliState.get_active(cli),
  });

  const activeProfileId = activeQuery.data ?? null;

  const profileQuery = useQuery({
    queryKey: ["launcher", "profile", activeProfileId, cli],
    queryFn: () =>
      activeProfileId
        ? profileApi.get(activeProfileId, cli)
        : Promise.resolve(null),
    enabled: !!activeProfileId,
  });

  const activeProfile: Profile | null = profileQuery.data ?? null;

  const terminalsQuery = useQuery({
    queryKey: ["launcher", "terminals"],
    queryFn: () => launcher.detect_terminals(),
  });

  const terminals = terminalsQuery.data ?? [];

  const safetyQuery = useQuery({
    queryKey: [
      "launcher",
      "safety_summary",
      activeProfileId ?? "",
      cli,
    ] as const,
    queryFn: () =>
      activeProfileId
        ? launcher.get_safety_summary({
            profile_id: activeProfileId,
            target_cli: cli,
          })
        : Promise.resolve(undefined),
    enabled: !!activeProfileId,
  });

  const l1RulesQuery = useQuery({
    queryKey: ["launcher", "l1_count"],
    queryFn: () => sandbox.get_l1_rules(),
  });

  const l2RedlinesQuery = useQuery({
    queryKey: ["launcher", "l2_count"],
    queryFn: () => sandbox.list_l2_redlines(),
  });

  const safetySummary = useMemo(() => {
    if (!safetyQuery.data) return undefined;
    return {
      ...safetyQuery.data,
      l1ActiveCount:
        l1RulesQuery.data?.filter((r) => r.enabled).length ??
        safetyQuery.data.l1ActiveCount,
      l2RedlineCount:
        l2RedlinesQuery.data?.length ?? safetyQuery.data.l2RedlineCount,
    };
  }, [safetyQuery.data, l1RulesQuery.data, l2RedlinesQuery.data]);

  useEffect(() => {
    setSelectedTerminalId(null);
  }, [cli]);

  const startMutation = useMutation({
    mutationFn: (vars: {
      profileId: string;
      targetCli: TargetCli;
      terminalId: string;
      cwd: string;
    }): Promise<LaunchResult> =>
      launcher.start_cli({
        profile_id: vars.profileId,
        target_cli: vars.targetCli,
        terminal_id: vars.terminalId,
        cwd: vars.cwd,
      }),
    onSuccess: (result, vars) => {
      if (result.success) {
        const terminalName =
          terminals.find((t) => t.id === vars.terminalId)?.displayName ??
          vars.terminalId;
        const profileName = activeProfile?.name ?? vars.profileId;
        toast.success(t("launcher.success.title", { terminal: terminalName }), {
          description: t("launcher.success.body", {
            profile: profileName,
            cwd: result.cwd,
          }),
        });
      } else {
        setError(classifyError(result.error));
      }
    },
    onError: (err: unknown) => {
      const detail =
        err && typeof err === "object" && "message" in err
          ? typeof (err as { message: unknown }).message === "string"
            ? String((err as { message: string }).message)
            : JSON.stringify((err as { message: unknown }).message)
          : String(err);
      setError({ kind: "unknown", detail });
    },
  });

  const handleOpenWorkdir = useCallback(async () => {
    if (!activeProfileId) return;
    setIsOpeningWorkdir(true);
    try {
      const res = await launcher.open_workdir(activeProfileId);
      if (res.success) {
        toast.success(t("launcher.workdir.openSuccess"));
      } else {
        toast.error(t("launcher.workdir.openFailed"));
      }
    } catch {
      toast.error(t("launcher.workdir.openFailed"));
    } finally {
      setIsOpeningWorkdir(false);
    }
  }, [activeProfileId, t]);

  const handleLaunch = useCallback(() => {
    if (!activeProfileId || !selectedTerminalId) return;
    setError(null);
    startMutation.mutate({
      profileId: activeProfileId,
      targetCli: cli,
      terminalId: selectedTerminalId,
      cwd: cwdAbsoluteFor(activeProfileId),
    });
  }, [activeProfileId, cli, selectedTerminalId, startMutation]);

  const launchDisabledReason = (() => {
    if (startMutation.isPending) return t("launcher.disabledReason.launching");
    if (!activeProfileId) return t("launcher.disabledReason.noProfile");
    if (!selectedTerminalId) return t("launcher.disabledReason.noTerminal");
    return null;
  })();

  const canLaunch = launchDisabledReason === null;

  return (
    <div
      className="flex h-full flex-col gap-4 p-4"
      data-testid="launcher-panel"
    >
      <header className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold">{t("launcher.title")}</h1>
          <p className="text-xs text-muted-foreground">
            {t("launcher.subtitle")}
          </p>
        </div>
        <Tabs value={cli} onValueChange={(next) => setCli(next as TargetCli)}>
          <TabsList data-testid="launcher-cli-tabs">
            <TabsTrigger value="claude" data-testid="launcher-cli-tab-claude">
              {t("profile.cli.claude")}
            </TabsTrigger>
            <TabsTrigger value="codex" data-testid="launcher-cli-tab-codex">
              {t("profile.cli.codex")}
            </TabsTrigger>
          </TabsList>
        </Tabs>
      </header>

      {/* Upper half: Active Profile */}
      <section data-testid="launcher-active-profile">
        {!activeProfileId || !activeProfile ? (
          <div
            data-testid="launcher-active-profile-empty"
            className="flex flex-col items-start gap-2 rounded-xl border border-dashed border-border bg-muted/30 p-5"
          >
            <div className="flex items-center gap-2 text-sm font-semibold">
              <AlertTriangle className="h-4 w-4 text-amber-500" />
              {t("launcher.activeProfile.empty")}
            </div>
            <p className="text-xs text-muted-foreground">
              {t("launcher.activeProfile.emptyHint")}
            </p>
            <Button
              variant="link"
              size="sm"
              data-testid="launcher-active-profile-create-link"
              onClick={() => onNavigateProfileManager?.()}
            >
              {t("launcher.activeProfile.createLink")}
            </Button>
          </div>
        ) : (
          <div
            data-testid="launcher-active-profile-card"
            className="flex flex-col gap-3 rounded-xl border border-border bg-card/50 p-4"
          >
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-3">
                <span
                  data-testid="launcher-profile-icon"
                  className="inline-flex h-9 w-9 items-center justify-center rounded-md text-white"
                  style={{
                    backgroundColor: activeProfile.icon_color ?? "#3b82f6",
                  }}
                >
                  <PlayCircle className="h-5 w-5" />
                </span>
                <div>
                  <div className="flex items-center gap-2">
                    <span
                      className="text-base font-semibold"
                      data-testid="launcher-profile-name"
                    >
                      {activeProfile.name}
                    </span>
                    <Badge variant="outline" className="text-[10px] uppercase">
                      {cli}
                    </Badge>
                  </div>
                  <div className="mt-0.5 text-xs text-muted-foreground">
                    <span data-testid="launcher-profile-provider">
                      {t("launcher.activeProfile.providerLabel")}:{" "}
                      {activeProfile.provider_id ?? (
                        <em>{t("launcher.activeProfile.noProvider")}</em>
                      )}
                    </span>
                  </div>
                </div>
              </div>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              <Badge
                variant="secondary"
                data-testid="launcher-profile-mcp-count"
              >
                {t("launcher.activeProfile.mcpCount", {
                  count: activeProfile.mcp_ids.length,
                })}
              </Badge>
              <Badge
                variant="secondary"
                data-testid="launcher-profile-skills-count"
              >
                {t("launcher.activeProfile.skillsCount", {
                  count: activeProfile.skill_ids.length,
                })}
              </Badge>
            </div>
          </div>
        )}
      </section>

      {/* Lower half: launch setup */}
      <div className="grid gap-4 md:grid-cols-2">
        <TerminalPicker
          terminals={terminals}
          selectedId={selectedTerminalId}
          isLoading={terminalsQuery.isLoading}
          onSelect={setSelectedTerminalId}
        />
        {activeProfileId ? (
          <WorkdirInfo
            cwdDisplay={cwdDisplayFor(activeProfileId)}
            cwdAbsolute={cwdAbsoluteFor(activeProfileId)}
            isOpening={isOpeningWorkdir}
            onOpen={handleOpenWorkdir}
          />
        ) : (
          <div
            data-testid="launcher-workdir-placeholder"
            className="rounded-xl border border-dashed border-border bg-muted/20 p-4 text-xs text-muted-foreground"
          >
            {t("launcher.activeProfile.emptyHint")}
          </div>
        )}
      </div>

      <SafetySummary
        targetCli={cli}
        data={safetySummary}
        isLoading={safetyQuery.isLoading}
      />

      <div className="mt-auto flex flex-col items-stretch gap-2 pt-2">
        <Button
          size="lg"
          variant="default"
          className="h-12 text-base font-semibold"
          onClick={handleLaunch}
          disabled={!canLaunch}
          data-testid="launcher-launch-button"
          title={launchDisabledReason ?? undefined}
        >
          {startMutation.isPending ? (
            <>
              <Loader2 className="mr-2 h-5 w-5 animate-spin" />
              {t("launcher.launching")}
            </>
          ) : (
            <>
              <PlayCircle className="mr-2 h-5 w-5" />
              {t("launcher.launchButton")}
            </>
          )}
        </Button>
        {!canLaunch && launchDisabledReason ? (
          <p
            className="text-center text-xs text-muted-foreground"
            data-testid="launcher-launch-disabled-reason"
          >
            {launchDisabledReason}
          </p>
        ) : null}
      </div>

      <LaunchErrorDialog
        error={error}
        cli={cli}
        onClose={() => setError(null)}
        onNavigateInstaller={onNavigateInstaller}
        onNavigateProfileManager={onNavigateProfileManager}
      />
    </div>
  );
}

interface LaunchErrorDialogProps {
  error: ErrorState | null;
  cli: TargetCli;
  onClose: () => void;
  onNavigateInstaller?: () => void;
  onNavigateProfileManager?: () => void;
}

function LaunchErrorDialog({
  error,
  cli,
  onClose,
  onNavigateInstaller,
  onNavigateProfileManager,
}: LaunchErrorDialogProps) {
  const { t } = useTranslation();
  const isOpen = error !== null;

  const messageKey = error
    ? (
        {
          node_missing: "launcher.error.nodeMissing",
          cli_missing: "launcher.error.cliMissing",
          profile_invalid: "launcher.error.profileInvalid",
          terminal_not_found: "launcher.error.terminalNotFound",
          unknown: "launcher.error.unknown",
        } as const
      )[error.kind]
    : "";

  const messageOpts: Record<string, string | undefined> = error
    ? {
        detail: error.detail,
        cli: cli === "claude" ? "Claude Code" : "Codex",
      }
    : {};

  const showInstallerLink =
    error?.kind === "node_missing" || error?.kind === "cli_missing";
  const showProfileLink = error?.kind === "profile_invalid";

  return (
    <Dialog open={isOpen} onOpenChange={(open) => !open && onClose()}>
      <DialogContent data-testid="launcher-error-dialog">
        <DialogHeader>
          <DialogTitle>{t("launcher.error.title")}</DialogTitle>
          <DialogDescription
            data-testid="launcher-error-message"
            className="pt-2"
          >
            {error ? t(messageKey, messageOpts) : null}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button
            variant="outline"
            onClick={onClose}
            data-testid="launcher-error-close"
          >
            {t("launcher.error.close")}
          </Button>
          {showInstallerLink ? (
            <Button
              variant="default"
              data-testid="launcher-error-fix-link"
              onClick={() => {
                onClose();
                onNavigateInstaller?.();
              }}
            >
              {t("launcher.error.fixLink")}
            </Button>
          ) : null}
          {showProfileLink ? (
            <Button
              variant="default"
              data-testid="launcher-error-fix-profile-link"
              onClick={() => {
                onClose();
                onNavigateProfileManager?.();
              }}
            >
              {t("launcher.error.fixProfileLink")}
            </Button>
          ) : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
