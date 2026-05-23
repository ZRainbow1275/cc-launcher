import { invoke } from "@tauri-apps/api/core";

import {
  L1Rule,
  L2Redline,
  OperationResult,
  SandboxLevel,
  UnlockRequest,
} from "../contracts";

interface BackendL1Rule {
  id: string;
  category: string;
  pattern: string;
  title_key: string;
  description_key: string;
  enabled: boolean;
  unlockable: boolean;
  unlocked_until: number | null;
  updated_at: number;
}

function epochMsToIso(ms: number | null | undefined): string | null {
  if (ms == null) return null;
  return new Date(ms).toISOString();
}

function adaptL1Rule(raw: BackendL1Rule): L1Rule {
  return L1Rule.parse({
    id: raw.id,
    category: raw.category,
    pattern: raw.pattern,
    titleKey: raw.title_key,
    descriptionKey: raw.description_key,
    enabled: raw.enabled,
    unlockable: raw.unlockable,
    unlockedUntil: epochMsToIso(raw.unlocked_until),
    updatedAt: epochMsToIso(raw.updated_at) ?? new Date().toISOString(),
  });
}

interface BackendUnlockResult {
  rule_id: string;
  success: boolean;
  unlocked_until: number | null;
}

export const sandboxReal = {
  async get_l1_rules(): Promise<L1Rule[]> {
    const raw = await invoke<BackendL1Rule[]>("sandbox_get_l1_rules");
    return raw.map(adaptL1Rule);
  },

  async set_l1_rule(
    rule_id: string,
    enabled: boolean,
    _justification?: string,
  ): Promise<L1Rule> {
    const raw = await invoke<BackendL1Rule>("sandbox_set_l1_rule", {
      ruleId: rule_id,
      enabled,
    });
    return adaptL1Rule(raw);
  },

  async unlock_l1_rule(rule_id: string, keyword: string): Promise<L1Rule> {
    UnlockRequest.parse({ ruleId: rule_id, keyword });
    const result = await invoke<BackendUnlockResult>("sandbox_unlock_l1_rule", {
      ruleId: rule_id,
      keyword,
    });
    if (!result.success) {
      throw {
        code: "INVALID_UNLOCK_KEYWORD",
        message: {
          zh: "解锁失败",
          en: "Unlock failed",
          ja: "ロック解除に失敗しました",
        },
        retryable: false,
      };
    }
    // Backend doesn't return the updated rule; re-fetch the canonical state.
    const rules = await this.get_l1_rules();
    const updated = rules.find((r) => r.id === rule_id);
    if (!updated) {
      throw {
        code: "L1_RULE_NOT_FOUND",
        message: {
          zh: "未找到规则",
          en: "Rule not found",
          ja: "ルールが見つかりません",
        },
        retryable: false,
      };
    }
    return updated;
  },

  async list_l2_redlines(): Promise<L2Redline[]> {
    const raw = await invoke<unknown[]>("sandbox_list_l2_redlines");
    return raw.map((r) => L2Redline.parse(r));
  },

  async get_sandbox_level(): Promise<SandboxLevel> {
    const raw = await invoke<string>("sandbox_get_level");
    return SandboxLevel.parse(raw);
  },

  async set_sandbox_level(level: SandboxLevel): Promise<OperationResult> {
    SandboxLevel.parse(level);
    try {
      const ok = await invoke<boolean>("sandbox_set_level", { level });
      return OperationResult.parse({ success: ok });
    } catch (err) {
      return OperationResult.parse({
        success: false,
        errorCode: typeof err === "string" ? err : "SANDBOX_SET_LEVEL_FAILED",
      });
    }
  },
};

export type SandboxReal = typeof sandboxReal;
