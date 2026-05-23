import { invoke } from "@tauri-apps/api/core";

import {
  L1Rule,
  L2Redline,
  OperationResult,
  SandboxLevel,
  UnlockRequest,
} from "../contracts";

export const sandboxReal = {
  async get_l1_rules(): Promise<L1Rule[]> {
    const raw = await invoke<unknown[]>("sandbox_get_l1_rules");
    return raw.map((r) => L1Rule.parse(r));
  },

  async set_l1_rule(
    rule_id: string,
    enabled: boolean,
    _justification?: string,
  ): Promise<L1Rule> {
    const raw = await invoke<unknown>("sandbox_set_l1_rule", {
      ruleId: rule_id,
      enabled,
    });
    return L1Rule.parse(raw);
  },

  async unlock_l1_rule(rule_id: string, keyword: string): Promise<L1Rule> {
    UnlockRequest.parse({ ruleId: rule_id, keyword });
    await invoke<unknown>("sandbox_unlock_l1_rule", {
      ruleId: rule_id,
      keyword,
    });
    // Backend returns OperationResult; refetch canonical rule state.
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
    const raw = await invoke<unknown>("sandbox_set_level", { level });
    return OperationResult.parse(raw);
  },
};

export type SandboxReal = typeof sandboxReal;
