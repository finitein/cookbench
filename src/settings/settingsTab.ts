export type SettingsTabId = "general" | "sources" | "hooks" | "notifications" | "archive";

export const SETTINGS_TAB_KEY = "cookbench.settings.initialTab";

const TABS = new Set<SettingsTabId>(["general", "sources", "hooks", "notifications", "archive"]);

function isSettingsTab(value: string | null | undefined): value is SettingsTabId {
  return value != null && TABS.has(value as SettingsTabId);
}

/** Ask the next Settings window mount to open a specific tab (Bar → Settings deep-link). */
export function requestSettingsTab(tab: SettingsTabId): void {
  try {
    localStorage.setItem(SETTINGS_TAB_KEY, tab);
  } catch {
    // Browser fixtures or restricted storage must not block opening Settings.
  }
}

/** Read and clear a pending deep-link tab. Defaults to null so tray/gear stay on general. */
export function consumeSettingsTab(): SettingsTabId | null {
  try {
    const value = localStorage.getItem(SETTINGS_TAB_KEY);
    localStorage.removeItem(SETTINGS_TAB_KEY);
    return isSettingsTab(value) ? value : null;
  } catch {
    return null;
  }
}
