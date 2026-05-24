import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { settings as settingsD4 } from "@/lib/api/mock";
import type { Locale, UiMode } from "@/lib/api/contracts";
import { cn } from "@/lib/utils";

const UI_MODE_QUERY_KEY = ["settings", "uiMode"] as const;
const LOCALE_QUERY_KEY = ["settings", "locale"] as const;

interface SegmentedOption<T extends string> {
  value: T;
  label: string;
}

interface SegmentedControlProps<T extends string> {
  value: T | undefined;
  options: ReadonlyArray<SegmentedOption<T>>;
  onChange: (value: T) => void;
  disabled?: boolean;
  ariaLabel: string;
}

function SegmentedControl<T extends string>({
  value,
  options,
  onChange,
  disabled,
  ariaLabel,
}: SegmentedControlProps<T>) {
  return (
    <div
      role="radiogroup"
      aria-label={ariaLabel}
      className="inline-flex gap-1 rounded-md border border-border-default bg-background p-1"
    >
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <Button
            key={opt.value}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(opt.value)}
            disabled={disabled}
            size="sm"
            variant={active ? "default" : "ghost"}
            className={cn(
              "min-w-[96px]",
              active
                ? "shadow-sm"
                : "text-muted-foreground hover:text-foreground hover:bg-muted",
            )}
          >
            {opt.label}
          </Button>
        );
      })}
    </div>
  );
}

export function GeneralSettingsD4Section() {
  const { t, i18n } = useTranslation();
  const queryClient = useQueryClient();

  const uiModeQuery = useQuery({
    queryKey: UI_MODE_QUERY_KEY,
    queryFn: () => settingsD4.get_ui_mode(),
    staleTime: Infinity,
  });

  const localeQuery = useQuery({
    queryKey: LOCALE_QUERY_KEY,
    queryFn: () => settingsD4.get_locale(),
    staleTime: Infinity,
  });

  const uiModeMutation = useMutation({
    mutationFn: (next: UiMode) => settingsD4.set_ui_mode(next),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: UI_MODE_QUERY_KEY });
    },
    onError: () => {
      toast.error(
        t("settings.uiMode.switchFailed", { defaultValue: "切换界面模式失败" }),
      );
    },
  });

  const localeMutation = useMutation({
    mutationFn: (next: Locale) => settingsD4.set_locale(next),
    onSuccess: (_result, newLocale) => {
      void queryClient.invalidateQueries({ queryKey: LOCALE_QUERY_KEY });
      void i18n.changeLanguage(newLocale);
    },
    onError: () => {
      toast.error(
        t("settings.locale.switchFailed", { defaultValue: "切换语言失败" }),
      );
    },
  });

  const uiModeOptions: ReadonlyArray<SegmentedOption<UiMode>> = [
    {
      value: "novice",
      label: t("settings.uiMode.novice", { defaultValue: "小白模式" }),
    },
    {
      value: "expert",
      label: t("settings.uiMode.expert", { defaultValue: "专家模式" }),
    },
  ];

  const localeOptions: ReadonlyArray<SegmentedOption<Locale>> = [
    { value: "zh", label: "中文" },
    { value: "en", label: "English" },
    { value: "ja", label: "日本語" },
  ];

  return (
    <div
      data-testid="general-settings-d4-section"
      className="space-y-6 rounded-lg border border-border-default bg-background/40 p-4"
    >
      <section className="space-y-2">
        <header className="space-y-1">
          <h3 className="text-sm font-medium">
            {t("settings.uiMode.label", { defaultValue: "界面模式" })}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("settings.uiMode.hint", {
              defaultValue:
                "小白模式仅显示核心操作；专家模式解锁全部高级面板。",
            })}
          </p>
        </header>
        <SegmentedControl<UiMode>
          value={uiModeQuery.data}
          options={uiModeOptions}
          onChange={(next) => uiModeMutation.mutate(next)}
          disabled={uiModeQuery.isLoading || uiModeMutation.isPending}
          ariaLabel={t("settings.uiMode.label", { defaultValue: "界面模式" })}
        />
      </section>

      <section className="space-y-2">
        <header className="space-y-1">
          <h3 className="text-sm font-medium">
            {t("settings.locale.label", { defaultValue: "界面语言" })}
          </h3>
          <p className="text-xs text-muted-foreground">
            {t("settings.locale.hint", {
              defaultValue: "切换后立即生效，并持久化到后端配置。",
            })}
          </p>
        </header>
        <SegmentedControl<Locale>
          value={localeQuery.data}
          options={localeOptions}
          onChange={(next) => localeMutation.mutate(next)}
          disabled={localeQuery.isLoading || localeMutation.isPending}
          ariaLabel={t("settings.locale.label", { defaultValue: "界面语言" })}
        />
      </section>
    </div>
  );
}
