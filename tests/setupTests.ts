import "@testing-library/jest-dom";
import { afterAll, afterEach, beforeAll, vi } from "vitest";
import { cleanup } from "@testing-library/react";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import { server } from "./msw/server";
import { resetProviderState } from "./msw/state";
import "./msw/tauriMocks";
import en from "../src/i18n/locales/en.json";
import ja from "../src/i18n/locales/ja.json";
import zh from "../src/i18n/locales/zh.json";

// Globally disable the App.tsx first-launch onboarding redirect by default,
// so pre-existing tests that mount <App /> with an empty localStorage are
// unaffected. Tests that specifically want to exercise the redirect can
// opt-in by setting this flag to false in their own beforeEach.
if (typeof window !== "undefined") {
  (
    window as unknown as { __CC_DISABLE_FIRST_LAUNCH_REDIRECT__?: boolean }
  ).__CC_DISABLE_FIRST_LAUNCH_REDIRECT__ = true;
}

beforeAll(async () => {
  server.listen({ onUnhandledRequest: "warn" });
  await i18n.use(initReactI18next).init({
    lng: "zh",
    fallbackLng: "en",
    resources: {
      zh: { translation: zh },
      en: { translation: en },
      ja: { translation: ja },
    },
    interpolation: {
      escapeValue: false,
    },
  });
});

afterEach(() => {
  cleanup();
  resetProviderState();
  server.resetHandlers();
  vi.clearAllMocks();
});

afterAll(() => {
  server.close();
});
