import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import type { Profile, SwitchResult } from "@/lib/api/contracts";
import { SwitchPreview } from "@/components/profile/SwitchPreview";

function makeProfile(overrides: Partial<Profile> = {}): Profile {
  const ts = Date.now();
  return {
    id: "p-1",
    target_cli: "claude",
    name: "Default",
    description: undefined,
    icon: "Sparkles",
    icon_color: "#3b82f6",
    provider_id: "anthropic-official",
    settings_json: "{}",
    sort_index: 0,
    is_builtin: false,
    mcp_ids: [],
    skill_ids: [],
    created_at: ts,
    updated_at: ts,
    ...overrides,
  };
}

describe("SwitchPreview", () => {
  it("renders diff sections when MCP / Skills / settings / provider change", () => {
    const current = makeProfile({
      id: "curr",
      mcp_ids: ["a", "b"],
      skill_ids: ["x"],
      provider_id: "anthropic-official",
      settings_json: JSON.stringify({ model: "claude-old" }),
    });
    const next = makeProfile({
      id: "next",
      mcp_ids: ["b", "c"],
      skill_ids: ["x", "y"],
      provider_id: "other-provider",
      settings_json: JSON.stringify({ model: "claude-new" }),
    });

    render(
      <SwitchPreview
        open
        current={current}
        next={next}
        isSwitching={false}
        failure={null}
        onConfirm={vi.fn()}
        onRetry={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByTestId("diff-mcp-add-c")).toBeInTheDocument();
    expect(screen.getByTestId("diff-mcp-remove-a")).toBeInTheDocument();
    expect(screen.getByTestId("diff-skills-add-y")).toBeInTheDocument();
    expect(screen.getByTestId("diff-provider")).toBeInTheDocument();
    expect(screen.getByTestId("diff-settings")).toBeInTheDocument();
  });

  it("shows unchanged hints when profiles match", () => {
    const profile = makeProfile({
      mcp_ids: ["a"],
      skill_ids: ["x"],
      provider_id: "p-same",
      settings_json: "{}",
    });

    render(
      <SwitchPreview
        open
        current={profile}
        next={profile}
        isSwitching={false}
        failure={null}
        onConfirm={vi.fn()}
        onRetry={vi.fn()}
        onClose={vi.fn()}
      />,
    );

    expect(screen.getByTestId("diff-mcp-unchanged")).toBeInTheDocument();
    expect(screen.getByTestId("diff-skills-unchanged")).toBeInTheDocument();
    expect(screen.getByTestId("diff-provider-unchanged")).toBeInTheDocument();
    expect(screen.getByTestId("diff-settings-unchanged")).toBeInTheDocument();
  });

  it("invokes onConfirm on confirm button click", () => {
    const onConfirm = vi.fn();
    render(
      <SwitchPreview
        open
        current={makeProfile({ id: "a" })}
        next={makeProfile({ id: "b" })}
        isSwitching={false}
        failure={null}
        onConfirm={onConfirm}
        onRetry={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByTestId("switch-confirm"));
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it("renders failure block with backup dir and retry button on failure", () => {
    const failure: SwitchResult = {
      success: false,
      profileId: "b",
      targetCli: "claude",
      switchedAt: new Date().toISOString(),
      backupDir: "/home/u/.cc-switch/backups/profile-switch-1234",
      error: {
        code: "ATOMIC_WRITE_FAILED",
        message: {
          zh: "原子写入失败",
          en: "Atomic write failed",
          ja: "アトミック書き込みに失敗",
        },
        retryable: true,
      },
    };
    const onRetry = vi.fn();
    render(
      <SwitchPreview
        open
        current={makeProfile({ id: "a" })}
        next={makeProfile({ id: "b" })}
        isSwitching={false}
        failure={failure}
        onConfirm={vi.fn()}
        onRetry={onRetry}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByTestId("switch-failure")).toBeInTheDocument();
    expect(screen.getByTestId("switch-backup-dir")).toHaveTextContent(
      "/home/u/.cc-switch/backups/profile-switch-1234",
    );
    fireEvent.click(screen.getByTestId("switch-retry"));
    expect(onRetry).toHaveBeenCalledOnce();
  });

  it("disables confirm button while switching", () => {
    render(
      <SwitchPreview
        open
        current={makeProfile({ id: "a" })}
        next={makeProfile({ id: "b" })}
        isSwitching={true}
        failure={null}
        onConfirm={vi.fn()}
        onRetry={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.getByTestId("switch-confirm")).toBeDisabled();
  });
});
