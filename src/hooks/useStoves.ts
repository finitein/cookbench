import { useEffect, useState } from "react";
import { subscribeToStoves, type StoveTransport } from "../services/stoves";
import type { StoveSnapshot } from "../types/stove";

const EMPTY_SNAPSHOT: StoveSnapshot = { revision: 0, stoves: [] };

/** React view of the desktop store, including snapshot recovery after gaps. */
export function useStoves(transport?: StoveTransport): StoveSnapshot {
  const [snapshot, setSnapshot] = useState<StoveSnapshot>(EMPTY_SNAPSHOT);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    void subscribeToStoves((next) => {
      if (active) setSnapshot(next);
    }, transport).then((cleanup) => {
      unlisten = cleanup;
    }).catch(() => {
      // The empty view is a usable startup fallback while Tauri is unavailable.
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [transport]);

  return snapshot;
}
