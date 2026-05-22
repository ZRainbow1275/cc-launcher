import { screen, waitFor, fireEvent, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProfileManager } from "@/components/profile/ProfileManager";
import {
  cliStateMock,
  profileMock,
  renderWithMockIPC,
  teardownMockIPC,
} from "@/lib/api/mock";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

beforeEach(() => {
  // each test sets up its own scenario via renderWithMockIPC
});

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("ProfileManager", () => {
  it("loads and displays profiles from the active CLI", async () => {
    renderWithMockIPC("fully-configured", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list")).toBeInTheDocument();
    });

    const claudeProfiles = await profileMock.list("claude");
    for (const p of claudeProfiles) {
      expect(screen.getByTestId(`profile-row-${p.id}`)).toBeInTheDocument();
    }

    const active = await cliStateMock.get_active("claude");
    expect(active).not.toBeNull();
    expect(
      screen.getByTestId(`profile-active-badge-${active!}`),
    ).toBeInTheDocument();
  });

  it("switches between CLI tabs and shows the matching profile set", async () => {
    const user = userEvent.setup();
    renderWithMockIPC("fully-configured", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list")).toBeInTheDocument();
    });

    await user.click(screen.getByTestId("profile-cli-tab-codex"));

    await waitFor(async () => {
      const codexProfiles = await profileMock.list("codex");
      for (const p of codexProfiles) {
        expect(screen.getByTestId(`profile-row-${p.id}`)).toBeInTheDocument();
      }
    });
  });

  it("creates a profile via the editor and persists it", async () => {
    renderWithMockIPC("all-installed-no-profile", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list-empty")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("profile-create-button"));

    await waitFor(() => {
      expect(screen.getByTestId("profile-editor-dialog")).toBeInTheDocument();
    });

    const nameInput = screen.getByTestId(
      "profile-editor-name",
    ) as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: "Integration Profile" } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("profile-editor-submit"));
    });

    await waitFor(async () => {
      const list = await profileMock.list("claude");
      expect(list.find((p) => p.name === "Integration Profile")).toBeTruthy();
    });
  });

  it("blocks submission when name is empty", async () => {
    renderWithMockIPC("all-installed-no-profile", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list-empty")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("profile-create-button"));

    await waitFor(() => {
      expect(screen.getByTestId("profile-editor-dialog")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("profile-editor-submit"));

    await waitFor(() => {
      expect(screen.getByTestId("error-name")).toBeInTheDocument();
    });

    const list = await profileMock.list("claude");
    expect(list.length).toBe(0);
  });

  it("opens the switch preview and activates the chosen profile on confirm", async () => {
    renderWithMockIPC("fully-configured", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list")).toBeInTheDocument();
    });

    const claudeProfiles = await profileMock.list("claude");
    const currentActive = await cliStateMock.get_active("claude");
    const target = claudeProfiles.find((p) => p.id !== currentActive);
    expect(target).toBeDefined();

    fireEvent.click(screen.getByTestId(`profile-switch-${target!.id}`));

    await waitFor(() => {
      expect(
        screen.getByTestId("profile-switch-preview-dialog"),
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("switch-confirm"));
    });

    await waitFor(async () => {
      const active = await cliStateMock.get_active("claude");
      expect(active).toBe(target!.id);
    });
  });

  it("shows failure block with backup dir on switch failure", async () => {
    renderWithMockIPC("fully-configured", <ProfileManager />, {
      failures: [{ domain: "profile", command: "activate" }],
    });

    await waitFor(() => {
      expect(screen.getByTestId("profile-list")).toBeInTheDocument();
    });

    const claudeProfiles = await profileMock.list("claude");
    const currentActive = await cliStateMock.get_active("claude");
    const target = claudeProfiles.find((p) => p.id !== currentActive);
    expect(target).toBeDefined();

    fireEvent.click(screen.getByTestId(`profile-switch-${target!.id}`));

    await waitFor(() => {
      expect(
        screen.getByTestId("profile-switch-preview-dialog"),
      ).toBeInTheDocument();
    });

    await act(async () => {
      fireEvent.click(screen.getByTestId("switch-confirm"));
    });

    await waitFor(() => {
      expect(screen.getByTestId("switch-failure")).toBeInTheDocument();
      expect(screen.getByTestId("switch-retry")).toBeInTheDocument();
    });
  });

  it("rejects deleting a builtin profile (UI prevents the call)", async () => {
    renderWithMockIPC("fully-configured", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list")).toBeInTheDocument();
    });

    const claudeProfiles = await profileMock.list("claude");
    const builtin = claudeProfiles.find((p) => p.is_builtin);
    expect(builtin).toBeDefined();

    const deleteBtn = screen.getByTestId(`profile-delete-${builtin!.id}`);
    expect(deleteBtn).toBeDisabled();
  });

  it("opens the editor in edit mode with prefilled name", async () => {
    renderWithMockIPC("fully-configured", <ProfileManager />);

    await waitFor(() => {
      expect(screen.getByTestId("profile-list")).toBeInTheDocument();
    });

    const claudeProfiles = await profileMock.list("claude");
    const editable = claudeProfiles.find((p) => !p.is_builtin);
    expect(editable).toBeDefined();

    fireEvent.click(screen.getByTestId(`profile-edit-${editable!.id}`));

    await waitFor(() => {
      expect(screen.getByTestId("profile-editor-dialog")).toBeInTheDocument();
    });

    const nameInput = screen.getByTestId(
      "profile-editor-name",
    ) as HTMLInputElement;
    expect(nameInput.value).toBe(editable!.name);

    fireEvent.change(nameInput, { target: { value: "Renamed Profile" } });

    await act(async () => {
      fireEvent.click(screen.getByTestId("profile-editor-submit"));
    });

    await waitFor(async () => {
      const list = await profileMock.list("claude");
      const updated = list.find((p) => p.id === editable!.id);
      expect(updated?.name).toBe("Renamed Profile");
    });
  });
});
