import {
  OnboardingAnswers,
  OnboardingState,
  OperationResult,
} from "../contracts";

// Backend command surface for onboarding does not exist yet (Phase B did not
// land `onboarding_get_state` / `onboarding_complete`). Until those land, fall
// back to a localStorage-backed implementation. See phase-c-parity.md §F.

const STORAGE_KEY = "cc-launcher:onboarding-state";

function readState(): OnboardingState {
  if (typeof window === "undefined") {
    return OnboardingState.parse({
      completed: false,
      completedAt: null,
      answers: null,
    });
  }
  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return OnboardingState.parse({
      completed: false,
      completedAt: null,
      answers: null,
    });
  }
  try {
    return OnboardingState.parse(JSON.parse(raw));
  } catch {
    return OnboardingState.parse({
      completed: false,
      completedAt: null,
      answers: null,
    });
  }
}

function writeState(state: OnboardingState): void {
  if (typeof window === "undefined") return;
  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}

export const onboardingReal = {
  async get_state(): Promise<OnboardingState> {
    return readState();
  },

  async complete(answers: OnboardingAnswers): Promise<OperationResult> {
    const parsed = OnboardingAnswers.parse(answers);
    const state: OnboardingState = OnboardingState.parse({
      completed: true,
      completedAt: new Date().toISOString(),
      answers: parsed,
    });
    writeState(state);
    return OperationResult.parse({ success: true });
  },
};

export type OnboardingReal = typeof onboardingReal;
