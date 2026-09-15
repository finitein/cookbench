import { afterEach, describe, expect, it } from "vitest";

import { SETTINGS_TAB_KEY, consumeSettingsTab, requestSettingsTab } from "./settingsTab";

describe("settingsTab", () => {
  afterEach(() => {
    localStorage.removeItem(SETTINGS_TAB_KEY);
  });

  it("stores and consumes a deep-link tab once", () => {
    requestSettingsTab("sources");
    expect(localStorage.getItem(SETTINGS_TAB_KEY)).toBe("sources");
    expect(consumeSettingsTab()).toBe("sources");
    expect(consumeSettingsTab()).toBeNull();
  });

  it("ignores unknown stored values", () => {
    localStorage.setItem(SETTINGS_TAB_KEY, "not-a-tab");
    expect(consumeSettingsTab()).toBeNull();
  });
});
