import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Loader2, RotateCcw, Save } from "lucide-react";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { settings as settingsApi } from "@/lib/api/mock";
import { InstallerSourceConfig } from "@/lib/api/contracts";

import { REGISTRY_PROBE_QUERY_KEY } from "./RegistryPicker";

export const INSTALLER_SOURCE_CONFIG_QUERY_KEY = [
  "settings",
  "installerSourceConfig",
] as const;

interface InstallerSourceSettingsProps {
  onSaved?: (config: InstallerSourceConfig) => void;
}

type Draft = Record<keyof InstallerSourceConfig, string>;

const EMPTY_DRAFT: Draft = {
  npmRegistry: "",
  nodeDistMirror: "",
  gitForWindowsMirror: "",
};

function toDraft(config: InstallerSourceConfig | undefined): Draft {
  return {
    npmRegistry: config?.npmRegistry ?? "",
    nodeDistMirror: config?.nodeDistMirror ?? "",
    gitForWindowsMirror: config?.gitForWindowsMirror ?? "",
  };
}

function normalizeDraft(draft: Draft): InstallerSourceConfig {
  const next: InstallerSourceConfig = {};
  const npmRegistry = draft.npmRegistry.trim().replace(/\/+$/, "");
  const nodeDistMirror = draft.nodeDistMirror.trim().replace(/\/+$/, "");
  const gitForWindowsMirror = draft.gitForWindowsMirror
    .trim()
    .replace(/\/+$/, "");
  if (npmRegistry) next.npmRegistry = npmRegistry;
  if (nodeDistMirror) next.nodeDistMirror = nodeDistMirror;
  if (gitForWindowsMirror) next.gitForWindowsMirror = gitForWindowsMirror;
  return InstallerSourceConfig.parse(next);
}

function parseDraft(draft: Draft): {
  config: InstallerSourceConfig | null;
  invalid: boolean;
} {
  try {
    return { config: normalizeDraft(draft), invalid: false };
  } catch {
    return { config: null, invalid: true };
  }
}

function hasAnySource(config: InstallerSourceConfig | undefined): boolean {
  return Boolean(
    config?.npmRegistry ||
      config?.nodeDistMirror ||
      config?.gitForWindowsMirror,
  );
}

export function InstallerSourceSettings({
  onSaved,
}: InstallerSourceSettingsProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState<Draft>(EMPTY_DRAFT);

  const query = useQuery({
    queryKey: INSTALLER_SOURCE_CONFIG_QUERY_KEY,
    queryFn: () => settingsApi.get_installer_source_config(),
    staleTime: 30_000,
  });

  useEffect(() => {
    setDraft(toDraft(query.data));
  }, [query.data]);

  const normalized = useMemo(() => parseDraft(draft), [draft]);

  const saveMutation = useMutation({
    mutationFn: (config: InstallerSourceConfig) =>
      settingsApi.set_installer_source_config(config),
    onSuccess: (_result, savedConfig) => {
      queryClient.setQueryData(INSTALLER_SOURCE_CONFIG_QUERY_KEY, savedConfig);
      void queryClient.invalidateQueries({
        queryKey: REGISTRY_PROBE_QUERY_KEY,
      });
      onSaved?.(savedConfig);
      toast.success(t("installer.source.saved"));
    },
    onError: () => {
      toast.error(t("installer.source.saveFailed"));
    },
  });

  const resetMutation = useMutation({
    mutationFn: () => settingsApi.reset_installer_source_config(),
    onSuccess: () => {
      setDraft(EMPTY_DRAFT);
      queryClient.setQueryData(INSTALLER_SOURCE_CONFIG_QUERY_KEY, {});
      void queryClient.invalidateQueries({
        queryKey: REGISTRY_PROBE_QUERY_KEY,
      });
      onSaved?.({});
      toast.success(t("installer.source.resetDone"));
    },
    onError: () => {
      toast.error(t("installer.source.resetFailed"));
    },
  });

  const busy =
    query.isLoading || saveMutation.isPending || resetMutation.isPending;
  const savedHasAnySource = hasAnySource(query.data);

  return (
    <Card data-testid="installer-source-settings">
      <CardContent className="pt-6 space-y-4">
        <div className="flex items-start justify-between gap-3">
          <div className="space-y-1">
            <p className="font-medium">{t("installer.source.title")}</p>
            <p className="text-xs text-muted-foreground">
              {t("installer.source.description")}
            </p>
          </div>
          {query.isFetching ? (
            <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
          ) : null}
        </div>

        <div className="grid gap-3 md:grid-cols-3">
          <label className="space-y-1 text-xs">
            <span className="font-medium">
              {t("installer.source.npmRegistry")}
            </span>
            <Input
              type="url"
              value={draft.npmRegistry}
              onChange={(e) =>
                setDraft((prev) => ({
                  ...prev,
                  npmRegistry: e.target.value,
                }))
              }
              placeholder="https://registry.example.com"
              disabled={busy}
              aria-invalid={normalized.invalid}
              data-testid="installer-source-npm-registry"
            />
          </label>

          <label className="space-y-1 text-xs">
            <span className="font-medium">
              {t("installer.source.nodeDistMirror")}
            </span>
            <Input
              type="url"
              value={draft.nodeDistMirror}
              onChange={(e) =>
                setDraft((prev) => ({
                  ...prev,
                  nodeDistMirror: e.target.value,
                }))
              }
              placeholder="https://mirror.example.com/node"
              disabled={busy}
              aria-invalid={normalized.invalid}
              data-testid="installer-source-node-dist-mirror"
            />
          </label>

          <label className="space-y-1 text-xs">
            <span className="font-medium">
              {t("installer.source.gitForWindowsMirror")}
            </span>
            <Input
              type="url"
              value={draft.gitForWindowsMirror}
              onChange={(e) =>
                setDraft((prev) => ({
                  ...prev,
                  gitForWindowsMirror: e.target.value,
                }))
              }
              placeholder="https://mirror.example.com/git-for-windows"
              disabled={busy}
              aria-invalid={normalized.invalid}
              data-testid="installer-source-git-for-windows-mirror"
            />
          </label>
        </div>

        {normalized.invalid ? (
          <p
            className="text-xs text-red-600"
            data-testid="installer-source-invalid"
          >
            {t("installer.source.invalidUrl")}
          </p>
        ) : null}

        <div className="flex flex-wrap justify-end gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={busy || !savedHasAnySource}
            onClick={() => resetMutation.mutate()}
            data-testid="installer-source-reset"
          >
            {resetMutation.isPending ? (
              <Loader2 className="h-4 w-4 mr-1 animate-spin" />
            ) : (
              <RotateCcw className="h-4 w-4 mr-1" />
            )}
            {t("installer.source.reset")}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={busy || normalized.invalid}
            onClick={() => {
              if (normalized.config) saveMutation.mutate(normalized.config);
            }}
            data-testid="installer-source-save"
          >
            {saveMutation.isPending ? (
              <Loader2 className="h-4 w-4 mr-1 animate-spin" />
            ) : (
              <Save className="h-4 w-4 mr-1" />
            )}
            {t("installer.source.save")}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
