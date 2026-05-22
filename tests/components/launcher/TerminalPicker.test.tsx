import { render, screen, fireEvent } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TerminalPicker } from "@/components/launcher/TerminalPicker";
import {
  macTerminals,
  windowsTerminals,
  noTerminals,
} from "@/lib/api/mock/fixtures/terminals";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, opts?: Record<string, unknown>) =>
      opts ? `${key}|${JSON.stringify(opts)}` : key,
    i18n: { language: "en" },
  }),
}));

beforeEach(() => {
  // noop
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("TerminalPicker (Windows)", () => {
  it("renders wt.exe + cmd.exe + powershell, with wt recommended", () => {
    const onSelect = vi.fn();
    render(
      <TerminalPicker
        terminals={windowsTerminals()}
        selectedId={null}
        isLoading={false}
        onSelect={onSelect}
      />,
    );

    expect(
      screen.getByTestId("launcher-terminal-option-wt"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("launcher-terminal-option-cmd"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("launcher-terminal-option-powershell"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("launcher-terminal-recommended-wt"),
    ).toBeInTheDocument();
  });

  it("auto-selects the recommended terminal on mount when none selected", () => {
    const onSelect = vi.fn();
    render(
      <TerminalPicker
        terminals={windowsTerminals()}
        selectedId={null}
        isLoading={false}
        onSelect={onSelect}
      />,
    );
    expect(onSelect).toHaveBeenCalledWith("wt");
  });

  it("invokes onSelect when clicking an available terminal", () => {
    const onSelect = vi.fn();
    render(
      <TerminalPicker
        terminals={windowsTerminals()}
        selectedId={"wt"}
        isLoading={false}
        onSelect={onSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("launcher-terminal-option-cmd"));
    expect(onSelect).toHaveBeenCalledWith("cmd");
  });
});

describe("TerminalPicker (macOS)", () => {
  it("renders Terminal.app + iTerm2 with Terminal.app recommended and iTerm2 greyed", () => {
    const onSelect = vi.fn();
    render(
      <TerminalPicker
        terminals={macTerminals()}
        selectedId={null}
        isLoading={false}
        onSelect={onSelect}
      />,
    );

    expect(
      screen.getByTestId("launcher-terminal-option-terminal-app"),
    ).toBeInTheDocument();
    expect(
      screen.getByTestId("launcher-terminal-recommended-terminal-app"),
    ).toBeInTheDocument();

    // iTerm2 is unavailable (installed=false) in the fixture
    const iterm = screen.getByTestId("launcher-terminal-option-iterm2");
    expect(iterm).toBeDisabled();
    expect(
      screen.getByTestId("launcher-terminal-unavailable-iterm2"),
    ).toBeInTheDocument();
  });

  it("does not call onSelect when clicking an unavailable terminal", () => {
    const onSelect = vi.fn();
    render(
      <TerminalPicker
        terminals={macTerminals()}
        selectedId={"terminal-app"}
        isLoading={false}
        onSelect={onSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("launcher-terminal-option-iterm2"));
    expect(onSelect).not.toHaveBeenCalledWith("iterm2");
  });
});

describe("TerminalPicker (empty)", () => {
  it("shows empty state when no terminals detected", () => {
    const onSelect = vi.fn();
    render(
      <TerminalPicker
        terminals={noTerminals()}
        selectedId={null}
        isLoading={false}
        onSelect={onSelect}
      />,
    );
    expect(screen.getByTestId("launcher-terminal-empty")).toBeInTheDocument();
  });
});
