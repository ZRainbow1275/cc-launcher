import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { StatusBar } from "@/components/novice";
import { renderWithMockIPC, teardownMockIPC } from "@/lib/api/mock";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() },
}));

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("StatusBar", () => {
  it("renders all 3 status segments (profile / sandbox / system)", async () => {
    renderWithMockIPC("fully-configured", <StatusBar />);

    await waitFor(() => {
      expect(screen.getByTestId("novice-status-bar")).toBeInTheDocument();
      expect(screen.getByTestId("novice-status-profile")).toBeInTheDocument();
      expect(screen.getByTestId("novice-status-sandbox")).toBeInTheDocument();
      expect(screen.getByTestId("system-check-summary")).toBeInTheDocument();
    });
  });

  it("shows 'no profile' label when no profile is active", async () => {
    renderWithMockIPC("new-user", <StatusBar />);

    await waitFor(() => {
      const seg = screen.getByTestId("novice-status-profile");
      expect(seg.textContent ?? "").toContain("novice.statusBar.noProfile");
    });
  });

  it("system click opens the SystemCheckSummary dashboard dialog", async () => {
    renderWithMockIPC("fully-configured", <StatusBar />);

    await waitFor(() => {
      expect(screen.getByTestId("system-check-summary")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("system-check-summary"));

    await waitFor(() => {
      expect(
        screen.getByTestId("system-check-summary-dialog"),
      ).toBeInTheDocument();
    });
  });
});
