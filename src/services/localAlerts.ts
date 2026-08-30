import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

export const LOCAL_ALERT_EVENT = "cookbench://local-alert";
export const LOCAL_ALERT_TEST_STOVE_ID = "__cookbench_test__";
export const LOCAL_ALERT_DURATION_MS = 1_200;

export type LocalAlertPayload = {
  stoveId: string;
  project: string;
  event: string;
};

export function isLocalAlertPayload(value: unknown): value is LocalAlertPayload {
  if (!value || typeof value !== "object") return false;
  const payload = value as Record<string, unknown>;
  return typeof payload.stoveId === "string"
    && typeof payload.project === "string"
    && typeof payload.event === "string";
}

/** Makes the matching Stove briefly conspicuous without changing its state. */
export function useLocalAlert(): string | null {
  const [activeStoveId, setActiveStoveId] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    let clearTimer: ReturnType<typeof setTimeout> | undefined;
    let unlisten: (() => void) | undefined;

    void listen<LocalAlertPayload>(LOCAL_ALERT_EVENT, ({ payload }) => {
      if (disposed || !isLocalAlertPayload(payload)) return;
      if (clearTimer) clearTimeout(clearTimer);
      setActiveStoveId(payload.stoveId);
      clearTimer = setTimeout(() => {
        if (!disposed) setActiveStoveId(null);
      }, LOCAL_ALERT_DURATION_MS);
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    }).catch(() => {
      // Browser fixtures and unsupported platforms have no Tauri event bridge.
    });

    return () => {
      disposed = true;
      if (clearTimer) clearTimeout(clearTimer);
      unlisten?.();
    };
  }, []);

  return activeStoveId;
}
