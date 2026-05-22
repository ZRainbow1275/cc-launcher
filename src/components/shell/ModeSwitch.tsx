import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Switch } from "@/components/ui/switch";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { useUiMode } from "@/hooks/useUiMode";
import type { UiMode } from "@/lib/api/contracts";
import { extractErrorMessage } from "@/utils/errorUtils";

export interface ModeSwitchProps {
  className?: string;
}

export function ModeSwitch({ className }: ModeSwitchProps) {
  const { t } = useTranslation();
  const { mode, setMode, isLoading } = useUiMode();
  const [pendingMode, setPendingMode] = useState<UiMode | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const isExpert = mode === "expert";

  const handleToggle = (checked: boolean) => {
    const next: UiMode = checked ? "expert" : "novice";
    if (next === mode) return;
    setPendingMode(next);
  };

  const handleCancel = () => {
    if (submitting) return;
    setPendingMode(null);
  };

  const handleConfirm = async () => {
    if (!pendingMode) return;
    setSubmitting(true);
    try {
      await setMode(pendingMode);
      toast.success(
        t(`shell.modeSwitch.switchedTo.${pendingMode}`, {
          defaultValue:
            pendingMode === "expert" ? "已切换到专家模式" : "已切换到小白模式",
        }),
      );
      setPendingMode(null);
    } catch (err) {
      toast.error(
        t("shell.modeSwitch.switchFailed", {
          error: extractErrorMessage(err),
          defaultValue: "切换失败：{{error}}",
        }),
      );
    } finally {
      setSubmitting(false);
    }
  };

  const dialogOpen = pendingMode !== null;
  const confirmDescriptionKey =
    pendingMode === "expert"
      ? "shell.modeSwitch.confirmNoviceToExpert"
      : "shell.modeSwitch.confirmExpertToNovice";
  const confirmDescriptionDefault =
    pendingMode === "expert"
      ? "进入专家模式将解锁所有高级面板，可能误改影响 CLI。继续？"
      : "切回小白模式将隐藏专家面板（数据保留）。继续？";

  return (
    <div className={className}>
      <div className="flex items-center justify-between gap-4">
        <div className="flex flex-col">
          <Label htmlFor="cc-mode-switch" className="text-sm font-medium">
            {t("shell.modeSwitch.title", { defaultValue: "界面模式" })}
          </Label>
          <p className="text-xs text-muted-foreground mt-1">
            {isExpert
              ? t("shell.modeSwitch.expertHint", {
                  defaultValue: "专家模式：显示全部高级面板",
                })
              : t("shell.modeSwitch.noviceHint", {
                  defaultValue: "小白模式：仅显示核心操作",
                })}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-xs text-muted-foreground">
            {t("shell.modeSwitch.novice", { defaultValue: "小白" })}
          </span>
          <Switch
            id="cc-mode-switch"
            checked={isExpert}
            onCheckedChange={handleToggle}
            disabled={isLoading || submitting}
            aria-label={t("shell.modeSwitch.title", {
              defaultValue: "界面模式",
            })}
          />
          <span className="text-xs text-muted-foreground">
            {t("shell.modeSwitch.expert", { defaultValue: "专家" })}
          </span>
        </div>
      </div>

      <Dialog
        open={dialogOpen}
        onOpenChange={(open) => {
          if (!open) handleCancel();
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {t("shell.modeSwitch.confirmTitle", {
                defaultValue: "确认切换模式",
              })}
            </DialogTitle>
            <DialogDescription>
              {t(confirmDescriptionKey, {
                defaultValue: confirmDescriptionDefault,
              })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={handleCancel}
              disabled={submitting}
            >
              {t("common.cancel", { defaultValue: "取消" })}
            </Button>
            <Button onClick={handleConfirm} disabled={submitting}>
              {submitting
                ? t("common.loading", { defaultValue: "加载中..." })
                : t("common.confirm", { defaultValue: "确定" })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
