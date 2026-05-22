import { useTranslation } from "react-i18next";
import type { TargetCli } from "@/lib/api/contracts";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";

interface CliTabsProps {
  value: TargetCli;
  onValueChange: (value: TargetCli) => void;
}

const CLI_OPTIONS: { value: TargetCli; labelKey: string }[] = [
  { value: "claude", labelKey: "profile.cli.claude" },
  { value: "codex", labelKey: "profile.cli.codex" },
];

export function CliTabs({ value, onValueChange }: CliTabsProps) {
  const { t } = useTranslation();

  return (
    <Tabs
      value={value}
      onValueChange={(next) => onValueChange(next as TargetCli)}
    >
      <TabsList data-testid="profile-cli-tabs">
        {CLI_OPTIONS.map((opt) => (
          <TabsTrigger
            key={opt.value}
            value={opt.value}
            data-testid={`profile-cli-tab-${opt.value}`}
          >
            {t(opt.labelKey)}
          </TabsTrigger>
        ))}
      </TabsList>
    </Tabs>
  );
}
