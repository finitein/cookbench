import { invoke } from "@tauri-apps/api/core";

export type GlobalBarPlacement =
  | "topLeft"
  | "topCenter"
  | "topRight"
  | "bottomLeft"
  | "bottomCenter"
  | "bottomRight";

export type DetachedBarWire = {
  stoveId: string;
};

export type DisplaySettingsWire = {
  globalBarVisible: boolean;
  globalBarPlacement: GlobalBarPlacement;
  detachedBars: DetachedBarWire[];
};

export type DisplaySettingsInput = Pick<DisplaySettingsWire, "globalBarVisible" | "globalBarPlacement">;

export function getDisplaySettings(): Promise<DisplaySettingsWire> {
  return invoke<DisplaySettingsWire>("get_display_settings");
}

export function configureDisplaySettings(input: DisplaySettingsInput): Promise<DisplaySettingsWire> {
  return invoke<DisplaySettingsWire>("configure_display_settings", { input });
}

/** Closes only Cookbench's detached window; it does not touch a native session. */
export function closeDetachedBar(stoveId: string): Promise<boolean> {
  return invoke<boolean>("close_detached_bar", { stoveId });
}
