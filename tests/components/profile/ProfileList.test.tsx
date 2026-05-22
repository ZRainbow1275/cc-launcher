import { render, screen, fireEvent } from "@testing-library/react";
import { describe, it, expect, vi } from "vitest";
import type { Profile, TargetCli } from "@/lib/api/contracts";
import { ProfileList } from "@/components/profile/ProfileList";

function makeProfile(overrides: Partial<Profile> = {}): Profile {
  const ts = Date.now();
  return {
    id: "p-1",
    target_cli: "claude" as TargetCli,
    name: "Default",
    description: "desc",
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

describe("ProfileList", () => {
  it("renders empty state when no profiles", () => {
    render(
      <ProfileList
        targetCli="claude"
        profiles={[]}
        activeProfileId={null}
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-list-empty")).toBeInTheDocument();
  });

  it("renders loading skeleton when loading", () => {
    render(
      <ProfileList
        targetCli="claude"
        profiles={[]}
        activeProfileId={null}
        isLoading={true}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-list-loading")).toBeInTheDocument();
  });

  it("shows active badge on the active profile row", () => {
    const profiles = [
      makeProfile({ id: "p-1", name: "First", sort_index: 0 }),
      makeProfile({ id: "p-2", name: "Active", sort_index: 1 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId="p-2"
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-active-badge-p-2")).toBeInTheDocument();
    expect(
      screen.queryByTestId("profile-active-badge-p-1"),
    ).not.toBeInTheDocument();
  });

  it("disables Switch button on the currently active row", () => {
    const profiles = [
      makeProfile({ id: "p-1", name: "Active", sort_index: 0 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId="p-1"
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-switch-p-1")).toBeDisabled();
  });

  it("disables Delete for builtin profiles", () => {
    const profiles = [
      makeProfile({ id: "p-b", is_builtin: true }),
      makeProfile({ id: "p-x", sort_index: 1 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId={null}
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-delete-p-b")).toBeDisabled();
  });

  it("disables Delete for the currently active profile", () => {
    const profiles = [
      makeProfile({ id: "p-1", sort_index: 0 }),
      makeProfile({ id: "p-2", sort_index: 1 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId="p-1"
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-delete-p-1")).toBeDisabled();
  });

  it("disables Delete when this is the last non-builtin profile", () => {
    const profiles = [
      makeProfile({ id: "p-b", is_builtin: true, sort_index: 0 }),
      makeProfile({ id: "p-only", sort_index: 1 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId="p-b"
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    expect(screen.getByTestId("profile-delete-p-only")).toBeDisabled();
  });

  it("invokes callbacks on action buttons", () => {
    const onSwitch = vi.fn();
    const onEdit = vi.fn();
    const onDelete = vi.fn();
    const onCreate = vi.fn();
    const profiles = [
      makeProfile({ id: "p-a", sort_index: 0 }),
      makeProfile({ id: "p-b", sort_index: 1 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId={null}
        isLoading={false}
        onCreate={onCreate}
        onEdit={onEdit}
        onDelete={onDelete}
        onSwitch={onSwitch}
      />,
    );

    fireEvent.click(screen.getByTestId("profile-create-button"));
    expect(onCreate).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByTestId("profile-switch-p-a"));
    expect(onSwitch).toHaveBeenCalledWith(
      expect.objectContaining({ id: "p-a" }),
    );

    fireEvent.click(screen.getByTestId("profile-edit-p-b"));
    expect(onEdit).toHaveBeenCalledWith(expect.objectContaining({ id: "p-b" }));

    fireEvent.click(screen.getByTestId("profile-delete-p-b"));
    expect(onDelete).toHaveBeenCalledWith(
      expect.objectContaining({ id: "p-b" }),
    );
  });

  it("sorts profiles by sort_index then created_at", () => {
    const profiles = [
      makeProfile({ id: "later", sort_index: 2, created_at: 1 }),
      makeProfile({ id: "first", sort_index: 0, created_at: 100 }),
      makeProfile({ id: "middle", sort_index: 1, created_at: 50 }),
    ];
    render(
      <ProfileList
        targetCli="claude"
        profiles={profiles}
        activeProfileId={null}
        isLoading={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onSwitch={vi.fn()}
      />,
    );
    const rows = screen.getAllByText(/Default/);
    expect(rows.length).toBeGreaterThan(0);
    const firstRowIdx = screen
      .getByTestId("profile-list")
      .innerHTML.indexOf("profile-row-first");
    const middleRowIdx = screen
      .getByTestId("profile-list")
      .innerHTML.indexOf("profile-row-middle");
    const laterRowIdx = screen
      .getByTestId("profile-list")
      .innerHTML.indexOf("profile-row-later");
    expect(firstRowIdx).toBeGreaterThan(-1);
    expect(firstRowIdx).toBeLessThan(middleRowIdx);
    expect(middleRowIdx).toBeLessThan(laterRowIdx);
  });
});
