import { invoke } from "@tauri-apps/api/core";

import {
  ActiveProfileMap,
  OperationResult,
  Profile,
  ProfileCreatePayload,
  ProfileUpdatePayload,
  SwitchResult,
  TargetCli,
} from "../contracts";

export const profileReal = {
  async list(target_cli: TargetCli): Promise<Profile[]> {
    TargetCli.parse(target_cli);
    const raw = await invoke<unknown>("profile_list", {
      targetCli: target_cli,
    });
    return (raw as unknown[]).map((p) => Profile.parse(p));
  },

  async get(id: string, target_cli: TargetCli): Promise<Profile | null> {
    TargetCli.parse(target_cli);
    const raw = await invoke<unknown>("profile_get", {
      id,
      targetCli: target_cli,
    });
    return raw == null ? null : Profile.parse(raw);
  },

  async create(payload: ProfileCreatePayload): Promise<Profile> {
    const parsed = ProfileCreatePayload.parse(payload);
    const raw = await invoke<unknown>("profile_create", { payload: parsed });
    return Profile.parse(raw);
  },

  async update(
    id: string,
    target_cli: TargetCli,
    payload: ProfileUpdatePayload,
  ): Promise<Profile> {
    TargetCli.parse(target_cli);
    const parsed = ProfileUpdatePayload.parse(payload);
    const raw = await invoke<unknown>("profile_update", {
      id,
      targetCli: target_cli,
      payload: parsed,
    });
    return Profile.parse(raw);
  },

  async delete(id: string, target_cli: TargetCli): Promise<OperationResult> {
    TargetCli.parse(target_cli);
    const raw = await invoke<unknown>("profile_delete", {
      id,
      targetCli: target_cli,
    });
    return OperationResult.parse(raw);
  },

  async activate(id: string, target_cli: TargetCli): Promise<SwitchResult> {
    TargetCli.parse(target_cli);
    const raw = await invoke<unknown>("profile_activate", {
      id,
      targetCli: target_cli,
    });
    return SwitchResult.parse(raw);
  },
};

interface ProfileQueryResult {
  profile: unknown;
}

export const cliStateReal = {
  async get_active(target_cli: TargetCli): Promise<string | null> {
    TargetCli.parse(target_cli);
    const raw = await invoke<ProfileQueryResult>("profile_get_active", {
      targetCli: target_cli,
    });
    if (raw.profile == null) return null;
    const parsed = Profile.parse(raw.profile);
    return parsed.id;
  },

  async list_all_active(): Promise<ActiveProfileMap> {
    const raw = await invoke<unknown>("profile_list_all_active");
    return ActiveProfileMap.parse(raw);
  },
};

export type ProfileReal = typeof profileReal;
export type CliStateReal = typeof cliStateReal;
