import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Profile,
  ProfileCreatePayload,
  ProfileUpdatePayload,
  type TargetCli,
} from "@/lib/api/contracts";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Checkbox } from "@/components/ui/checkbox";
import { cn } from "@/lib/utils";
import { X } from "lucide-react";

export interface ProviderOption {
  id: string;
  name: string;
  target_cli: TargetCli;
}

export interface McpOption {
  id: string;
  name: string;
  enabledForCli: TargetCli[];
}

export interface SkillOption {
  id: string;
  name: string;
}

export type ProfileEditorMode =
  | { kind: "create"; targetCli: TargetCli }
  | { kind: "edit"; profile: Profile };

interface ProfileEditorProps {
  open: boolean;
  mode: ProfileEditorMode | null;
  providerOptions: ProviderOption[];
  mcpOptions: McpOption[];
  skillOptions: SkillOption[];
  onClose: () => void;
  onSubmitCreate: (payload: ProfileCreatePayload) => Promise<void> | void;
  onSubmitUpdate: (
    id: string,
    target_cli: TargetCli,
    payload: ProfileUpdatePayload,
  ) => Promise<void> | void;
  isSubmitting?: boolean;
}

interface FormState {
  name: string;
  description: string;
  icon: string;
  icon_color: string;
  provider_id: string;
  settings_json: string;
  mcp_ids: string[];
  skill_ids: string[];
}

const DEFAULT_FORM: FormState = {
  name: "",
  description: "",
  icon: "Sparkles",
  icon_color: "#3b82f6",
  provider_id: "",
  settings_json: "{}",
  mcp_ids: [],
  skill_ids: [],
};

const NO_PROVIDER_VALUE = "__none__";

function buildInitialForm(mode: ProfileEditorMode | null): FormState {
  if (!mode) return { ...DEFAULT_FORM };
  if (mode.kind === "create") return { ...DEFAULT_FORM };
  const p = mode.profile;
  return {
    name: p.name,
    description: p.description ?? "",
    icon: p.icon ?? "Sparkles",
    icon_color: p.icon_color ?? "#3b82f6",
    provider_id: p.provider_id ?? "",
    settings_json: p.settings_json ?? "{}",
    mcp_ids: [...p.mcp_ids],
    skill_ids: [...p.skill_ids],
  };
}

function getTargetCli(mode: ProfileEditorMode | null): TargetCli | null {
  if (!mode) return null;
  return mode.kind === "create" ? mode.targetCli : mode.profile.target_cli;
}

export function ProfileEditor({
  open,
  mode,
  providerOptions,
  mcpOptions,
  skillOptions,
  onClose,
  onSubmitCreate,
  onSubmitUpdate,
  isSubmitting = false,
}: ProfileEditorProps) {
  const { t } = useTranslation();
  const targetCli = getTargetCli(mode);

  const [form, setForm] = useState<FormState>(() => buildInitialForm(mode));
  const [errors, setErrors] = useState<Record<string, string>>({});

  useEffect(() => {
    if (open) {
      setForm(buildInitialForm(mode));
      setErrors({});
    }
  }, [open, mode]);

  const filteredProviders = useMemo(
    () =>
      targetCli
        ? providerOptions.filter((p) => p.target_cli === targetCli)
        : [],
    [providerOptions, targetCli],
  );

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  function toggleMember(key: "mcp_ids" | "skill_ids", id: string) {
    setForm((prev) => {
      const exists = prev[key].includes(id);
      return {
        ...prev,
        [key]: exists ? prev[key].filter((x) => x !== id) : [...prev[key], id],
      };
    });
  }

  function validate(): boolean {
    const next: Record<string, string> = {};
    if (!form.name.trim()) {
      next.name = t("profile.editor.errors.nameRequired");
    }
    const settings = form.settings_json.trim() || "{}";
    try {
      const parsed = JSON.parse(settings);
      if (
        parsed === null ||
        Array.isArray(parsed) ||
        typeof parsed !== "object"
      ) {
        next.settings_json = t("profile.editor.errors.settingsMustBeObject");
      }
    } catch {
      next.settings_json = t("profile.editor.errors.settingsInvalidJson");
    }
    setErrors(next);
    return Object.keys(next).length === 0;
  }

  async function handleSubmit() {
    if (!mode || !targetCli) return;
    if (!validate()) return;

    const settings = form.settings_json.trim() || "{}";
    const provider_id: string | null = form.provider_id
      ? form.provider_id
      : null;

    if (mode.kind === "create") {
      const payload = ProfileCreatePayload.parse({
        target_cli: targetCli,
        name: form.name.trim(),
        description: form.description.trim() || undefined,
        icon: form.icon || undefined,
        icon_color: form.icon_color || undefined,
        provider_id,
        settings_json: settings,
        mcp_ids: form.mcp_ids,
        skill_ids: form.skill_ids,
      });
      await onSubmitCreate(payload);
    } else {
      const payload = ProfileUpdatePayload.parse({
        name: form.name.trim(),
        description: form.description.trim() || undefined,
        icon: form.icon || undefined,
        icon_color: form.icon_color || undefined,
        provider_id,
        settings_json: settings,
        mcp_ids: form.mcp_ids,
        skill_ids: form.skill_ids,
      });
      await onSubmitUpdate(mode.profile.id, mode.profile.target_cli, payload);
    }
  }

  if (!mode) return null;

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next) onClose();
      }}
    >
      <DialogContent className="max-w-2xl" data-testid="profile-editor-dialog">
        <DialogHeader>
          <DialogTitle>
            {mode.kind === "create"
              ? t("profile.editor.titleCreate")
              : t("profile.editor.titleEdit")}
          </DialogTitle>
          <DialogDescription>
            {t("profile.editor.subtitle", { cli: targetCli })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="profile-name">
              {t("profile.editor.nameLabel")}
              <span className="ml-1 text-destructive">*</span>
            </Label>
            <Input
              id="profile-name"
              value={form.name}
              onChange={(e) => update("name", e.target.value)}
              data-testid="profile-editor-name"
              placeholder={t("profile.editor.namePlaceholder")}
            />
            {errors.name && (
              <p className="text-xs text-destructive" data-testid="error-name">
                {errors.name}
              </p>
            )}
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="profile-description">
              {t("profile.editor.descriptionLabel")}
            </Label>
            <Textarea
              id="profile-description"
              value={form.description}
              onChange={(e) => update("description", e.target.value)}
              rows={2}
              data-testid="profile-editor-description"
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div className="space-y-1.5">
              <Label htmlFor="profile-icon">
                {t("profile.editor.iconLabel")}
              </Label>
              <Input
                id="profile-icon"
                value={form.icon}
                onChange={(e) => update("icon", e.target.value)}
                data-testid="profile-editor-icon"
                placeholder="Sparkles"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="profile-icon-color">
                {t("profile.editor.iconColorLabel")}
              </Label>
              <div className="flex items-center gap-2">
                <Input
                  id="profile-icon-color"
                  type="color"
                  value={form.icon_color}
                  onChange={(e) => update("icon_color", e.target.value)}
                  className="h-9 w-14 cursor-pointer p-1"
                  data-testid="profile-editor-color"
                />
                <Input
                  value={form.icon_color}
                  onChange={(e) => update("icon_color", e.target.value)}
                  className="flex-1"
                  placeholder="#3b82f6"
                />
              </div>
            </div>
          </div>

          <div className="space-y-1.5">
            <Label htmlFor="profile-provider">
              {t("profile.editor.providerLabel")}
            </Label>
            <Select
              value={form.provider_id || NO_PROVIDER_VALUE}
              onValueChange={(v) =>
                update("provider_id", v === NO_PROVIDER_VALUE ? "" : v)
              }
            >
              <SelectTrigger
                id="profile-provider"
                data-testid="profile-editor-provider"
              >
                <SelectValue
                  placeholder={t("profile.editor.providerPlaceholder")}
                />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value={NO_PROVIDER_VALUE}>
                  {t("profile.editor.providerEmpty")}
                </SelectItem>
                {filteredProviders.map((opt) => (
                  <SelectItem key={opt.id} value={opt.id}>
                    {opt.name}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <MultiSelect
            testId="profile-editor-mcp"
            label={t("profile.editor.mcpLabel")}
            emptyMessage={t("profile.editor.mcpEmpty")}
            options={mcpOptions.map((m) => ({
              id: m.id,
              label: m.name,
              hint:
                targetCli && m.enabledForCli.includes(targetCli)
                  ? t("profile.editor.mcpEnabledHint")
                  : undefined,
            }))}
            value={form.mcp_ids}
            onToggle={(id) => toggleMember("mcp_ids", id)}
          />

          <MultiSelect
            testId="profile-editor-skills"
            label={t("profile.editor.skillsLabel")}
            emptyMessage={t("profile.editor.skillsEmpty")}
            options={skillOptions.map((s) => ({ id: s.id, label: s.name }))}
            value={form.skill_ids}
            onToggle={(id) => toggleMember("skill_ids", id)}
          />

          <div className="space-y-1.5">
            <Label htmlFor="profile-settings-json">
              {t("profile.editor.settingsLabel")}
            </Label>
            <Textarea
              id="profile-settings-json"
              value={form.settings_json}
              onChange={(e) => update("settings_json", e.target.value)}
              rows={6}
              className="font-mono text-xs"
              data-testid="profile-editor-settings"
              placeholder='{ "model": "claude-sonnet-4-7" }'
            />
            {errors.settings_json && (
              <p
                className="text-xs text-destructive"
                data-testid="error-settings"
              >
                {errors.settings_json}
              </p>
            )}
          </div>
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={isSubmitting}>
            {t("common.cancel")}
          </Button>
          <Button
            onClick={() => void handleSubmit()}
            disabled={isSubmitting}
            data-testid="profile-editor-submit"
          >
            {isSubmitting ? t("common.saving") : t("common.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface MultiSelectOption {
  id: string;
  label: string;
  hint?: string;
}

interface MultiSelectProps {
  testId: string;
  label: string;
  emptyMessage: string;
  options: MultiSelectOption[];
  value: string[];
  onToggle: (id: string) => void;
}

function MultiSelect({
  testId,
  label,
  emptyMessage,
  options,
  value,
  onToggle,
}: MultiSelectProps) {
  return (
    <div className="space-y-1.5">
      <Label>{label}</Label>
      {options.length === 0 ? (
        <p
          className="text-xs text-muted-foreground"
          data-testid={`${testId}-empty`}
        >
          {emptyMessage}
        </p>
      ) : (
        <div
          className="flex flex-col gap-1.5 rounded-md border border-border-default p-2"
          data-testid={testId}
        >
          {options.map((opt) => {
            const checked = value.includes(opt.id);
            return (
              <label
                key={opt.id}
                className={cn(
                  "flex cursor-pointer items-center gap-2 rounded px-2 py-1 text-sm hover:bg-muted",
                  checked && "bg-muted",
                )}
              >
                <Checkbox
                  checked={checked}
                  onCheckedChange={() => onToggle(opt.id)}
                  data-testid={`${testId}-${opt.id}`}
                />
                <span className="flex-1">{opt.label}</span>
                {opt.hint && (
                  <Badge variant="outline" className="text-[10px]">
                    {opt.hint}
                  </Badge>
                )}
              </label>
            );
          })}
        </div>
      )}
      {value.length > 0 && (
        <div className="flex flex-wrap items-center gap-1">
          {value.map((id) => {
            const option = options.find((o) => o.id === id);
            return (
              <Badge key={id} variant="secondary" className="text-xs">
                {option?.label ?? id}
                <button
                  type="button"
                  className="ml-1 inline-flex"
                  onClick={() => onToggle(id)}
                  aria-label="remove"
                >
                  <X className="h-3 w-3" />
                </button>
              </Badge>
            );
          })}
        </div>
      )}
    </div>
  );
}
