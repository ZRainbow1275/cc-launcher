import type { Profile, TargetCli } from "../../contracts";

let counter = 0;
function makeId(prefix: string): string {
  counter += 1;
  return `${prefix}-${counter.toString(36)}`;
}

export function resetIdCounter(): void {
  counter = 0;
}

const now = () => Date.now();

export function defaultProfile(cli: TargetCli, name = "Default"): Profile {
  const ts = now();
  return {
    id: `default-${cli}`,
    target_cli: cli,
    name,
    description:
      cli === "claude" ? "Claude Code 默认配置" : "Codex CLI 默认配置",
    icon: "Sparkles",
    icon_color: cli === "claude" ? "#c97a3a" : "#1f6feb",
    provider_id: cli === "claude" ? "anthropic-official" : "openai-official",
    settings_json: "{}",
    sort_index: 0,
    is_builtin: true,
    mcp_ids: [],
    skill_ids: [],
    created_at: ts,
    updated_at: ts,
  };
}

export function workProfile(cli: TargetCli, label: string): Profile {
  const ts = now();
  return {
    id: makeId(`${cli}-work`),
    target_cli: cli,
    name: label,
    description: `${label} 场景下的 ${cli} Profile`,
    icon: "Briefcase",
    icon_color: "#16a34a",
    provider_id: cli === "claude" ? "anthropic-official" : "openai-official",
    settings_json: JSON.stringify({
      model: cli === "claude" ? "claude-sonnet-4-7" : "gpt-5",
    }),
    sort_index: 1,
    is_builtin: false,
    mcp_ids: ["github-mcp"],
    skill_ids: ["frontend-design"],
    created_at: ts,
    updated_at: ts,
  };
}

export function expProfile(cli: TargetCli): Profile {
  const ts = now();
  return {
    id: makeId(`${cli}-exp`),
    target_cli: cli,
    name: "实验",
    description: "实验性 Profile（启用了非稳定 feature）",
    icon: "FlaskConical",
    icon_color: "#a855f7",
    provider_id: null,
    settings_json: "{}",
    sort_index: 2,
    is_builtin: false,
    mcp_ids: [],
    skill_ids: [],
    created_at: ts,
    updated_at: ts,
  };
}
