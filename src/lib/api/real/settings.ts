import { invoke } from "@tauri-apps/api/core";

import {
  InstallerSourceConfig,
  Locale,
  OperationResult,
  UiMode,
} from "../contracts";

export const settingsReal = {
  async get_ui_mode(): Promise<UiMode> {
    const raw = await invoke<string>("settings_get_ui_mode");
    return UiMode.parse(raw);
  },

  async set_ui_mode(mode: UiMode): Promise<OperationResult> {
    const parsed = UiMode.parse(mode);
    const raw = await invoke<unknown>("settings_set_ui_mode", { mode: parsed });
    return OperationResult.parse(raw);
  },

  async get_locale(): Promise<Locale> {
    const raw = await invoke<string>("settings_get_locale");
    return Locale.parse(raw);
  },

  async set_locale(locale: Locale): Promise<OperationResult> {
    const parsed = Locale.parse(locale);
    const raw = await invoke<unknown>("settings_set_locale", {
      locale: parsed,
    });
    return OperationResult.parse(raw);
  },

  async get_installer_source_config(): Promise<InstallerSourceConfig> {
    const raw = await invoke<unknown>("settings_get_installer_source_config");
    return InstallerSourceConfig.parse(raw);
  },

  async set_installer_source_config(
    config: InstallerSourceConfig,
  ): Promise<OperationResult> {
    const parsed = InstallerSourceConfig.parse(config);
    const raw = await invoke<unknown>("settings_set_installer_source_config", {
      config: parsed,
    });
    return OperationResult.parse(raw);
  },

  async reset_installer_source_config(): Promise<OperationResult> {
    const raw = await invoke<unknown>("settings_reset_installer_source_config");
    return OperationResult.parse(raw);
  },
};

export type SettingsReal = typeof settingsReal;
