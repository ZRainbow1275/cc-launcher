import { useCallback } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { settings } from "@/lib/api/mock";
import type { UiMode } from "@/lib/api/contracts";

const UI_MODE_QUERY_KEY = ["settings", "uiMode"] as const;
const DEFAULT_MODE: UiMode = "novice";

export interface UseUiModeResult {
  mode: UiMode;
  setMode: (mode: UiMode) => Promise<void>;
  isLoading: boolean;
  isError: boolean;
  refetch: () => Promise<void>;
}

export function useUiMode(): UseUiModeResult {
  const queryClient = useQueryClient();

  const query = useQuery({
    queryKey: UI_MODE_QUERY_KEY,
    queryFn: () => settings.get_ui_mode(),
    staleTime: Infinity,
  });

  const mutation = useMutation({
    mutationFn: (next: UiMode) => settings.set_ui_mode(next),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: UI_MODE_QUERY_KEY });
    },
  });

  const setMode = useCallback(
    async (next: UiMode) => {
      await mutation.mutateAsync(next);
    },
    [mutation],
  );

  const refetch = useCallback(async () => {
    await query.refetch();
  }, [query]);

  return {
    mode: query.data ?? DEFAULT_MODE,
    setMode,
    isLoading: query.isLoading,
    isError: query.isError,
    refetch,
  };
}
