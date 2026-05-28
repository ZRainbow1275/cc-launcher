import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  ArrowLeft,
  ArrowRight,
  CheckCircle,
  ChevronRight,
  Loader2,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Checkbox } from "@/components/ui/checkbox";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import {
  SystemCheckSummary,
  SYSTEM_PROBE_QUERY_KEY,
} from "@/components/system-check";
import { installer, profile as profileApi } from "@/lib/api/mock";
import type {
  CliInstallStatus,
  InstallPhase,
  InstallProgress as InstallProgressEvent,
  InstallerSourceConfig,
  NodeStatus,
  RegistryPickResult,
  SystemProbeReport,
  TargetCli,
} from "@/lib/api/contracts";
import { systemProbe } from "@/lib/api/mock";

import { CliCard } from "./CliCard";
import { InstallProgress } from "./InstallProgress";
import { InstallerSourceSettings } from "./InstallerSourceSettings";
import { REGISTRY_PROBE_QUERY_KEY, RegistryPicker } from "./RegistryPicker";

type WizardStep = 1 | 2 | 3 | 4;

interface PerCliState {
  events: InstallProgressEvent[];
  isStreaming: boolean;
  cancelled: boolean;
  failed: boolean;
  completed: boolean;
  installedVersion?: string;
  attemptedRegistry?: string;
}

function stringifyInstallError(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function syntheticInstallFailureEvent(
  registry: string,
  cause: string,
): InstallProgressEvent {
  return {
    phase: "failed",
    message: {
      zh: "安装失败，未收到后端失败事件",
      en: "Install failed before the backend delivered a failure event",
      ja: "バックエンドの失敗イベント到着前にインストールが失敗しました",
    },
    percent: 0,
    registry,
    error: {
      code: "INSTALL_STREAM_REJECTED",
      message: {
        zh: "安装通道异常中断",
        en: "Install stream ended with an error",
        ja: "インストールストリームがエラーで終了しました",
      },
      cause,
      retryable: true,
    },
  };
}

function emptyCliState(): PerCliState {
  return {
    events: [],
    isStreaming: false,
    cancelled: false,
    failed: false,
    completed: false,
  };
}

interface InstallerWizardProps {
  initialStep?: WizardStep;
  onComplete?: () => void;
  onViewProfiles?: (cli: TargetCli) => void;
}

function detectAllClis(): Promise<Record<TargetCli, CliInstallStatus>> {
  return Promise.all([
    installer.detect_cli("claude"),
    installer.detect_cli("codex"),
  ]).then(([claude, codex]) => ({ claude, codex }));
}

const NODE_PATH_WIN = "%LOCALAPPDATA%\\cc-switch\\runtime\\node\\";
const NODE_PATH_MAC = "~/Library/Application Support/cc-switch/runtime/node/";

function osNodePath(): string {
  if (typeof navigator !== "undefined") {
    if (navigator.platform.toLowerCase().includes("mac")) return NODE_PATH_MAC;
  }
  return NODE_PATH_WIN;
}

export function InstallerWizard({
  initialStep = 1,
  onComplete,
  onViewProfiles,
}: InstallerWizardProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [step, setStep] = useState<WizardStep>(initialStep);
  const [selectedClis, setSelectedClis] = useState<Record<TargetCli, boolean>>({
    claude: true,
    codex: true,
  });
  const [chosenRegistry, setChosenRegistry] = useState<string | null>(null);
  const [installState, setInstallState] = useState<
    Record<TargetCli, PerCliState>
  >({
    claude: emptyCliState(),
    codex: emptyCliState(),
  });
  const [currentInstallingCli, setCurrentInstallingCli] =
    useState<TargetCli | null>(null);
  const [showCardView, setShowCardView] = useState(false);
  const [pendingUninstall, setPendingUninstall] = useState<TargetCli | null>(
    null,
  );
  const cancelRef = useRef<Record<TargetCli, boolean>>({
    claude: false,
    codex: false,
  });

  const probeQuery = useQuery<SystemProbeReport>({
    queryKey: SYSTEM_PROBE_QUERY_KEY,
    queryFn: () => systemProbe.run(),
  });

  const detectAllQuery = useQuery({
    queryKey: ["installer", "detect_all"],
    queryFn: detectAllClis,
  });

  const detectNodeQuery = useQuery<NodeStatus>({
    queryKey: ["installer", "detect_node"],
    queryFn: () => installer.detect_node(),
  });

  const profileClaudeQuery = useQuery({
    queryKey: ["profile", "list", "claude"],
    queryFn: () => profileApi.list("claude"),
  });
  const profileCodexQuery = useQuery({
    queryKey: ["profile", "list", "codex"],
    queryFn: () => profileApi.list("codex"),
  });

  useEffect(() => {
    if (!detectAllQuery.data) return;
    setSelectedClis((prev) => ({
      claude: detectAllQuery.data.claude.installed ? true : prev.claude,
      codex: detectAllQuery.data.codex.installed ? true : prev.codex,
    }));
  }, [detectAllQuery.data]);

  const probeCounts = useMemo(() => {
    const items = probeQuery.data?.items ?? [];
    let red = 0;
    let yellow = 0;
    for (const it of items) {
      if (it.status === "red" || it.status === "missing") red += 1;
      else if (it.status === "yellow") yellow += 1;
    }
    return { red, yellow };
  }, [probeQuery.data]);

  const installNodeMutation = useMutation({
    mutationFn: async (): Promise<NodeStatus> => {
      const stream = installer.install_node();
      for await (const evt of stream) {
        if (evt.phase === "failed") {
          throw evt.error ?? new Error("Install Node failed");
        }
      }
      return installer.detect_node();
    },
    onSuccess: (next) => {
      queryClient.setQueryData(["installer", "detect_node"], next);
    },
  });

  const uninstallMutation = useMutation({
    mutationFn: (cli: TargetCli) => installer.uninstall_cli(cli),
    onSuccess: (_res, cli) => {
      void queryClient.invalidateQueries({
        queryKey: ["installer", "detect_all"],
      });
      setInstallState((prev) => ({ ...prev, [cli]: emptyCliState() }));
      setPendingUninstall(null);
    },
    onError: () => {
      setPendingUninstall(null);
    },
  });

  const updatePerCli = useCallback(
    (cli: TargetCli, patch: Partial<PerCliState>) => {
      setInstallState((prev) => ({
        ...prev,
        [cli]: { ...prev[cli], ...patch },
      }));
    },
    [],
  );

  const runInstall = useCallback(
    async (cli: TargetCli, registry: string) => {
      cancelRef.current[cli] = false;
      setCurrentInstallingCli(cli);
      updatePerCli(cli, {
        events: [],
        isStreaming: true,
        failed: false,
        completed: false,
        cancelled: false,
        attemptedRegistry: registry,
      });

      const stream = installer.install_cli(cli, { registry });
      const collected: InstallProgressEvent[] = [];
      let lastPhase: InstallPhase | null = null;
      try {
        for await (const evt of stream) {
          if (cancelRef.current[cli]) {
            break;
          }
          collected.push(evt);
          lastPhase = evt.phase;
          updatePerCli(cli, { events: [...collected] });
        }
      } catch (error: unknown) {
        const failedEvent = syntheticInstallFailureEvent(
          registry,
          stringifyInstallError(error),
        );
        collected.push(failedEvent);
        updatePerCli(cli, {
          events: [...collected],
          failed: true,
          isStreaming: false,
        });
        return;
      }
      const completed = lastPhase === "completed";
      const failed = lastPhase === "failed";
      let installedVersion: string | undefined;
      if (completed) {
        const fresh = await installer.detect_cli(cli);
        installedVersion = fresh.version;
        queryClient.setQueryData<Record<TargetCli, CliInstallStatus>>(
          ["installer", "detect_all"],
          (prev) =>
            prev ? { ...prev, [cli]: fresh } : { claude: fresh, codex: fresh },
        );
      }
      updatePerCli(cli, {
        isStreaming: false,
        completed,
        failed,
        installedVersion,
      });
    },
    [queryClient, updatePerCli],
  );

  const handleNext = useCallback(() => {
    setStep((s) => (s < 4 ? ((s + 1) as WizardStep) : s));
  }, []);

  const handleBack = useCallback(() => {
    setStep((s) => (s > 1 ? ((s - 1) as WizardStep) : s));
  }, []);

  const selectedList = useMemo(
    () =>
      (Object.keys(selectedClis) as TargetCli[]).filter(
        (cli) => selectedClis[cli],
      ),
    [selectedClis],
  );

  const remainingToInstall = useMemo(
    () =>
      selectedList.filter((cli) => {
        const installed = detectAllQuery.data?.[cli].installed;
        const state = installState[cli];
        if (installed && !state.failed) return false;
        return !state.completed;
      }),
    [selectedList, detectAllQuery.data, installState],
  );

  const sortedAvailableRegistries = useMemo(() => {
    const cached = queryClient.getQueryData<RegistryPickResult>(
      REGISTRY_PROBE_QUERY_KEY,
    );
    if (!cached) return [];
    return [...cached.candidates]
      .filter((c) => c.ok)
      .sort((a, b) => a.latencyMs - b.latencyMs)
      .map((c) => c.url);
  }, [queryClient, installState]);

  const handleStartInstallStep4 = useCallback(async () => {
    if (!chosenRegistry) return;
    const next = remainingToInstall[0];
    if (!next) {
      setShowCardView(true);
      onComplete?.();
      return;
    }
    await runInstall(next, chosenRegistry);
  }, [chosenRegistry, remainingToInstall, runInstall, onComplete]);

  useEffect(() => {
    if (step !== 4 || showCardView) return;
    if (currentInstallingCli) return;
    if (remainingToInstall.length === 0) {
      setShowCardView(true);
      onComplete?.();
    }
  }, [
    step,
    showCardView,
    currentInstallingCli,
    remainingToInstall,
    onComplete,
  ]);

  useEffect(() => {
    if (!currentInstallingCli) return;
    const state = installState[currentInstallingCli];
    if (state.failed) {
      setCurrentInstallingCli(null);
      return;
    }
    if (state.completed) {
      setCurrentInstallingCli(null);
      const rest = remainingToInstall.filter((c) => c !== currentInstallingCli);
      if (rest.length > 0 && chosenRegistry) {
        void runInstall(rest[0]!, chosenRegistry);
      }
    }
  }, [
    installState,
    currentInstallingCli,
    remainingToInstall,
    chosenRegistry,
    runInstall,
  ]);

  const handleCancel = useCallback(
    (cli: TargetCli) => {
      cancelRef.current[cli] = true;
      updatePerCli(cli, {
        cancelled: true,
        isStreaming: false,
      });
      setCurrentInstallingCli(null);
    },
    [updatePerCli],
  );

  const handleRetrySame = useCallback(
    (cli: TargetCli) => {
      const reg = installState[cli].attemptedRegistry ?? chosenRegistry;
      if (!reg) return;
      void runInstall(cli, reg);
    },
    [installState, chosenRegistry, runInstall],
  );

  const handleRetryDifferent = useCallback(
    (cli: TargetCli) => {
      const tried = installState[cli].attemptedRegistry;
      const next = sortedAvailableRegistries.find((u) => u !== tried);
      if (!next) return;
      setChosenRegistry(next);
      void runInstall(cli, next);
    },
    [installState, sortedAvailableRegistries, runInstall],
  );

  const handleInstallerSourceSaved = useCallback(
    (config: InstallerSourceConfig) => {
      setChosenRegistry(config.npmRegistry ?? null);
    },
    [],
  );

  const profileCount = useCallback(
    (cli: TargetCli) => {
      if (cli === "claude") return profileClaudeQuery.data?.length ?? 0;
      return profileCodexQuery.data?.length ?? 0;
    },
    [profileClaudeQuery.data, profileCodexQuery.data],
  );

  const stepNum = step;
  const step1Block =
    probeCounts.red > 0 ? "red" : probeCounts.yellow > 0 ? "yellow" : "green";

  const nodeReady =
    detectNodeQuery.data?.installed &&
    (detectNodeQuery.data?.majorVersion ?? 0) >= 20;

  const canAdvanceStep1 = step1Block !== "red";
  const canAdvanceStep2 = Boolean(nodeReady);
  const canAdvanceStep3 = selectedList.length > 0;

  if (showCardView) {
    return (
      <CardViewSection
        cliStatus={detectAllQuery.data}
        profileCount={profileCount}
        onUninstallRequest={(cli) => setPendingUninstall(cli)}
        onViewProfiles={(cli) => onViewProfiles?.(cli)}
        onReopen={() => {
          setShowCardView(false);
          setStep(1);
        }}
        pendingUninstall={pendingUninstall}
        uninstallPending={uninstallMutation.isPending}
        onCancelUninstall={() => setPendingUninstall(null)}
        onConfirmUninstall={(cli) => uninstallMutation.mutate(cli)}
      />
    );
  }

  return (
    <div
      data-testid="installer-wizard"
      data-step={String(stepNum)}
      className="p-6 space-y-4 max-w-3xl mx-auto"
    >
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-lg">{t("installer.title")}</CardTitle>
          <p className="text-xs text-muted-foreground">
            {t("installer.subtitle")}
          </p>
        </CardHeader>
        <CardContent className="pt-0">
          <StepIndicator current={stepNum} />
        </CardContent>
      </Card>

      {stepNum === 1 ? (
        <Card data-testid="installer-step-1">
          <CardContent className="pt-6 space-y-4">
            <SystemCheckSummary variant="row" />
            {step1Block === "red" ? (
              <Alert
                variant="destructive"
                data-testid="installer-step-1-red-block"
              >
                <AlertTitle>{t("installer.step1.redBlocks")}</AlertTitle>
              </Alert>
            ) : step1Block === "yellow" ? (
              <Alert data-testid="installer-step-1-yellow-warn">
                <AlertTitle>{t("installer.step1.yellowWarn")}</AlertTitle>
              </Alert>
            ) : (
              <Alert data-testid="installer-step-1-ok">
                <AlertTitle>{t("installer.step1.continue")}</AlertTitle>
              </Alert>
            )}
          </CardContent>
        </Card>
      ) : null}

      {stepNum === 2 ? (
        <div className="space-y-4" data-testid="installer-step-2">
          <InstallerSourceSettings onSaved={handleInstallerSourceSaved} />
          <Card>
            <CardContent className="pt-6 space-y-4">
              {detectNodeQuery.isLoading ? (
                <div className="flex items-center gap-2 text-sm text-muted-foreground">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  {t("installer.step2.detecting")}
                </div>
              ) : nodeReady ? (
                <Alert data-testid="installer-step-2-ready">
                  <CheckCircle className="h-4 w-4" />
                  <AlertTitle>
                    {t("installer.step2.ready", {
                      version: detectNodeQuery.data?.version ?? "",
                    })}
                  </AlertTitle>
                  <AlertDescription>
                    <span className="text-xs text-muted-foreground">
                      {t("installer.step2.pathLabel")}: {osNodePath()}
                    </span>
                  </AlertDescription>
                </Alert>
              ) : (
                <Alert data-testid="installer-step-2-missing">
                  <AlertTitle>{t("installer.step2.missing")}</AlertTitle>
                  <AlertDescription className="space-y-3">
                    <p className="text-xs">
                      {t("installer.step2.pathLabel")}:{" "}
                      <span className="font-mono">{osNodePath()}</span>
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {t("installer.step2.installNote")}
                    </p>
                    <p className="text-xs text-muted-foreground">
                      {t("installer.step2.uninstallNote")}
                    </p>
                    <Button
                      type="button"
                      size="sm"
                      disabled={installNodeMutation.isPending}
                      onClick={() => installNodeMutation.mutate()}
                      data-testid="installer-step-2-install-node"
                    >
                      {installNodeMutation.isPending ? (
                        <Loader2 className="h-4 w-4 animate-spin mr-1" />
                      ) : null}
                      {installNodeMutation.isPending
                        ? t("installer.step2.installing")
                        : t("installer.step2.installButton")}
                    </Button>
                  </AlertDescription>
                </Alert>
              )}
            </CardContent>
          </Card>
        </div>
      ) : null}

      {stepNum === 3 ? (
        <Card data-testid="installer-step-3">
          <CardContent className="pt-6 space-y-4">
            <div>
              <p className="font-medium text-sm">
                {t("installer.step3.title")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("installer.step3.description")}
              </p>
            </div>
            {(["claude", "codex"] as TargetCli[]).map((cli) => {
              const installed = detectAllQuery.data?.[cli].installed;
              return (
                <label
                  key={cli}
                  className="flex items-center gap-3 px-3 py-2 rounded-md border hover:bg-muted/40 cursor-pointer"
                  data-testid={`installer-step-3-row-${cli}`}
                >
                  <Checkbox
                    checked={selectedClis[cli]}
                    onCheckedChange={(v) =>
                      setSelectedClis((prev) => ({
                        ...prev,
                        [cli]: Boolean(v),
                      }))
                    }
                    data-testid={`installer-step-3-checkbox-${cli}`}
                  />
                  <span className="flex-1 text-sm">
                    {t(`installer.step3.${cli}`)}
                  </span>
                  {installed ? (
                    <span
                      data-testid={`installer-step-3-installed-${cli}`}
                      className="text-xs rounded bg-emerald-100 dark:bg-emerald-900/40 text-emerald-900 dark:text-emerald-100 px-2 py-0.5"
                    >
                      {t("installer.step3.installed")}
                    </span>
                  ) : null}
                </label>
              );
            })}
            {selectedList.length === 0 ? (
              <p
                className="text-xs text-red-600"
                data-testid="installer-step-3-empty"
              >
                {t("installer.step3.noneSelected")}
              </p>
            ) : null}
          </CardContent>
        </Card>
      ) : null}

      {stepNum === 4 ? (
        <div className="space-y-4" data-testid="installer-step-4">
          <InstallerSourceSettings onSaved={handleInstallerSourceSaved} />
          <RegistryPicker
            selectedUrl={chosenRegistry}
            onSelect={(url) => setChosenRegistry(url)}
          />
          {chosenRegistry ? (
            <Alert data-testid="installer-step-4-selected-source">
              <AlertTitle>
                {t("installer.step4.registryPicker.currentSource")}
              </AlertTitle>
              <AlertDescription className="font-mono break-all text-xs">
                {chosenRegistry}
              </AlertDescription>
            </Alert>
          ) : null}
          {chosenRegistry ? (
            <div className="flex justify-end">
              <Button
                type="button"
                onClick={handleStartInstallStep4}
                disabled={
                  Boolean(currentInstallingCli) ||
                  remainingToInstall.length === 0
                }
                data-testid="installer-step-4-start"
              >
                {t("installer.step4.registryPicker.confirmSelection")}
              </Button>
            </div>
          ) : null}
          {selectedList.map((cli) => {
            const state = installState[cli];
            if (state.events.length === 0 && !state.isStreaming) return null;
            return (
              <InstallProgress
                key={cli}
                cli={cli}
                events={state.events}
                isStreaming={state.isStreaming}
                installedVersion={state.installedVersion}
                onCancel={() => handleCancel(cli)}
                onRetrySameMirror={() => handleRetrySame(cli)}
                onRetryDifferentMirror={() => handleRetryDifferent(cli)}
              />
            );
          })}
          {selectedList.every((c) => installState[c].completed) &&
          selectedList.length > 0 ? (
            <Alert data-testid="installer-step-4-all-done">
              <AlertTitle>{t("installer.step4.allDone")}</AlertTitle>
            </Alert>
          ) : null}
        </div>
      ) : null}

      <div className="flex items-center justify-between">
        <Button
          type="button"
          variant="outline"
          disabled={stepNum === 1}
          onClick={handleBack}
          data-testid="installer-back"
        >
          <ArrowLeft className="h-4 w-4 mr-1" />
          {t("installer.nav.back")}
        </Button>
        {stepNum < 4 ? (
          <Button
            type="button"
            onClick={handleNext}
            disabled={
              (stepNum === 1 && !canAdvanceStep1) ||
              (stepNum === 2 && !canAdvanceStep2) ||
              (stepNum === 3 && !canAdvanceStep3)
            }
            data-testid="installer-next"
          >
            {t("installer.nav.next")}
            <ArrowRight className="h-4 w-4 ml-1" />
          </Button>
        ) : (
          <Button
            type="button"
            onClick={() => {
              setShowCardView(true);
              onComplete?.();
            }}
            disabled={
              selectedList.length === 0 ||
              !selectedList.every((c) => installState[c].completed)
            }
            data-testid="installer-finish"
          >
            {t("installer.nav.finish")}
            <ChevronRight className="h-4 w-4 ml-1" />
          </Button>
        )}
      </div>
    </div>
  );
}

interface StepIndicatorProps {
  current: WizardStep;
}

const STEP_KEYS: { id: WizardStep; key: string }[] = [
  { id: 1, key: "installer.steps.systemCheck.title" },
  { id: 2, key: "installer.steps.node.title" },
  { id: 3, key: "installer.steps.select.title" },
  { id: 4, key: "installer.steps.install.title" },
];

function StepIndicator({ current }: StepIndicatorProps) {
  const { t } = useTranslation();
  return (
    <ol
      className="flex items-center gap-2 text-xs"
      data-testid="installer-step-indicator"
    >
      {STEP_KEYS.map((s, idx) => (
        <li key={s.id} className="flex items-center gap-2">
          <span
            data-testid={`installer-step-pill-${s.id}`}
            data-active={s.id === current ? "true" : "false"}
            data-done={s.id < current ? "true" : "false"}
            className={`px-2.5 py-1 rounded-full font-mono ${
              s.id === current
                ? "bg-blue-500 text-white"
                : s.id < current
                  ? "bg-emerald-500 text-white"
                  : "bg-muted text-muted-foreground"
            }`}
          >
            {s.id}
          </span>
          <span
            className={
              s.id === current ? "font-medium" : "text-muted-foreground"
            }
          >
            {t(s.key)}
          </span>
          {idx < STEP_KEYS.length - 1 ? (
            <ChevronRight className="h-3 w-3 text-muted-foreground" />
          ) : null}
        </li>
      ))}
    </ol>
  );
}

interface CardViewSectionProps {
  cliStatus: Record<TargetCli, CliInstallStatus> | undefined;
  profileCount: (cli: TargetCli) => number;
  onUninstallRequest: (cli: TargetCli) => void;
  onViewProfiles: (cli: TargetCli) => void;
  onReopen: () => void;
  pendingUninstall: TargetCli | null;
  uninstallPending: boolean;
  onCancelUninstall: () => void;
  onConfirmUninstall: (cli: TargetCli) => void;
}

function CardViewSection({
  cliStatus,
  profileCount,
  onUninstallRequest,
  onViewProfiles,
  onReopen,
  pendingUninstall,
  uninstallPending,
  onCancelUninstall,
  onConfirmUninstall,
}: CardViewSectionProps) {
  const { t } = useTranslation();
  const clis: TargetCli[] = ["claude", "codex"];
  return (
    <div
      data-testid="installer-card-view"
      className="p-6 space-y-4 max-w-3xl mx-auto"
    >
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle className="text-lg">
              {t("installer.cardView.title")}
            </CardTitle>
            <p className="text-xs text-muted-foreground">
              {t("installer.cardView.subtitle")}
            </p>
          </div>
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={onReopen}
            data-testid="installer-card-view-reopen"
          >
            {t("installer.cardView.reinstall")}
          </Button>
        </CardHeader>
      </Card>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
        {clis.map((cli) => {
          const status =
            cliStatus?.[cli] ??
            ({
              cli,
              installed: false,
              lastChecked: new Date().toISOString(),
            } as CliInstallStatus);
          return (
            <CliCard
              key={cli}
              cli={cli}
              status={status}
              profileCount={profileCount(cli)}
              isUninstalling={uninstallPending && pendingUninstall === cli}
              onUninstall={() => onUninstallRequest(cli)}
              onViewProfiles={() => onViewProfiles(cli)}
            />
          );
        })}
      </div>
      <ConfirmDialog
        isOpen={pendingUninstall !== null}
        title={t("installer.cardView.uninstallConfirmTitle", {
          cli: pendingUninstall ?? "",
        })}
        message={t("installer.cardView.uninstallConfirm", {
          cli: pendingUninstall ?? "",
        })}
        variant="destructive"
        onCancel={onCancelUninstall}
        onConfirm={() =>
          pendingUninstall && onConfirmUninstall(pendingUninstall)
        }
      />
    </div>
  );
}
