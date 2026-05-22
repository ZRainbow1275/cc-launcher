import { fireEvent, screen, waitFor, act } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { OnboardingDialog } from "@/components/novice";
import {
  onboardingMock,
  renderWithMockIPC,
  teardownMockIPC,
} from "@/lib/api/mock";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), info: vi.fn() },
}));

afterEach(() => {
  teardownMockIPC();
  vi.clearAllMocks();
});

describe("OnboardingDialog", () => {
  it("4-step flow advances correctly", async () => {
    renderWithMockIPC("new-user", <OnboardingDialog open />);

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-step1")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("onboarding-next-1"));

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-step2")).toBeInTheDocument();
    });

    // Step 2 next is disabled until checkbox checked
    expect(screen.getByTestId("onboarding-next-2")).toBeDisabled();

    fireEvent.click(screen.getByTestId("onboarding-accept-checkbox"));

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-next-2")).not.toBeDisabled();
    });

    fireEvent.click(screen.getByTestId("onboarding-next-2"));

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-step3")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("onboarding-next-3"));

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-step4")).toBeInTheDocument();
      expect(screen.getByTestId("onboarding-finish")).toBeInTheDocument();
    });
  });

  it("finish with default selection calls onboarding.complete with acceptedRedlines=true + preferredCli=claude", async () => {
    const spy = vi.spyOn(onboardingMock, "complete");

    renderWithMockIPC("new-user", <OnboardingDialog open />);

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-step1")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("onboarding-next-1"));
    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step2")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("onboarding-accept-checkbox"));
    await waitFor(() =>
      expect(screen.getByTestId("onboarding-next-2")).not.toBeDisabled(),
    );

    fireEvent.click(screen.getByTestId("onboarding-next-2"));
    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step3")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("onboarding-next-3"));
    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step4")).toBeInTheDocument(),
    );

    await act(async () => {
      fireEvent.click(screen.getByTestId("onboarding-finish"));
    });

    await waitFor(() => {
      expect(spy).toHaveBeenCalledTimes(1);
    });

    const arg = spy.mock.calls[0]![0];
    expect(arg.preferredCli).toBe("claude");
    expect(arg.acceptedRedlines).toBe(true);

    spy.mockRestore();
  });

  it("codex selection passes through to onboarding.complete", async () => {
    const spy = vi.spyOn(onboardingMock, "complete");

    renderWithMockIPC("new-user", <OnboardingDialog open />);

    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step1")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByTestId("onboarding-next-1"));

    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step2")).toBeInTheDocument(),
    );
    fireEvent.click(screen.getByTestId("onboarding-accept-checkbox"));
    await waitFor(() =>
      expect(screen.getByTestId("onboarding-next-2")).not.toBeDisabled(),
    );
    fireEvent.click(screen.getByTestId("onboarding-next-2"));

    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step3")).toBeInTheDocument(),
    );

    fireEvent.click(screen.getByTestId("onboarding-cli-codex"));

    await waitFor(() => {
      expect(screen.getByTestId("onboarding-cli-codex")).toHaveAttribute(
        "data-selected",
        "true",
      );
    });

    fireEvent.click(screen.getByTestId("onboarding-next-3"));

    await waitFor(() =>
      expect(screen.getByTestId("onboarding-step4")).toBeInTheDocument(),
    );

    await act(async () => {
      fireEvent.click(screen.getByTestId("onboarding-finish"));
    });

    await waitFor(() => {
      expect(spy).toHaveBeenCalledTimes(1);
    });
    expect(spy.mock.calls[0]![0].preferredCli).toBe("codex");

    spy.mockRestore();
  });
});
