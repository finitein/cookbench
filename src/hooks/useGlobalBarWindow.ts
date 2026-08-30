import { useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { listen } from "@tauri-apps/api/event";

import {
  attachGlobalBarDragHandle,
  applyGlobalBarWidth,
  globalBarWindowSize,
  prepareNativeGlobalBarDocument,
  recordGlobalBarPosition,
  resizeGlobalBarWindow,
} from "../services/globalBarWindow";
import { createPositionPersistence } from "../services/detachedStoves";
import { getDisplaySettings, type GlobalBarSize } from "../settings/display/service";

/** Makes only the brand strip draggable and trims transparent window hit area. */
export function useGlobalBarWindow() {
  useEffect(() => {
    prepareNativeGlobalBarDocument();
    void getDisplaySettings().then((settings) => applyGlobalBarWidth(settings.globalBarSize)).catch(() => {
      // The browser fixture has no native command surface. Its natural width remains useful.
    });
    const bar = document.querySelector<HTMLElement>(".global-bar");
    const handle = document.querySelector<HTMLElement>(".global-bar__brand");
    if (!bar || !handle) return;
    const detach = attachGlobalBarDragHandle(handle);
    const positionPersistence = createPositionPersistence(({ x, y }) => {
      void recordGlobalBarPosition(x, y);
    });
    let disposed = false;
    let stopMoving: (() => void) | undefined;
    let stopSizeChange: (() => void) | undefined;
    try {
      void getCurrentWebviewWindow().onMoved(({ payload }) => {
        positionPersistence.schedule(payload);
      }).then((unlisten) => {
        if (disposed) unlisten();
        else stopMoving = unlisten;
      });
    } catch {
      // Browser fixtures do not expose a native window; production Tauri does.
    }
    try {
      void listen<GlobalBarSize>("global-bar-size-changed", ({ payload }) => {
        applyGlobalBarWidth(payload);
      }).then((unlisten) => {
        if (disposed) unlisten();
        else stopSizeChange = unlisten;
      });
    } catch {
      // Browser fixtures do not expose native events.
    }
    let timer: ReturnType<typeof setTimeout> | undefined;
    const resize = () => {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        const { width, height } = bar.getBoundingClientRect();
        void resizeGlobalBarWindow(globalBarWindowSize({ width, height }));
      }, 60);
    };
    const observer = new ResizeObserver(resize);
    observer.observe(bar);
    resize();
    return () => {
      disposed = true;
      if (timer) clearTimeout(timer);
      positionPersistence.flush();
      stopMoving?.();
      stopSizeChange?.();
      observer.disconnect();
      detach();
    };
  }, []);
}
