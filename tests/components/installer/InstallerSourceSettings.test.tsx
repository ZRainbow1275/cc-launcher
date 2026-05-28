import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import { InstallerSourceSettings } from "@/components/installer";
import { renderWithMockIPC, teardownMockIPC } from "@/lib/api/mock";
import { settingsMock } from "@/lib/api/mock";

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("InstallerSourceSettings", () => {
  it("blocks invalid URLs and saves valid installer sources", async () => {
    const user = userEvent.setup();
    renderWithMockIPC("new-user", <InstallerSourceSettings />);

    await waitFor(() => {
      expect(
        screen.getByTestId("installer-source-settings"),
      ).toBeInTheDocument();
    });

    await waitFor(() => {
      expect(
        (screen.getByTestId("installer-source-save") as HTMLButtonElement)
          .disabled,
      ).toBe(false);
    });
    const saveButton = screen.getByTestId(
      "installer-source-save",
    ) as HTMLButtonElement;

    await user.type(
      screen.getByTestId("installer-source-npm-registry"),
      "not-a-url",
    );

    await waitFor(() => {
      expect(
        screen.getByTestId("installer-source-invalid"),
      ).toBeInTheDocument();
    });
    expect(saveButton.disabled).toBe(true);

    await user.clear(screen.getByTestId("installer-source-npm-registry"));
    await user.type(
      screen.getByTestId("installer-source-npm-registry"),
      "https://vps.example.com/npm/",
    );
    await user.type(
      screen.getByTestId("installer-source-node-dist-mirror"),
      "https://vps.example.com/node",
    );
    await user.type(
      screen.getByTestId("installer-source-git-for-windows-mirror"),
      "https://vps.example.com/git",
    );
    await user.click(saveButton);

    await waitFor(async () => {
      const config = await settingsMock.get_installer_source_config();
      expect(config).toEqual({
        npmRegistry: "https://vps.example.com/npm",
        nodeDistMirror: "https://vps.example.com/node",
        gitForWindowsMirror: "https://vps.example.com/git",
      });
    });
    await waitFor(() => {
      expect(
        screen.getByTestId("installer-source-active-npm"),
      ).toHaveTextContent("https://vps.example.com/npm");
    });
  });
});
