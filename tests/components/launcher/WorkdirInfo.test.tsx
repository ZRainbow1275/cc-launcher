import { render, screen, fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { WorkdirInfo } from "@/components/launcher/WorkdirInfo";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}|${JSON.stringify(opts)}` : key,
    i18n: { language: "en" },
  }),
}));

beforeEach(() => {});

afterEach(() => {
  vi.clearAllMocks();
});

describe("WorkdirInfo", () => {
  it("renders the display path and absolute path", () => {
    render(
      <WorkdirInfo
        cwdDisplay="~/cc-launcher-projects/test-id"
        cwdAbsolute="C:\\Users\\you\\cc-launcher-projects\\test-id"
        isOpening={false}
        onOpen={() => {}}
      />,
    );

    expect(screen.getByTestId("launcher-workdir-path").textContent).toContain(
      "~/cc-launcher-projects/test-id",
    );
  });

  it("invokes onOpen when the open-folder button is clicked", () => {
    const onOpen = vi.fn();
    render(
      <WorkdirInfo
        cwdDisplay="~/cc-launcher-projects/test-id"
        cwdAbsolute="C:\\Users\\you\\cc-launcher-projects\\test-id"
        isOpening={false}
        onOpen={onOpen}
      />,
    );
    fireEvent.click(screen.getByTestId("launcher-workdir-open"));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("disables the open button while opening", () => {
    render(
      <WorkdirInfo
        cwdDisplay="~/cc-launcher-projects/test-id"
        cwdAbsolute="C:\\Users\\you\\cc-launcher-projects\\test-id"
        isOpening={true}
        onOpen={() => {}}
      />,
    );
    expect(screen.getByTestId("launcher-workdir-open")).toBeDisabled();
  });

  it("renders the reset-workdir button as disabled and inert (L2 redline)", () => {
    render(
      <WorkdirInfo
        cwdDisplay="~/cc-launcher-projects/test-id"
        cwdAbsolute="C:\\Users\\you\\cc-launcher-projects\\test-id"
        isOpening={false}
        onOpen={() => {}}
      />,
    );
    const resetBtn = screen.getByTestId("launcher-workdir-reset-disabled");
    expect(resetBtn).toBeDisabled();
    expect(resetBtn.getAttribute("aria-disabled")).toBe("true");
  });
});
