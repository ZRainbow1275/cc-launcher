import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertTriangle,
  CheckCircle2,
  Loader2,
  RefreshCw,
  Wifi,
  WifiOff,
  XCircle,
} from "lucide-react";

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent } from "@/components/ui/card";
import { installer } from "@/lib/api/mock";
import type { RegistryPickResult, RegistryProbe } from "@/lib/api/contracts";

export const REGISTRY_PROBE_QUERY_KEY = ["installer", "probe_registries"];

interface RegistryPickerProps {
  selectedUrl: string | null;
  onSelect: (url: string) => void;
  onAllFailed?: () => void;
}

function rowStatusLabel(
  probe: RegistryProbe,
  t: (k: string) => string,
): string {
  if (probe.ok) return t("installer.step4.registryPicker.rowOk");
  if (probe.error === "timeout") {
    return t("installer.step4.registryPicker.rowTimeout");
  }
  return t("installer.step4.registryPicker.rowFailed");
}

function sortCandidates(items: RegistryProbe[]): RegistryProbe[] {
  return [...items].sort((a, b) => {
    if (a.ok !== b.ok) return a.ok ? -1 : 1;
    return a.latencyMs - b.latencyMs;
  });
}

export function RegistryPicker({
  selectedUrl,
  onSelect,
  onAllFailed,
}: RegistryPickerProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [manualUrl, setManualUrl] = useState("");
  const [manualMode, setManualMode] = useState(false);

  const query = useQuery<RegistryPickResult, Error>({
    queryKey: REGISTRY_PROBE_QUERY_KEY,
    queryFn: () => installer.smart_pick_registry(),
    retry: false,
    staleTime: 0,
  });

  const sorted = useMemo(
    () => (query.data ? sortCandidates(query.data.candidates) : []),
    [query.data],
  );

  const fastestUrl = useMemo(() => {
    const winner = sorted.find((c) => c.ok);
    return winner?.url ?? null;
  }, [sorted]);

  const isAllFailed = !query.isLoading && !query.data && Boolean(query.error);

  const effectiveSelected =
    selectedUrl ?? (query.data ? query.data.chosen : null);

  const handleReprobe = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: REGISTRY_PROBE_QUERY_KEY });
  }, [queryClient]);

  useEffect(() => {
    if (isAllFailed) onAllFailed?.();
  }, [isAllFailed, onAllFailed]);

  useEffect(() => {
    if (!selectedUrl && query.data?.chosen) {
      onSelect(query.data.chosen);
    }
  }, [selectedUrl, query.data?.chosen, onSelect]);

  if (isAllFailed) {
    return (
      <Card
        data-testid="registry-picker-all-failed"
        className="border-red-200 dark:border-red-900 bg-red-50 dark:bg-red-950"
      >
        <CardContent className="pt-6 space-y-3">
          <Alert variant="destructive">
            <AlertTriangle className="h-4 w-4" />
            <AlertTitle>
              {t("installer.step4.registryPicker.allFailed")}
            </AlertTitle>
            <AlertDescription className="text-red-900 dark:text-red-100" />
          </Alert>
          <div className="flex flex-wrap gap-2">
            <Button
              type="button"
              variant="default"
              size="sm"
              onClick={handleReprobe}
              data-testid="registry-picker-retry"
            >
              <RefreshCw className="h-4 w-4 mr-1" />
              {t("installer.step4.registryPicker.retry")}
            </Button>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => setManualMode((v) => !v)}
              data-testid="registry-picker-manual-toggle"
            >
              {t("installer.step4.registryPicker.manual")}
            </Button>
          </div>
          {manualMode ? (
            <div className="flex gap-2">
              <Input
                value={manualUrl}
                onChange={(e) => setManualUrl(e.target.value)}
                placeholder={t(
                  "installer.step4.registryPicker.manualPlaceholder",
                )}
                data-testid="registry-picker-manual-input"
              />
              <Button
                type="button"
                size="sm"
                disabled={!manualUrl}
                onClick={() => onSelect(manualUrl)}
                data-testid="registry-picker-manual-submit"
              >
                {t("installer.step4.registryPicker.select")}
              </Button>
            </div>
          ) : null}
        </CardContent>
      </Card>
    );
  }

  return (
    <Card data-testid="registry-picker">
      <CardContent className="pt-6 space-y-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <p className="font-medium">
              {t("installer.step4.registryPicker.title")}
            </p>
            <p className="text-xs text-muted-foreground">
              {t("installer.step4.registryPicker.subtitle")}
            </p>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={query.isFetching}
            onClick={handleReprobe}
            data-testid="registry-picker-reprobe"
          >
            {query.isFetching ? (
              <Loader2 className="h-4 w-4 mr-1 animate-spin" />
            ) : (
              <RefreshCw className="h-4 w-4 mr-1" />
            )}
            {query.isFetching
              ? t("installer.step4.registryPicker.probing")
              : t("installer.step4.registryPicker.probe")}
          </Button>
        </div>

        <div className="space-y-2" data-testid="registry-picker-rows">
          {sorted.map((probe) => {
            const isFastest = probe.ok && probe.url === fastestUrl;
            const isSelected = effectiveSelected === probe.url;
            const Icon = probe.ok ? CheckCircle2 : XCircle;
            return (
              <button
                type="button"
                key={probe.name}
                data-testid={`registry-row-${probe.name}`}
                data-ok={probe.ok ? "true" : "false"}
                data-selected={isSelected ? "true" : "false"}
                onClick={() => probe.ok && onSelect(probe.url)}
                disabled={!probe.ok}
                className={`w-full flex items-center justify-between gap-3 rounded-md border px-3 py-2 text-left transition-colors ${
                  isSelected
                    ? "border-blue-500 bg-blue-50 dark:bg-blue-950"
                    : "border-border-default hover:bg-muted/40"
                } ${!probe.ok ? "opacity-60 cursor-not-allowed" : ""}`}
              >
                <span className="flex items-center gap-2">
                  <Icon
                    className={`h-4 w-4 ${
                      probe.ok ? "text-emerald-600" : "text-red-600"
                    }`}
                  />
                  <span className="font-medium text-sm">{probe.name}</span>
                  <span className="text-xs text-muted-foreground">
                    {probe.url}
                  </span>
                </span>
                <span className="flex items-center gap-2 text-xs">
                  {isFastest ? (
                    <span
                      data-testid={`registry-row-${probe.name}-fastest`}
                      className="px-1.5 py-0.5 rounded bg-emerald-100 text-emerald-900 dark:bg-emerald-900/40 dark:text-emerald-100"
                    >
                      {t("installer.step4.registryPicker.fastest")}
                    </span>
                  ) : null}
                  <span className="font-mono">
                    {probe.ok
                      ? t("installer.step4.registryPicker.latency", {
                          ms: probe.latencyMs,
                        })
                      : rowStatusLabel(probe, t)}
                  </span>
                  {probe.ok ? (
                    <Wifi className="h-3.5 w-3.5 text-emerald-600" />
                  ) : (
                    <WifiOff className="h-3.5 w-3.5 text-red-600" />
                  )}
                </span>
              </button>
            );
          })}
        </div>

        {sorted.some((c) => c.name === "tencent" && !c.ok) ? (
          <p className="text-xs text-muted-foreground">
            {t("installer.step4.registryPicker.note")}
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}
