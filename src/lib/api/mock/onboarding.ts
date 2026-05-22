import {
  OnboardingAnswers,
  OnboardingState,
  OperationResult,
} from "../contracts";
import { errors } from "./fixtures/i18n";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "onboarding";

function nowIso(): string {
  return new Date().toISOString();
}

export const onboardingMock = {
  async get_state(): Promise<OnboardingState> {
    if (shouldFail(DOMAIN, "get_state")) throw errors.networkUnreachable;
    await delay();
    return OnboardingState.parse(getState().onboarding);
  },

  async complete(answers: OnboardingAnswers): Promise<OperationResult> {
    const parsed = OnboardingAnswers.parse(answers);
    if (shouldFail(DOMAIN, "complete")) throw errors.networkUnreachable;
    await delay();
    const state = getState();
    state.onboarding = {
      completed: true,
      completedAt: nowIso(),
      answers: parsed,
    };
    state.uiMode = parsed.uiMode;
    state.locale = parsed.locale;
    return OperationResult.parse({ success: true });
  },
};

export type OnboardingMock = typeof onboardingMock;
