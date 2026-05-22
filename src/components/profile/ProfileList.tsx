import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ArrowRightLeft, Pencil, Plus, Sparkles, Trash2 } from "lucide-react";
import type { Profile, TargetCli } from "@/lib/api/contracts";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

interface ProfileListProps {
  targetCli: TargetCli;
  profiles: Profile[];
  activeProfileId: string | null;
  isLoading: boolean;
  onCreate: () => void;
  onEdit: (profile: Profile) => void;
  onDelete: (profile: Profile) => void;
  onSwitch: (profile: Profile) => void;
}

export function ProfileList({
  targetCli,
  profiles,
  activeProfileId,
  isLoading,
  onCreate,
  onEdit,
  onDelete,
  onSwitch,
}: ProfileListProps) {
  const { t } = useTranslation();

  const sorted = useMemo(() => {
    return [...profiles].sort((a, b) => {
      const ai = a.sort_index ?? Number.MAX_SAFE_INTEGER;
      const bi = b.sort_index ?? Number.MAX_SAFE_INTEGER;
      if (ai !== bi) return ai - bi;
      return a.created_at - b.created_at;
    });
  }, [profiles]);

  const nonBuiltinCount = useMemo(
    () => profiles.filter((p) => !p.is_builtin).length,
    [profiles],
  );

  const isLastDeletable = (profile: Profile): boolean => {
    if (profile.is_builtin) return false;
    return nonBuiltinCount <= 1;
  };

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex items-center justify-between gap-2">
        <h2 className="text-base font-semibold">
          {t("profile.list.title", { cli: targetCli })}
        </h2>
        <Button
          size="sm"
          onClick={onCreate}
          data-testid="profile-create-button"
        >
          <Plus className="mr-1 h-4 w-4" />
          {t("profile.list.createButton")}
        </Button>
      </div>

      {isLoading ? (
        <div className="space-y-3" data-testid="profile-list-loading">
          {[0, 1, 2].map((i) => (
            <div
              key={i}
              className="h-24 w-full rounded-lg border border-dashed border-muted-foreground/40 bg-muted/40"
            />
          ))}
        </div>
      ) : sorted.length === 0 ? (
        <div
          className="rounded-lg border border-dashed border-border-default px-6 py-10 text-center text-sm text-muted-foreground"
          data-testid="profile-list-empty"
        >
          {t("profile.list.empty")}
        </div>
      ) : (
        <ul className="space-y-3" data-testid="profile-list">
          {sorted.map((profile) => {
            const isActive = profile.id === activeProfileId;
            const lastDeletable = isLastDeletable(profile);
            return (
              <li
                key={profile.id}
                data-testid={`profile-row-${profile.id}`}
                className={cn(
                  "flex flex-col gap-3 rounded-lg border bg-card p-4 transition-colors",
                  isActive
                    ? "border-emerald-500/60 ring-1 ring-emerald-500/30"
                    : "border-border-default",
                )}
              >
                <div className="flex items-start gap-3">
                  <div
                    className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md"
                    style={{
                      backgroundColor: `${profile.icon_color ?? "#3b82f6"}20`,
                      color: profile.icon_color ?? "#3b82f6",
                    }}
                    aria-hidden
                  >
                    <Sparkles className="h-5 w-5" />
                  </div>

                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span
                        className="font-medium text-foreground"
                        data-testid={`profile-name-${profile.id}`}
                      >
                        {profile.name}
                      </span>
                      {isActive && (
                        <span
                          className="inline-flex items-center gap-1.5"
                          data-testid={`profile-active-badge-${profile.id}`}
                        >
                          <span className="h-2 w-2 rounded-full bg-emerald-500" />
                          <Badge
                            variant="secondary"
                            className="bg-emerald-500/15 text-emerald-700 dark:text-emerald-300"
                          >
                            {t("profile.list.active")}
                          </Badge>
                        </span>
                      )}
                      {profile.is_builtin && (
                        <Badge variant="outline">
                          {t("profile.list.builtin")}
                        </Badge>
                      )}
                    </div>
                    {profile.description && (
                      <p className="mt-1 text-xs text-muted-foreground">
                        {profile.description}
                      </p>
                    )}

                    <div className="mt-2 flex flex-wrap items-center gap-2 text-xs">
                      <Badge variant="outline">
                        {t("profile.list.providerBadge", {
                          name:
                            profile.provider_id ??
                            t("profile.list.providerEmpty"),
                        })}
                      </Badge>
                      <Badge variant="outline">
                        {t("profile.list.mcpBadge", {
                          count: profile.mcp_ids.length,
                        })}
                      </Badge>
                      <Badge variant="outline">
                        {t("profile.list.skillBadge", {
                          count: profile.skill_ids.length,
                        })}
                      </Badge>
                    </div>
                  </div>
                </div>

                <div className="flex flex-wrap items-center justify-end gap-2">
                  <Button
                    size="sm"
                    variant="secondary"
                    onClick={() => onSwitch(profile)}
                    disabled={isActive}
                    data-testid={`profile-switch-${profile.id}`}
                  >
                    <ArrowRightLeft className="mr-1 h-4 w-4" />
                    {isActive
                      ? t("profile.list.switchToCurrent")
                      : t("profile.list.switchTo")}
                  </Button>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={() => onEdit(profile)}
                    data-testid={`profile-edit-${profile.id}`}
                  >
                    <Pencil className="mr-1 h-4 w-4" />
                    {t("common.edit")}
                  </Button>

                  <DeleteButton
                    profile={profile}
                    disabled={profile.is_builtin || isActive || lastDeletable}
                    reasonKey={
                      profile.is_builtin
                        ? "profile.list.cannotDeleteBuiltin"
                        : isActive
                          ? "profile.list.cannotDeleteActive"
                          : lastDeletable
                            ? "profile.list.cannotDeleteLast"
                            : undefined
                    }
                    onDelete={() => onDelete(profile)}
                  />
                </div>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

interface DeleteButtonProps {
  profile: Profile;
  disabled: boolean;
  reasonKey: string | undefined;
  onDelete: () => void;
}

function DeleteButton({
  profile,
  disabled,
  reasonKey,
  onDelete,
}: DeleteButtonProps) {
  const { t } = useTranslation();
  const button = (
    <Button
      size="sm"
      variant="outline"
      onClick={() => {
        if (!disabled) onDelete();
      }}
      disabled={disabled}
      data-testid={`profile-delete-${profile.id}`}
      className="text-destructive hover:text-destructive"
    >
      <Trash2 className="mr-1 h-4 w-4" />
      {t("common.delete")}
    </Button>
  );

  if (!disabled || !reasonKey) {
    return button;
  }

  return (
    <TooltipProvider delayDuration={150}>
      <Tooltip>
        <TooltipTrigger asChild>
          <span tabIndex={0} aria-disabled className="inline-flex">
            {button}
          </span>
        </TooltipTrigger>
        <TooltipContent>{t(reasonKey)}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}
