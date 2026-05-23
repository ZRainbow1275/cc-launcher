import { Locale, OperationResult, UiMode } from "../contracts";

// Backend command surface for UI-mode / locale settings does not exist yet
// (Phase B did not land `settings_get_ui_mode` etc.). Until those land, fall
// back to a localStorage-backed implementation. See phase-c-parity.md §F.

const UI_MODE_KEY = "cc-launcher:ui-mode";
const LOCALE_KEY = "cc-launcher:locale";

function readUiMode(): UiMode {
  if (typeof window === "undefined") return UiMode.parse("novice");
  const raw = window.localStorage.getItem(UI_MODE_KEY);
  if (!raw) return UiMode.parse("novice");
  try {
    return UiMode.parse(raw);
  } catch {
    return UiMode.parse("novice");
  }
}

function readLocale(): Locale {
  if (typeof window === "undefined") return Locale.parse("zh");
  const raw = window.localStorage.getItem(LOCALE_KEY);
  if (!raw) return Locale.parse("zh");
  try {
    return Locale.parse(raw);
  } catch {
    return Locale.parse("zh");
  }
}

export const settingsReal = {
  async get_ui_mode(): Promise<UiMode> {
    return readUiMode();
  },

  async set_ui_mode(mode: UiMode): Promise<OperationResult> {
    const parsed = UiMode.parse(mode);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(UI_MODE_KEY, parsed);
    }
    return OperationResult.parse({ success: true });
  },

  async get_locale(): Promise<Locale> {
    return readLocale();
  },

  async set_locale(locale: Locale): Promise<OperationResult> {
    const parsed = Locale.parse(locale);
    if (typeof window !== "undefined") {
      window.localStorage.setItem(LOCALE_KEY, parsed);
    }
    return OperationResult.parse({ success: true });
  },
};

export type SettingsReal = typeof settingsReal;
