import { useEffect, useMemo, useState } from "react";
import { AlertTriangle, ArrowLeft, ArrowRight, ShieldOff } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";

export type DangerousConfirmLocale = "zh" | "en" | "ja";

export interface DangerousConfirmRuleInfo {
  ruleId: string;
  titleKey: string;
  descriptionKey: string;
  riskKeys: [string, string, string];
}

export interface DangerousConfirmProps {
  isOpen: boolean;
  rule: DangerousConfirmRuleInfo | null;
  locale: DangerousConfirmLocale;
  submitting?: boolean;
  onConfirm: (keyword: string) => void;
  onClose: () => void;
}

const KEYWORD_BY_LOCALE: Record<DangerousConfirmLocale, string> = {
  zh: "我已知晓",
  en: "I UNDERSTAND",
  ja: "理解しました",
};

type Step = 1 | 2 | 3;

export function DangerousConfirm({
  isOpen,
  rule,
  locale,
  submitting = false,
  onConfirm,
  onClose,
}: DangerousConfirmProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>(1);
  const [checks, setChecks] = useState<[boolean, boolean, boolean]>([
    false,
    false,
    false,
  ]);
  const [keyword, setKeyword] = useState("");

  const targetKeyword = useMemo(() => KEYWORD_BY_LOCALE[locale], [locale]);

  useEffect(() => {
    if (!isOpen) {
      setStep(1);
      setChecks([false, false, false]);
      setKeyword("");
    }
  }, [isOpen, rule?.ruleId]);

  if (!rule) {
    return (
      <Dialog
        open={isOpen}
        onOpenChange={(open) => {
          if (!open) onClose();
        }}
      >
        <DialogContent className="max-w-md" zIndex="alert" />
      </Dialog>
    );
  }

  const allChecked = checks.every(Boolean);
  const keywordMatches = keyword === targetKeyword;

  const handleClose = () => {
    if (submitting) return;
    onClose();
  };

  const handleNext = () => {
    if (step === 1) setStep(2);
    else if (step === 2 && allChecked) setStep(3);
  };

  const handleBack = () => {
    if (submitting) return;
    if (step === 2) setStep(1);
    else if (step === 3) setStep(2);
  };

  const handleSubmit = () => {
    if (!keywordMatches || submitting) return;
    onConfirm(targetKeyword);
  };

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => {
        if (!open) handleClose();
      }}
    >
      <DialogContent className="max-w-lg" zIndex="alert">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2 text-base font-semibold">
            <ShieldOff
              className="h-5 w-5 text-destructive"
              aria-hidden="true"
            />
            {t("sandbox.confirm.title")} ·{" "}
            <span className="text-muted-foreground">{t(rule.titleKey)}</span>
          </DialogTitle>
          <DialogDescription className="text-xs text-muted-foreground">
            {t("sandbox.confirm.step", { current: step })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 px-6 py-5">
          {step === 1 && (
            <div data-testid="dangerous-confirm-step-1" className="space-y-3">
              <div className="flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/5 p-3 text-sm">
                <AlertTriangle
                  className="mt-0.5 h-4 w-4 shrink-0 text-amber-600"
                  aria-hidden="true"
                />
                <div className="space-y-1">
                  <p className="font-medium">
                    {t("sandbox.confirm.step1Title")}
                  </p>
                  <p className="text-xs text-muted-foreground">
                    {t("sandbox.confirm.step1Lead")}
                  </p>
                </div>
              </div>
              <p className="rounded-md border border-border bg-muted/40 p-3 text-sm leading-relaxed">
                {t(rule.descriptionKey)}
              </p>
            </div>
          )}

          {step === 2 && (
            <div data-testid="dangerous-confirm-step-2" className="space-y-3">
              <p className="text-sm font-medium">
                {t("sandbox.confirm.step2Title")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("sandbox.confirm.step2Lead")}
              </p>
              <div className="space-y-2">
                {rule.riskKeys.map((riskKey, idx) => (
                  <label
                    key={riskKey}
                    className={cn(
                      "flex cursor-pointer items-start gap-3 rounded-md border border-border bg-card/40 p-3 text-sm hover:bg-muted/40",
                      checks[idx] && "border-amber-500/50 bg-amber-500/5",
                    )}
                  >
                    <Checkbox
                      checked={checks[idx]}
                      onCheckedChange={(value) => {
                        setChecks((prev) => {
                          const next = [...prev] as [boolean, boolean, boolean];
                          next[idx] = value === true;
                          return next;
                        });
                      }}
                      aria-label={t(riskKey)}
                      data-testid={`dangerous-confirm-risk-${idx + 1}`}
                    />
                    <span className="leading-snug">{t(riskKey)}</span>
                  </label>
                ))}
              </div>
            </div>
          )}

          {step === 3 && (
            <div data-testid="dangerous-confirm-step-3" className="space-y-3">
              <p className="text-sm font-medium">
                {t("sandbox.confirm.step3Title")}
              </p>
              <p className="text-xs text-muted-foreground">
                {t("sandbox.confirm.step3Lead")}
              </p>
              <div className="rounded-md border border-destructive/40 bg-destructive/5 p-3">
                <p className="text-xs text-muted-foreground">
                  {t("sandbox.confirm.step3Keyword")}
                </p>
                <p
                  data-testid="dangerous-confirm-keyword-display"
                  className="select-all font-mono text-base font-semibold tracking-wide text-destructive"
                >
                  {targetKeyword}
                </p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="dangerous-confirm-keyword-input">
                  {t("sandbox.confirm.step3Keyword")}
                </Label>
                <Input
                  id="dangerous-confirm-keyword-input"
                  data-testid="dangerous-confirm-keyword-input"
                  value={keyword}
                  onChange={(e) => setKeyword(e.target.value)}
                  placeholder={t("sandbox.confirm.step3Placeholder")}
                  autoComplete="off"
                  autoCorrect="off"
                  spellCheck={false}
                />
              </div>
            </div>
          )}
        </div>

        <DialogFooter className="flex flex-row items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            {step > 1 && (
              <Button
                variant="outline"
                size="sm"
                onClick={handleBack}
                disabled={submitting}
              >
                <ArrowLeft className="mr-1 h-3.5 w-3.5" />
                {t("sandbox.confirm.back")}
              </Button>
            )}
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={handleClose}
              disabled={submitting}
            >
              {t("sandbox.confirm.cancel")}
            </Button>
            {step === 1 && (
              <Button
                variant="destructive"
                size="sm"
                onClick={handleNext}
                data-testid="dangerous-confirm-step1-continue"
              >
                {t("sandbox.confirm.step1Continue")}
                <ArrowRight className="ml-1 h-3.5 w-3.5" />
              </Button>
            )}
            {step === 2 && (
              <Button
                variant="destructive"
                size="sm"
                onClick={handleNext}
                disabled={!allChecked}
                data-testid="dangerous-confirm-step2-next"
              >
                {t("sandbox.confirm.next")}
                <ArrowRight className="ml-1 h-3.5 w-3.5" />
              </Button>
            )}
            {step === 3 && (
              <Button
                variant="destructive"
                size="sm"
                onClick={handleSubmit}
                disabled={!keywordMatches || submitting}
                data-testid="dangerous-confirm-submit"
              >
                {submitting
                  ? t("sandbox.confirm.submitting")
                  : t("sandbox.confirm.step3Submit")}
              </Button>
            )}
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
