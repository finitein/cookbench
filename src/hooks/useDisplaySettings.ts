import { useEffect, useState } from "react";

import {
  subscribeToDisplaySettings,
  type DisplaySettingsWire,
} from "../settings/display/service";

/** Live, persisted display preferences for the main Cookbench window. */
export function useDisplaySettings(): DisplaySettingsWire | null {
  const [settings, setSettings] = useState<DisplaySettingsWire | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void subscribeToDisplaySettings((next) => {
      if (active) setSettings(next);
    }).then((cleanup) => {
      unlisten = cleanup;
    }).catch(() => {
      // The persisted default is hover details off when Tauri is unavailable.
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  return settings;
}
