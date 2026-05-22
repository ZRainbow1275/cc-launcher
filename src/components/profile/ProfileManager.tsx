import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  Profile,
  ProfileCreatePayload,
  ProfileUpdatePayload,
  SwitchResult,
  type TargetCli,
} from "@/lib/api/contracts";
import { cliState, profile as profileApi } from "@/lib/api/mock";
import { CliTabs } from "./CliTabs";
import { ProfileList } from "./ProfileList";
import {
  ProfileEditor,
  type ProfileEditorMode,
  type McpOption,
  type ProviderOption,
  type SkillOption,
} from "./ProfileEditor";
import { SwitchPreview } from "./SwitchPreview";
import { ConfirmDialog } from "@/components/ConfirmDialog";

interface ProfileManagerProps {
  initialCli?: TargetCli;
  providerOptions?: ProviderOption[];
  mcpOptions?: McpOption[];
  skillOptions?: SkillOption[];
}

const DEFAULT_PROVIDERS: ProviderOption[] = [
  {
    id: "anthropic-official",
    name: "Anthropic Official",
    target_cli: "claude",
  },
  { id: "openai-official", name: "OpenAI Official", target_cli: "codex" },
];

const DEFAULT_MCPS: McpOption[] = [
  {
    id: "github-mcp",
    name: "GitHub MCP",
    enabledForCli: ["claude", "codex"],
  },
  { id: "filesystem-mcp", name: "Filesystem MCP", enabledForCli: ["claude"] },
];

const DEFAULT_SKILLS: SkillOption[] = [
  { id: "frontend-design", name: "Frontend Design" },
  { id: "backend-architecture", name: "Backend Architecture" },
];

export function ProfileManager({
  initialCli = "claude",
  providerOptions = DEFAULT_PROVIDERS,
  mcpOptions = DEFAULT_MCPS,
  skillOptions = DEFAULT_SKILLS,
}: ProfileManagerProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [cli, setCli] = useState<TargetCli>(initialCli);

  const [editorMode, setEditorMode] = useState<ProfileEditorMode | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);

  const [pendingSwitch, setPendingSwitch] = useState<Profile | null>(null);
  const [switchFailure, setSwitchFailure] = useState<SwitchResult | null>(null);

  const [pendingDelete, setPendingDelete] = useState<Profile | null>(null);

  const profilesQuery = useQuery({
    queryKey: ["profile", "list", cli],
    queryFn: () => profileApi.list(cli),
  });

  const activeQuery = useQuery({
    queryKey: ["cli_state", "active", cli],
    queryFn: () => cliState.get_active(cli),
  });

  const invalidateAll = useCallback(
    (target: TargetCli) => {
      queryClient.invalidateQueries({ queryKey: ["profile", "list", target] });
      queryClient.invalidateQueries({
        queryKey: ["cli_state", "active", target],
      });
    },
    [queryClient],
  );

  const createMutation = useMutation({
    mutationFn: (payload: ProfileCreatePayload) => profileApi.create(payload),
    onSuccess: (created) => {
      toast.success(t("profile.toast.createSuccess", { name: created.name }));
      invalidateAll(created.target_cli);
      setEditorOpen(false);
      setEditorMode(null);
    },
    onError: (err: unknown) => {
      toast.error(extractErrorMessage(err));
    },
  });

  const updateMutation = useMutation({
    mutationFn: ({
      id,
      target_cli,
      payload,
    }: {
      id: string;
      target_cli: TargetCli;
      payload: ProfileUpdatePayload;
    }) => profileApi.update(id, target_cli, payload),
    onSuccess: (updated) => {
      toast.success(t("profile.toast.updateSuccess", { name: updated.name }));
      invalidateAll(updated.target_cli);
      setEditorOpen(false);
      setEditorMode(null);
    },
    onError: (err: unknown) => {
      toast.error(extractErrorMessage(err));
    },
  });

  const switchMutation = useMutation({
    mutationFn: ({ id, target_cli }: { id: string; target_cli: TargetCli }) =>
      profileApi.activate(id, target_cli),
    onSuccess: (result, vars) => {
      if (result.success) {
        toast.success(t("profile.toast.switchSuccess"));
        setPendingSwitch(null);
        setSwitchFailure(null);
        invalidateAll(vars.target_cli);
      } else {
        setSwitchFailure(result);
      }
    },
    onError: (err: unknown) => {
      const message = extractErrorMessage(err);
      toast.error(message);
      setSwitchFailure({
        success: false,
        profileId: pendingSwitch?.id ?? "",
        targetCli: cli,
        switchedAt: new Date().toISOString(),
        error: {
          code: "UNKNOWN",
          message: { zh: message, en: message, ja: message },
          retryable: true,
        },
      });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: ({ id, target_cli }: { id: string; target_cli: TargetCli }) =>
      profileApi.delete(id, target_cli),
    onSuccess: (res, vars) => {
      if (res.success) {
        toast.success(t("profile.toast.deleteSuccess"));
        invalidateAll(vars.target_cli);
      } else {
        toast.error(
          t(`profile.toast.deleteFailed.${res.errorCode}`, {
            defaultValue: res.errorCode ?? t("common.error"),
          }),
        );
      }
      setPendingDelete(null);
    },
    onError: (err: unknown) => {
      toast.error(extractErrorMessage(err));
      setPendingDelete(null);
    },
  });

  const currentActiveProfile = useMemo<Profile | null>(() => {
    if (!activeQuery.data || !profilesQuery.data) return null;
    return profilesQuery.data.find((p) => p.id === activeQuery.data) ?? null;
  }, [activeQuery.data, profilesQuery.data]);

  const handleCreate = useCallback(() => {
    setEditorMode({ kind: "create", targetCli: cli });
    setEditorOpen(true);
  }, [cli]);

  const handleEdit = useCallback((p: Profile) => {
    setEditorMode({ kind: "edit", profile: p });
    setEditorOpen(true);
  }, []);

  const handleDelete = useCallback((p: Profile) => {
    setPendingDelete(p);
  }, []);

  const handleSwitch = useCallback((p: Profile) => {
    setPendingSwitch(p);
    setSwitchFailure(null);
  }, []);

  const confirmSwitch = useCallback(() => {
    if (!pendingSwitch) return;
    switchMutation.mutate({
      id: pendingSwitch.id,
      target_cli: pendingSwitch.target_cli,
    });
  }, [pendingSwitch, switchMutation]);

  const confirmDelete = useCallback(() => {
    if (!pendingDelete) return;
    deleteMutation.mutate({
      id: pendingDelete.id,
      target_cli: pendingDelete.target_cli,
    });
  }, [pendingDelete, deleteMutation]);

  return (
    <div className="flex h-full flex-col gap-4 p-4">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-lg font-semibold">{t("profile.manager.title")}</h1>
        <CliTabs value={cli} onValueChange={setCli} />
      </div>

      <div className="flex-1 overflow-auto">
        <ProfileList
          targetCli={cli}
          profiles={profilesQuery.data ?? []}
          activeProfileId={activeQuery.data ?? null}
          isLoading={profilesQuery.isLoading}
          onCreate={handleCreate}
          onEdit={handleEdit}
          onDelete={handleDelete}
          onSwitch={handleSwitch}
        />
      </div>

      <ProfileEditor
        open={editorOpen}
        mode={editorMode}
        providerOptions={providerOptions}
        mcpOptions={mcpOptions}
        skillOptions={skillOptions}
        isSubmitting={createMutation.isPending || updateMutation.isPending}
        onClose={() => {
          setEditorOpen(false);
          setEditorMode(null);
        }}
        onSubmitCreate={async (payload) => {
          await createMutation.mutateAsync(payload);
        }}
        onSubmitUpdate={async (id, target_cli, payload) => {
          await updateMutation.mutateAsync({ id, target_cli, payload });
        }}
      />

      <SwitchPreview
        open={pendingSwitch !== null}
        current={currentActiveProfile}
        next={pendingSwitch}
        isSwitching={switchMutation.isPending}
        failure={switchFailure}
        onConfirm={confirmSwitch}
        onRetry={confirmSwitch}
        onClose={() => {
          setPendingSwitch(null);
          setSwitchFailure(null);
        }}
      />

      <ConfirmDialog
        isOpen={pendingDelete !== null}
        variant="destructive"
        title={t("profile.delete.title")}
        message={t("profile.delete.message", {
          name: pendingDelete?.name ?? "",
        })}
        confirmText={t("common.delete")}
        cancelText={t("common.cancel")}
        onConfirm={confirmDelete}
        onCancel={() => setPendingDelete(null)}
      />
    </div>
  );
}

function extractErrorMessage(err: unknown): string {
  if (err && typeof err === "object") {
    const e = err as {
      code?: string;
      message?: { zh?: string } | string;
    };
    if (typeof e.message === "string") return e.message;
    if (e.message && typeof e.message === "object" && e.message.zh)
      return e.message.zh;
    if (e.code) return e.code;
  }
  return String(err);
}
