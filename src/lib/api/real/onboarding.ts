import { invoke } from "@tauri-apps/api/core";

import {
  OnboardingAnswers,
  OnboardingState,
  OperationResult,
} from "../contracts";

export const onboardingReal = {
  async get_state(): Promise<OnboardingState> {
    const raw = await invoke<unknown>("onboarding_get_state");
    return OnboardingState.parse(raw);
  },

  async complete(answers: OnboardingAnswers): Promise<OperationResult> {
    const parsed = OnboardingAnswers.parse(answers);
    const raw = await invoke<unknown>("onboarding_complete", {
      answers: parsed,
    });
    return OperationResult.parse(raw);
  },
};

export type OnboardingReal = typeof onboardingReal;
