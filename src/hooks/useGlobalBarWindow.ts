import { useEffect } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

import {
  attachGlobalBarDragHandle,
  globalBarWindowSize,
  prepareNativeGlobalBarDocument,
  recordGlobalBarPosition,
  resizeGlobalBarWindow,
} from "../services/globalBarWindow";
import { createPositionPersistence } from "../services/detachedStoves";

/** Makes only the brand strip draggable and trims transparent window hit area. */
export function useGlobalBarWindow() {
  useEffect(() => {
    prepareNativeGlobalBarDocument();
    const bar = document.querySelector<HTMLElement>(".global-bar");
    const handle = document.querySelector<HTMLElement>(".global-bar__brand");
    if (!bar || !handle) return;
    const detach = attachGlobalBarDragHandle(handle);
    const positionPersistence = createPositionPersistence(({ x, y }) => {
      void recordGlobalBarPosition(x, y);
    });
    let disposed = false;
    let stopMoving: (() => void) | undefined;
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
      observer.disconnect();
      detach();
    };
  }, []);
}
