import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";

export const LOCAL_ALERT_EVENT = "cookbench://local-alert";
export const LOCAL_ALERT_TEST_STOVE_ID = "__cookbench_test__";
export const LOCAL_ALERT_DURATION_MS = 1_200;

export type LocalAlertPayload = {
  stoveId: string;
  project: string;
  event: string;
};

export type LocalAlertState = {
  activeStoveId: string | null;
  dismiss: (stoveId: string) => void;
};

export function isLocalAlertPayload(value: unknown): value is LocalAlertPayload {
  if (!value || typeof value !== "object") return false;
  const payload = value as Record<string, unknown>;
  return typeof payload.stoveId === "string"
    && typeof payload.project === "string"
    && typeof payload.event === "string";
}

/** Keeps completion alerts visible until acknowledged; other alerts stay brief. */
export function useLocalAlert(): LocalAlertState {
  const [activeStoveId, setActiveStoveId] = useState<string | null>(null);
  const clearTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const dismiss = useCallback((stoveId: string) => {
    setActiveStoveId((current) => {
      if (current !== stoveId) return current;
      if (clearTimer.current) clearTimeout(clearTimer.current);
      clearTimer.current = undefined;
      return null;
    });
  }, []);

  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;

    void listen<LocalAlertPayload>(LOCAL_ALERT_EVENT, ({ payload }) => {
      if (disposed || !isLocalAlertPayload(payload)) return;
      if (clearTimer.current) clearTimeout(clearTimer.current);
      setActiveStoveId(payload.stoveId);
      clearTimer.current = payload.event === "cooked"
        ? undefined
        : setTimeout(() => {
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
      if (clearTimer.current) clearTimeout(clearTimer.current);
      unlisten?.();
    };
  }, []);

  return { activeStoveId, dismiss };
}
