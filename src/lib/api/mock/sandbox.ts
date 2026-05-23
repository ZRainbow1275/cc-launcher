import {
  L1Rule,
  L2Redline,
  OperationResult,
  SandboxLevel,
  UnlockRequest,
} from "../contracts";
import { errors } from "./fixtures/i18n";
import { l2Redlines } from "./fixtures/sandbox";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "sandbox";

/** D-11: response shape for `sandbox_check_redline_match`. */
export interface RedlineMatchResult {
  matched: false;
  ruleId?: undefined;
  pattern?: undefined;
}

/** D-11: response shape for `sandbox_get_audit_log`. */
export interface AuditEntry {
  id: string;
  timestamp: string;
  category: string;
  command: string;
  cwd: string;
  outcome: "blocked" | "warned" | "allowed";
}

function nowIso(): string {
  return new Date().toISOString();
}

const UNLOCK_DURATION_MS = 24 * 60 * 60 * 1000;
const VALID_KEYWORDS = new Set([
  "I-UNDERSTAND-THE-RISK",
  "UNLOCK",
  "I UNDERSTAND",
  "我已知晓",
  "理解しました",
]);

export const sandboxMock = {
  async get_l1_rules(): Promise<L1Rule[]> {
    if (shouldFail(DOMAIN, "get_l1_rules")) throw errors.networkUnreachable;
    await delay();
    return getState().l1Rules.map((r) => L1Rule.parse(r));
  },

  async set_l1_rule(
    rule_id: string,
    enabled: boolean,
    _justification?: string,
  ): Promise<L1Rule> {
    if (shouldFail(DOMAIN, "set_l1_rule")) throw errors.networkUnreachable;
    await delay();
    const state = getState();
    const idx = state.l1Rules.findIndex((r) => r.id === rule_id);
    if (idx === -1) {
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
    const rule = state.l1Rules[idx]!;
    if (!rule.unlockable && !enabled) {
      throw {
        code: "L1_RULE_NOT_UNLOCKABLE",
        message: {
          zh: "该规则不可关闭",
          en: "Rule cannot be disabled",
          ja: "このルールは無効化できません",
        },
        retryable: false,
      };
    }
    const updated = L1Rule.parse({
      ...rule,
      enabled,
      updatedAt: nowIso(),
    });
    state.l1Rules[idx] = updated;
    return updated;
  },

  async unlock_l1_rule(rule_id: string, keyword: string): Promise<L1Rule> {
    UnlockRequest.parse({ ruleId: rule_id, keyword });
    if (shouldFail(DOMAIN, "unlock_l1_rule")) throw errors.networkUnreachable;
    await delay();
    if (!VALID_KEYWORDS.has(keyword)) {
      throw {
        code: "INVALID_UNLOCK_KEYWORD",
        message: {
          zh: "解锁关键词错误",
          en: "Invalid unlock keyword",
          ja: "解除キーワードが正しくありません",
        },
        retryable: false,
      };
    }
    const state = getState();
    const idx = state.l1Rules.findIndex((r) => r.id === rule_id);
    if (idx === -1) {
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
    const rule = state.l1Rules[idx]!;
    if (!rule.unlockable) {
      throw {
        code: "L1_RULE_NOT_UNLOCKABLE",
        message: {
          zh: "该规则永不可解锁",
          en: "Rule is permanently locked",
          ja: "このルールは解除できません",
        },
        retryable: false,
      };
    }
    const until = new Date(Date.now() + UNLOCK_DURATION_MS).toISOString();
    const updated = L1Rule.parse({
      ...rule,
      enabled: false,
      unlockedUntil: until,
      updatedAt: nowIso(),
    });
    state.l1Rules[idx] = updated;
    return updated;
  },

  async list_l2_redlines(): Promise<L2Redline[]> {
    if (shouldFail(DOMAIN, "list_l2_redlines")) throw errors.networkUnreachable;
    await delay();
    return l2Redlines().map((r) => L2Redline.parse(r));
  },

  async get_sandbox_level(): Promise<SandboxLevel> {
    if (shouldFail(DOMAIN, "get_sandbox_level"))
      throw errors.networkUnreachable;
    await delay();
    return SandboxLevel.parse(getState().sandboxLevel);
  },

  async set_sandbox_level(level: SandboxLevel): Promise<OperationResult> {
    SandboxLevel.parse(level);
    if (shouldFail(DOMAIN, "set_sandbox_level"))
      throw errors.networkUnreachable;
    await delay();
    getState().sandboxLevel = level;
    return OperationResult.parse({ success: true });
  },

  // D-11: backend `sandbox_check_redline_match` parity. The real backend runs
  // the actual regex/substring match; the mock always reports "no match" so
  // vitest scenarios stay deterministic.
  async check_redline_match(
    _arg: string,
    _cwd: string,
  ): Promise<RedlineMatchResult> {
    if (shouldFail(DOMAIN, "check_redline_match"))
      throw errors.networkUnreachable;
    await delay();
    return { matched: false };
  },

  // D-11: backend `sandbox_get_audit_log` parity. Mock returns an empty log
  // by default; scenarios can extend `getState().sandboxAuditLog` later.
  async get_audit_log(limit?: number): Promise<AuditEntry[]> {
    if (shouldFail(DOMAIN, "get_audit_log")) throw errors.networkUnreachable;
    await delay();
    const seeded = (getState() as unknown as { sandboxAuditLog?: AuditEntry[] })
      .sandboxAuditLog;
    const entries: AuditEntry[] = Array.isArray(seeded) ? seeded : [];
    if (typeof limit === "number" && limit >= 0) {
      return entries.slice(0, limit);
    }
    return entries;
  },
};

export type SandboxMock = typeof sandboxMock;
