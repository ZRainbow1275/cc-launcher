import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { L2RedlineCard } from "@/components/sandbox/L2RedlineCard";
import type { L2Redline } from "@/lib/api/contracts";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) => key,
  }),
}));

const sample: L2Redline = {
  id: "disk_wipe.rm_root",
  category: "DiskWipe",
  pattern: "(?i)^rm\\s+-rf\\s+/\\s*$",
  descriptionKey: "sandbox.l2.disk_wipe.rm_root",
  matchType: "regex",
};

describe("L2RedlineCard", () => {
  it("renders the redline pattern, badge, and description in read-only form", () => {
    render(<L2RedlineCard redline={sample} os="windows" />);

    expect(screen.getByTestId("l2-redline-disk_wipe.rm_root")).toBeTruthy();
    expect(screen.getByText("sandbox.l2.permanentLockBadge")).toBeTruthy();
    expect(screen.getByText("sandbox.l2.matchType.regex")).toBeTruthy();
    expect(screen.getByText(sample.pattern)).toBeTruthy();
    expect(screen.getByText("sandbox.l2.disk_wipe.rm_root")).toBeTruthy();
  });

  it("does not render any form controls (read-only)", () => {
    const { container } = render(
      <L2RedlineCard redline={sample} os="windows" />,
    );

    expect(container.querySelectorAll("button").length).toBe(0);
    expect(container.querySelectorAll("input").length).toBe(0);
    expect(container.querySelectorAll('[role="switch"]').length).toBe(0);
    expect(container.querySelectorAll('[role="checkbox"]').length).toBe(0);
  });

  it("points to the Windows doc on Windows OS", () => {
    render(<L2RedlineCard redline={sample} os="windows" />);
    const link = screen.getByRole("link");
    const href = link.getAttribute("href") ?? "";
    expect(href).toContain("windows");
    expect(href).not.toContain("macos");
  });

  it("points to the macOS doc on macOS OS", () => {
    render(<L2RedlineCard redline={sample} os="macos" />);
    const link = screen.getByRole("link");
    const href = link.getAttribute("href") ?? "";
    expect(href).toContain("macos");
    expect(href).not.toContain("windows");
  });
});
