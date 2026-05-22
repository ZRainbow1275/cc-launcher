import { Locale, OperationResult, UiMode } from "../contracts";
import { errors } from "./fixtures/i18n";
import { delay, shouldFail } from "./runtime";
import { getState } from "./scenarios";

const DOMAIN = "settings";

export const settingsMock = {
  async get_ui_mode(): Promise<UiMode> {
    if (shouldFail(DOMAIN, "get_ui_mode")) throw errors.networkUnreachable;
    await delay();
    return UiMode.parse(getState().uiMode);
  },

  async set_ui_mode(mode: UiMode): Promise<OperationResult> {
    UiMode.parse(mode);
    if (shouldFail(DOMAIN, "set_ui_mode")) throw errors.networkUnreachable;
    await delay();
    getState().uiMode = mode;
    return OperationResult.parse({ success: true });
  },

  async get_locale(): Promise<Locale> {
    if (shouldFail(DOMAIN, "get_locale")) throw errors.networkUnreachable;
    await delay();
    return Locale.parse(getState().locale);
  },

  async set_locale(locale: Locale): Promise<OperationResult> {
    Locale.parse(locale);
    if (shouldFail(DOMAIN, "set_locale")) throw errors.networkUnreachable;
    await delay();
    getState().locale = locale;
    return OperationResult.parse({ success: true });
  },
};

export type SettingsMock = typeof settingsMock;
