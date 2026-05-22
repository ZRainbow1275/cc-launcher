import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { ShieldCheck, Sparkles, Rocket, CheckCircle2 } from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import { onboarding as onboardingApi } from "@/lib/api/mock";
import type { OnboardingAnswers, TargetCli } from "@/lib/api/contracts";

interface OnboardingDialogProps {
  open: boolean;
  defaultCli?: TargetCli;
}

type Step = 1 | 2 | 3 | 4;

const STEP_KEYS = ["step1", "step2", "step3", "step4"] as const;

function StepIndicator({ current }: { current: Step }) {
  return (
    <div
      data-testid="onboarding-step-indicator"
      className="flex items-center justify-center gap-2 py-2"
    >
      {STEP_KEYS.map((_, idx) => {
        const n = (idx + 1) as Step;
        const active = n === current;
        const done = n < current;
        return (
          <div
            key={n}
            data-testid={`onboarding-step-dot-${n}`}
            data-state={active ? "active" : done ? "done" : "pending"}
            className={cn(
              "h-2 w-8 rounded-full transition-colors",
              active && "bg-primary",
              done && "bg-emerald-500",
              !active && !done && "bg-muted",
            )}
          />
        );
      })}
    </div>
  );
}

export function OnboardingDialog({
  open,
  defaultCli = "claude",
}: OnboardingDialogProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [step, setStep] = useState<Step>(1);
  const [accepted, setAccepted] = useState(false);
  const [selectedCli, setSelectedCli] = useState<TargetCli>(defaultCli);

  const completeMutation = useMutation({
    mutationFn: async () => {
      const answers: OnboardingAnswers = {
        locale: "zh",
        uiMode: "novice",
        enableSandbox: true,
        acceptedRedlines: true,
        preferredCli: selectedCli,
      };
      return onboardingApi.complete(answers);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ["onboarding", "state"] });
    },
  });

  const handleNext = (): void => {
    if (step < 4) {
      setStep(((step as number) + 1) as Step);
    }
  };

  const handleFinish = (): void => {
    completeMutation.mutate();
  };

  return (
    <Dialog open={open}>
      <DialogContent
        zIndex="top"
        data-testid="onboarding-dialog"
        className="max-w-xl"
        onEscapeKeyDown={(e) => e.preventDefault()}
        onPointerDownOutside={(e) => e.preventDefault()}
        onInteractOutside={(e) => e.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle data-testid="onboarding-title">
            {t("novice.onboarding.title")}
          </DialogTitle>
          <DialogDescription>
            {t(`novice.onboarding.${STEP_KEYS[step - 1]}.title`)}
          </DialogDescription>
        </DialogHeader>

        <StepIndicator current={step} />

        <div className="px-6 py-2 min-h-[140px]">
          {step === 1 && (
            <div
              data-testid="onboarding-step1"
              className="flex flex-col items-center gap-3 text-center"
            >
              <Sparkles className="h-10 w-10 text-primary" aria-hidden />
              <p className="text-sm">{t("novice.onboarding.step1.body")}</p>
            </div>
          )}

          {step === 2 && (
            <div data-testid="onboarding-step2" className="flex flex-col gap-3">
              <div className="flex items-start gap-2">
                <ShieldCheck
                  className="h-5 w-5 text-yellow-600 mt-0.5"
                  aria-hidden
                />
                <p className="text-sm text-muted-foreground">
                  {t("novice.onboarding.step2.body")}
                </p>
              </div>
              <div className="rounded-md border border-border-default bg-muted/40 p-3 text-xs text-muted-foreground max-h-32 overflow-y-auto">
                {t("novice.onboarding.step2.terms")}
              </div>
              <div className="flex items-center gap-2">
                <Checkbox
                  id="onboarding-accept"
                  checked={accepted}
                  onCheckedChange={(c) => setAccepted(c === true)}
                  data-testid="onboarding-accept-checkbox"
                />
                <Label
                  htmlFor="onboarding-accept"
                  className="text-sm cursor-pointer"
                >
                  {t("novice.onboarding.step2.checkbox")}
                </Label>
              </div>
            </div>
          )}

          {step === 3 && (
            <div data-testid="onboarding-step3" className="flex flex-col gap-3">
              <div className="flex items-start gap-2">
                <Rocket className="h-5 w-5 text-blue-600 mt-0.5" aria-hidden />
                <p className="text-sm text-muted-foreground">
                  {t("novice.onboarding.step3.body")}
                </p>
              </div>
              <div
                role="radiogroup"
                aria-label={t("novice.onboarding.step3.title")}
                className="grid grid-cols-2 gap-3"
              >
                {(["claude", "codex"] as const).map((cli) => {
                  const selected = selectedCli === cli;
                  return (
                    <button
                      key={cli}
                      type="button"
                      role="radio"
                      aria-checked={selected}
                      data-testid={`onboarding-cli-${cli}`}
                      data-selected={selected ? "true" : "false"}
                      onClick={() => setSelectedCli(cli)}
                      className={cn(
                        "rounded-lg border px-4 py-3 text-sm font-medium transition-colors",
                        selected
                          ? "border-primary bg-primary/10"
                          : "border-border-default hover:bg-muted/50",
                      )}
                    >
                      {t(`novice.onboarding.step3.${cli}`)}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {step === 4 && (
            <div
              data-testid="onboarding-step4"
              className="flex flex-col items-center gap-3 text-center"
            >
              <CheckCircle2
                className="h-10 w-10 text-emerald-500"
                aria-hidden
              />
              <p className="text-sm">{t("novice.onboarding.step4.body")}</p>
              <p className="text-xs text-muted-foreground">
                {t(`novice.onboarding.step3.${selectedCli}`)}
              </p>
            </div>
          )}
        </div>

        <DialogFooter>
          {step < 4 && (
            <Button
              onClick={handleNext}
              disabled={step === 2 && !accepted}
              data-testid={`onboarding-next-${step}`}
            >
              {t(`novice.onboarding.${STEP_KEYS[step - 1]}.next`)}
            </Button>
          )}
          {step === 4 && (
            <Button
              onClick={handleFinish}
              disabled={completeMutation.isPending}
              data-testid="onboarding-finish"
            >
              {t("novice.onboarding.step4.finish")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
