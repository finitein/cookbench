import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export const DISPLAY_SETTINGS_CHANGED_EVENT = "cookbench://display-settings-changed";

export type GlobalBarPlacement =
  | "topLeft"
  | "topCenter"
  | "topRight"
  | "bottomLeft"
  | "bottomCenter"
  | "bottomRight";

export type AppLocale = "system" | "en" | "zh-CN" | "ja" | "ko";
export type GlobalBarMode = "full" | "minimal";

export type DetachedBarWire = {
  stoveId: string;
};

export type DisplaySettingsWire = {
  globalBarVisible: boolean;
  globalBarPlacement: GlobalBarPlacement;
  globalBarMode: GlobalBarMode;
  macStatusStoveCount: number;
  macStatusAvailable: boolean;
  hoverDetailsEnabled: boolean;
  locale: AppLocale;
  detachedBars: DetachedBarWire[];
};

export type DisplaySettingsInput = Pick<
  DisplaySettingsWire,
  "globalBarVisible" | "globalBarPlacement" | "globalBarMode" | "macStatusStoveCount" | "hoverDetailsEnabled" | "locale"
>;
export type DisplaySettingsPatch = Partial<DisplaySettingsInput>;

export function getDisplaySettings(): Promise<DisplaySettingsWire> {
  return invoke<DisplaySettingsWire>("get_display_settings");
}

export function configureDisplaySettings(input: DisplaySettingsInput): Promise<DisplaySettingsWire> {
  return invoke<DisplaySettingsWire>("configure_display_settings", { input });
}

/** Changes only the named preference, avoiding stale cross-window snapshots. */
export function patchDisplaySettings(patch: DisplaySettingsPatch): Promise<DisplaySettingsWire> {
  return invoke<DisplaySettingsWire>("patch_display_settings", { patch });
}

/** Keeps tray, native window titles, and system notifications in step with the webview locale. */
export function syncNativeLocale(locale: Exclude<AppLocale, "system">): Promise<void> {
  return invoke<void>("sync_native_locale", { locale });
}

export async function subscribeToDisplaySettings(
  onSettings: (settings: DisplaySettingsWire) => void,
): Promise<UnlistenFn> {
  const unlisten = await listen<DisplaySettingsWire>(DISPLAY_SETTINGS_CHANGED_EVENT, (event) => {
    onSettings(event.payload);
  }).catch((): UnlistenFn => () => {});
  onSettings(await getDisplaySettings());
  return unlisten;
}

/** Closes only Cookbench's detached window; it does not touch a native session. */
export function closeDetachedBar(stoveId: string): Promise<boolean> {
  return invoke<boolean>("close_detached_bar", { stoveId });
}

export type LaunchAtLoginWire = {
  enabled: boolean;
  defaultEnabled: boolean;
};

export function getLaunchAtLogin(): Promise<LaunchAtLoginWire> {
  return invoke<LaunchAtLoginWire>("get_launch_at_login");
}

export function setLaunchAtLogin(enabled: boolean): Promise<LaunchAtLoginWire> {
  return invoke<LaunchAtLoginWire>("set_launch_at_login", { enabled });
}
