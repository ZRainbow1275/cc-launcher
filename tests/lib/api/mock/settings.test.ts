import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  clearFailures,
  loadScenario,
  resetState,
  setMockDelay,
  settingsMock,
} from "@/lib/api/mock";

beforeEach(() => {
  setMockDelay(0);
  clearFailures();
  loadScenario("new-user");
});

afterEach(() => {
  resetState();
});

describe("settingsMock.get_ui_mode + set_ui_mode", () => {
  it("reads default novice mode and persists changes", async () => {
    const initial = await settingsMock.get_ui_mode();
    expect(initial).toBe("novice");
    await settingsMock.set_ui_mode("expert");
    const after = await settingsMock.get_ui_mode();
    expect(after).toBe("expert");
  });

  it("rejects invalid mode", async () => {
    await expect(
      settingsMock.set_ui_mode("guru" as never),
    ).rejects.toBeTruthy();
  });
});

describe("settingsMock.get_locale + set_locale", () => {
  it("reads default zh locale and persists ja", async () => {
    const initial = await settingsMock.get_locale();
    expect(initial).toBe("zh");
    await settingsMock.set_locale("ja");
    expect(await settingsMock.get_locale()).toBe("ja");
  });
});

describe("settingsMock installer source config", () => {
  it("persists and resets custom installer sources", async () => {
    expect(await settingsMock.get_installer_source_config()).toEqual({});

    await settingsMock.set_installer_source_config({
      npmRegistry: "https://vps.example.com/npm",
      nodeDistMirror: "https://vps.example.com/node",
      gitForWindowsMirror: "https://vps.example.com/git",
    });

    expect(await settingsMock.get_installer_source_config()).toEqual({
      npmRegistry: "https://vps.example.com/npm",
      nodeDistMirror: "https://vps.example.com/node",
      gitForWindowsMirror: "https://vps.example.com/git",
    });

    await settingsMock.reset_installer_source_config();
    expect(await settingsMock.get_installer_source_config()).toEqual({});
  });

  it("rejects invalid installer source URLs", async () => {
    await expect(
      settingsMock.set_installer_source_config({
        npmRegistry: "not-a-url",
      } as never),
    ).rejects.toBeTruthy();
  });
});
