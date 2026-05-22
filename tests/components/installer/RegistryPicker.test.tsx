import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { RegistryPicker } from "@/components/installer/RegistryPicker";
import { renderWithMockIPC, teardownMockIPC } from "@/lib/api/mock";

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("RegistryPicker", () => {
  it("displays 4 registries and marks the fastest after probing", async () => {
    const onSelect = vi.fn();
    renderWithMockIPC(
      "fully-configured",
      <RegistryPicker selectedUrl={null} onSelect={onSelect} />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("registry-row-npmjs")).toBeInTheDocument();
    });

    expect(screen.getByTestId("registry-row-npmmirror")).toBeInTheDocument();
    expect(screen.getByTestId("registry-row-tencent")).toBeInTheDocument();
    expect(screen.getByTestId("registry-row-huawei")).toBeInTheDocument();

    // npmmirror has lowest mock latency (180ms)
    await waitFor(() => {
      expect(
        screen.getByTestId("registry-row-npmmirror-fastest"),
      ).toBeInTheDocument();
    });
  });

  it("invokes onSelect when user clicks a different registry row", async () => {
    const onSelect = vi.fn();
    renderWithMockIPC(
      "fully-configured",
      <RegistryPicker selectedUrl={null} onSelect={onSelect} />,
    );

    await waitFor(() => {
      expect(screen.getByTestId("registry-row-huawei")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("registry-row-huawei"));
    expect(onSelect).toHaveBeenCalledWith(
      "https://mirrors.huaweicloud.com/repository/npm",
    );
  });

  it("renders an all-failed error block with retry under network-failure scenario", async () => {
    const onSelect = vi.fn();
    const onAllFailed = vi.fn();
    renderWithMockIPC(
      "network-failure",
      <RegistryPicker
        selectedUrl={null}
        onSelect={onSelect}
        onAllFailed={onAllFailed}
      />,
    );

    await waitFor(() => {
      expect(
        screen.getByTestId("registry-picker-all-failed"),
      ).toBeInTheDocument();
    });

    expect(screen.getByTestId("registry-picker-retry")).toBeInTheDocument();
    expect(
      screen.getByTestId("registry-picker-manual-toggle"),
    ).toBeInTheDocument();
    expect(onAllFailed).toHaveBeenCalled();
  });

  it("auto-propagates the fastest registry via onSelect when no selection is given", async () => {
    const onSelect = vi.fn();
    renderWithMockIPC(
      "fully-configured",
      <RegistryPicker selectedUrl={null} onSelect={onSelect} />,
    );

    await waitFor(() => {
      expect(
        screen.getByTestId("registry-row-npmmirror-fastest"),
      ).toBeInTheDocument();
    });

    expect(onSelect).toHaveBeenCalledWith("https://registry.npmmirror.com");
  });

  it("exposes a manual input after toggling manual mode in all-failed branch", async () => {
    const onSelect = vi.fn();
    renderWithMockIPC(
      "network-failure",
      <RegistryPicker selectedUrl={null} onSelect={onSelect} />,
    );

    await waitFor(() => {
      expect(
        screen.getByTestId("registry-picker-all-failed"),
      ).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("registry-picker-manual-toggle"));

    await waitFor(() => {
      expect(
        screen.getByTestId("registry-picker-manual-input"),
      ).toBeInTheDocument();
    });

    const input = screen.getByTestId(
      "registry-picker-manual-input",
    ) as HTMLInputElement;
    fireEvent.change(input, {
      target: { value: "https://my-registry.example.com" },
    });
    fireEvent.click(screen.getByTestId("registry-picker-manual-submit"));
    expect(onSelect).toHaveBeenCalledWith("https://my-registry.example.com");
  });
});
