import { useTranslation } from "react-i18next";
import {
  ArrowRight,
  Boxes,
  ChevronRight,
  Download,
  PlayCircle,
  Settings as SettingsIcon,
  ShieldCheck,
  Stethoscope,
} from "lucide-react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";

export type LauncherStep =
  | "systemCheck"
  | "sandbox"
  | "install"
  | "profile"
  | "launch";

export interface LauncherHomePageProps {
  onNavigate: (step: LauncherStep) => void;
}

interface StepDef {
  key: LauncherStep;
  ordinal: number;
  icon: typeof Stethoscope;
  titleKey: string;
  titleFallback: string;
  descKey: string;
  descFallback: string;
}

const STEPS: readonly StepDef[] = [
  {
    key: "systemCheck",
    ordinal: 1,
    icon: Stethoscope,
    titleKey: "launcherHome.steps.systemCheck.title",
    titleFallback: "系统自检",
    descKey: "launcherHome.steps.systemCheck.desc",
    descFallback: "扫描 OS / CPU / Node / Git / 网络等 17 项基础环境",
  },
  {
    key: "sandbox",
    ordinal: 2,
    icon: ShieldCheck,
    titleKey: "launcherHome.steps.sandbox.title",
    titleFallback: "沙盒",
    descKey: "launcherHome.steps.sandbox.desc",
    descFallback: "确认 L1 软拦截规则与 L2 硬红线的安全策略",
  },
  {
    key: "install",
    ordinal: 3,
    icon: Download,
    titleKey: "launcherHome.steps.install.title",
    titleFallback: "装机",
    descKey: "launcherHome.steps.install.desc",
    descFallback: "一键安装 Claude Code / Codex CLI 等 Agent 工具",
  },
  {
    key: "profile",
    ordinal: 4,
    icon: Boxes,
    titleKey: "launcherHome.steps.profile.title",
    titleFallback: "Profile",
    descKey: "launcherHome.steps.profile.desc",
    descFallback: "创建并激活每个 CLI 的 MCP / Skills / Provider 配置 Bundle",
  },
  {
    key: "launch",
    ordinal: 5,
    icon: PlayCircle,
    titleKey: "launcherHome.steps.launch.title",
    titleFallback: "启动",
    descKey: "launcherHome.steps.launch.desc",
    descFallback: "在系统终端中一键拉起选中的 CLI",
  },
] as const;

export function LauncherHomePage({ onNavigate }: LauncherHomePageProps) {
  const { t } = useTranslation();

  return (
    <div data-testid="launcher-home-page" className="flex flex-col gap-6 pb-12">
      <header className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold">
          {t("launcherHome.title", {
            defaultValue: "启动器 — 一站式配置 CC Launcher",
          })}
        </h1>
        <p className="text-sm text-muted-foreground">
          {t("launcherHome.subtitle", {
            defaultValue:
              "按 1 → 5 的顺序完成所有步骤，即可在 CC Launcher 中切换并启动 Agent CLI。",
          })}
        </p>
      </header>

      <div className="grid gap-3 sm:grid-cols-1 md:grid-cols-2 xl:grid-cols-3">
        {STEPS.map((step) => {
          const Icon = step.icon;
          return (
            <Card
              key={step.key}
              data-testid={`launcher-home-step-${step.key}`}
              className="group cursor-pointer transition-colors hover:border-primary"
              onClick={() => onNavigate(step.key)}
              role="button"
              tabIndex={0}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  onNavigate(step.key);
                }
              }}
            >
              <CardHeader className="space-y-2">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="inline-flex h-6 w-6 items-center justify-center rounded-full bg-primary/10 text-xs font-semibold text-primary">
                      {step.ordinal}
                    </span>
                    <Icon className="h-5 w-5 text-muted-foreground" />
                  </div>
                  <ChevronRight className="h-4 w-4 text-muted-foreground transition-transform group-hover:translate-x-0.5" />
                </div>
                <CardTitle className="text-lg">
                  {t(step.titleKey, { defaultValue: step.titleFallback })}
                </CardTitle>
                <CardDescription className="text-sm">
                  {t(step.descKey, { defaultValue: step.descFallback })}
                </CardDescription>
              </CardHeader>
              <CardContent className="pt-0">
                <Button
                  variant="outline"
                  size="sm"
                  className="gap-2"
                  onClick={(event) => {
                    event.stopPropagation();
                    onNavigate(step.key);
                  }}
                >
                  {t("launcherHome.openStep", {
                    defaultValue: "进入",
                  })}
                  <ArrowRight className="h-4 w-4" />
                </Button>
              </CardContent>
            </Card>
          );
        })}
      </div>

      <div className="rounded-lg border bg-muted/30 p-4 text-sm text-muted-foreground">
        <div className="flex items-center gap-2">
          <SettingsIcon className="h-4 w-4" />
          <span>
            {t("launcherHome.completion", {
              defaultValue:
                "完成所有步骤后会自动回到 CC Launcher 节点切换主界面。",
            })}
          </span>
        </div>
      </div>
    </div>
  );
}

export default LauncherHomePage;
