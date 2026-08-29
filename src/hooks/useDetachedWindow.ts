import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useState } from "react";
import type { StoveWire } from "../types/stove";
import { stoveForDetachedWindow } from "../services/detachedStoves";

export type DetachedWindowState = {
  isDetached: boolean;
  isSettings: boolean;
  stove: StoveWire | undefined;
};

/** Resolves a window label to a Stove without trusting query strings or paths. */
export function useDetachedWindowStove(stoves: readonly StoveWire[]): DetachedWindowState {
  const [label] = useState(() => {
    try {
      return getCurrentWebviewWindow().label;
    } catch {
      return "main";
    }
  });

  return {
    isDetached: label.startsWith("stove-"),
    isSettings: label === "settings",
    stove: stoveForDetachedWindow(stoves, label),
  };
}
