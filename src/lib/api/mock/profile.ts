import {
  ActiveProfileMap,
  OperationResult,
  Profile,
  ProfileCreatePayload,
  ProfileUpdatePayload,
  SwitchResult,
  TargetCli,
} from "../contracts";
import { errors, messages } from "./fixtures/i18n";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "profile";

/** Backend `ProfileMcpEntry` shape — see `src-tauri/src/services/profile.rs`. */
export interface McpRef {
  profile_id: string;
  target_cli: TargetCli;
  mcp_id: string;
  sort_index: number;
}

/** Backend `ProfileSkillEntry` shape — see `src-tauri/src/services/profile.rs`. */
export interface SkillRef {
  profile_id: string;
  target_cli: TargetCli;
  skill_id: string;
  sort_index: number;
}

function nowIso(): string {
  return new Date().toISOString();
}

function generateId(target: TargetCli): string {
  const rand = Math.random().toString(36).slice(2, 8);
  return `${target}-${rand}`;
}

export const profileMock = {
  async list(target_cli: TargetCli): Promise<Profile[]> {
    TargetCli.parse(target_cli);
    if (shouldFail(DOMAIN, "list")) throw errors.networkUnreachable;
    await delay();
    return getState()
      .profiles.filter((p) => p.target_cli === target_cli)
      .map((p) => Profile.parse(p));
  },

  async get(id: string, target_cli: TargetCli): Promise<Profile | null> {
    TargetCli.parse(target_cli);
    if (shouldFail(DOMAIN, "get")) throw errors.networkUnreachable;
    await delay();
    const found = getState().profiles.find(
      (p) => p.id === id && p.target_cli === target_cli,
    );
    return found ? Profile.parse(found) : null;
  },

  async create(payload: ProfileCreatePayload): Promise<Profile> {
    const parsed = ProfileCreatePayload.parse(payload);
    if (shouldFail(DOMAIN, "create")) throw errors.networkUnreachable;
    await delay();
    const ts = Date.now();
    const profile = Profile.parse({
      id: generateId(parsed.target_cli),
      target_cli: parsed.target_cli,
      name: parsed.name,
      description: parsed.description,
      icon: parsed.icon ?? "Sparkles",
      icon_color: parsed.icon_color ?? "#3b82f6",
      provider_id: parsed.provider_id ?? null,
      settings_json: parsed.settings_json ?? "{}",
      sort_index: getState().profiles.filter(
        (p) => p.target_cli === parsed.target_cli,
      ).length,
      is_builtin: false,
      mcp_ids: parsed.mcp_ids ?? [],
      skill_ids: parsed.skill_ids ?? [],
      created_at: ts,
      updated_at: ts,
    });
    getState().profiles.push(profile);
    return profile;
  },

  async update(
    id: string,
    target_cli: TargetCli,
    payload: ProfileUpdatePayload,
  ): Promise<Profile> {
    TargetCli.parse(target_cli);
    const parsed = ProfileUpdatePayload.parse(payload);
    if (shouldFail(DOMAIN, "update")) throw errors.networkUnreachable;
    await delay();
    const state = getState();
    const idx = state.profiles.findIndex(
      (p) => p.id === id && p.target_cli === target_cli,
    );
    if (idx === -1) {
      throw errors.profileNotFound;
    }
    const existing = state.profiles[idx]!;
    const updated = Profile.parse({
      ...existing,
      ...parsed,
      provider_id:
        parsed.provider_id === undefined
          ? existing.provider_id
          : parsed.provider_id,
      updated_at: Date.now(),
    });
    state.profiles[idx] = updated;
    return updated;
  },

  async delete(id: string, target_cli: TargetCli): Promise<OperationResult> {
    TargetCli.parse(target_cli);
    if (shouldFail(DOMAIN, "delete")) throw errors.networkUnreachable;
    await delay();
    const state = getState();
    const idx = state.profiles.findIndex(
      (p) => p.id === id && p.target_cli === target_cli,
    );
    if (idx === -1) {
      return OperationResult.parse({
        success: false,
        errorCode: "PROFILE_NOT_FOUND",
        message: messages.notFound,
      });
    }
    const target = state.profiles[idx]!;
    if (target.is_builtin) {
      return OperationResult.parse({
        success: false,
        errorCode: "BUILTIN_PROFILE_PROTECTED",
      });
    }
    if (state.activeProfiles[target_cli] === id) {
      return OperationResult.parse({
        success: false,
        errorCode: "PROFILE_IS_ACTIVE",
      });
    }
    state.profiles.splice(idx, 1);
    return OperationResult.parse({ success: true });
  },

  async activate(id: string, target_cli: TargetCli): Promise<SwitchResult> {
    TargetCli.parse(target_cli);
    if (shouldFail(DOMAIN, "activate")) {
      return SwitchResult.parse({
        success: false,
        profileId: id,
        targetCli: target_cli,
        switchedAt: nowIso(),
        error: errors.networkUnreachable,
      });
    }
    await delay();
    const state = getState();
    const found = state.profiles.find(
      (p) => p.id === id && p.target_cli === target_cli,
    );
    if (!found) {
      return SwitchResult.parse({
        success: false,
        profileId: id,
        targetCli: target_cli,
        switchedAt: nowIso(),
        error: errors.profileNotFound,
      });
    }
    state.activeProfiles[target_cli] = id;
    return SwitchResult.parse({
      success: true,
      profileId: id,
      targetCli: target_cli,
      backupDir: `~/.cc-switch/backups/profile-switch-${Date.now()}`,
      switchedAt: nowIso(),
    });
  },

  // D-11: backend `profile_list_mcp` parity — mock returns empty by default.
  async list_mcp(profile_id: string, target_cli: TargetCli): Promise<McpRef[]> {
    TargetCli.parse(target_cli);
    if (shouldFail(DOMAIN, "list_mcp")) throw errors.networkUnreachable;
    await delay();
    const found = getState().profiles.find(
      (p) => p.id === profile_id && p.target_cli === target_cli,
    );
    if (!found) return [];
    return found.mcp_ids.map((mcp_id, sort_index) => ({
      profile_id,
      target_cli,
      mcp_id,
      sort_index,
    }));
  },

  // D-11: backend `profile_list_skills` parity — mock returns empty by default.
  async list_skills(
    profile_id: string,
    target_cli: TargetCli,
  ): Promise<SkillRef[]> {
    TargetCli.parse(target_cli);
    if (shouldFail(DOMAIN, "list_skills")) throw errors.networkUnreachable;
    await delay();
    const found = getState().profiles.find(
      (p) => p.id === profile_id && p.target_cli === target_cli,
    );
    if (!found) return [];
    return found.skill_ids.map((skill_id, sort_index) => ({
      profile_id,
      target_cli,
      skill_id,
      sort_index,
    }));
  },
};

export const cliStateMock = {
  async get_active(target_cli: TargetCli): Promise<string | null> {
    TargetCli.parse(target_cli);
    if (shouldFail("cli_state", "get_active")) throw errors.networkUnreachable;
    await delay();
    return getState().activeProfiles[target_cli];
  },

  async list_all_active(): Promise<ActiveProfileMap> {
    if (shouldFail("cli_state", "list_all_active"))
      throw errors.networkUnreachable;
    await delay();
    return ActiveProfileMap.parse(getState().activeProfiles);
  },
};

export type ProfileMock = typeof profileMock;
export type CliStateMock = typeof cliStateMock;
