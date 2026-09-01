import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import {
  attachGlobalBarDragHandle,
  attachGlobalBarResizeHandle,
  prepareNativeGlobalBarDocument,
  intrinsicGlobalBarMinimumHeight,
  recordGlobalBarPosition,
  recordGlobalBarSize,
  setGlobalBarMinimumSize,
} from "../services/globalBarWindow";
import { createPositionPersistence } from "../services/detachedStoves";

/** Makes the white surface draggable and keeps native resize user-authored. */
export function useGlobalBarWindow() {
  useEffect(() => {
    prepareNativeGlobalBarDocument();
    const bar = document.querySelector<HTMLElement>(".global-bar");
    if (!bar) return;
    const detach = attachGlobalBarDragHandle(bar);
    let suppressResizeUntil = 0;
    let preferredHeight: number | undefined;
    let lastKnownWidth: number | undefined;
    const resizeHandles = [
      "North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest",
    ] as const;
    const resizeCleanups = resizeHandles.map((direction) => {
      const resizeHandle = document.createElement("div");
      resizeHandle.className = "global-bar__resize-handle";
      bar.append(resizeHandle);
      const detachResize = attachGlobalBarResizeHandle(resizeHandle, direction);
      return () => {
        detachResize();
        resizeHandle.remove();
      };
    });
    const positionPersistence = createPositionPersistence(({ x, y }) => {
      void recordGlobalBarPosition(x, y);
    });
    let disposed = false;
    let stopMoving: (() => void) | undefined;
    let stopResizing: (() => void) | undefined;
    try {
      void getCurrentWindow().onMoved(({ payload }) => {
        positionPersistence.schedule(payload);
      }).then((unlisten) => {
        if (disposed) unlisten();
        else stopMoving = unlisten;
      });
    } catch {
      // Browser fixtures do not expose a native window; production Tauri does.
    }
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    const persistNativeSize = () => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        void Promise.all([getCurrentWindow().outerSize(), getCurrentWindow().scaleFactor()])
          .then(([{ width, height }, scaleFactor]) => {
            const size = { width: width / scaleFactor, height: height / scaleFactor };
            const programmaticHeightOnly = Date.now() < suppressResizeUntil
              && lastKnownWidth != null
              && Math.abs(lastKnownWidth - size.width) < 1;
            lastKnownWidth = size.width;
            if (programmaticHeightOnly) return;
            preferredHeight = size.height;
            return recordGlobalBarSize(size);
          })
          .catch(() => {
            // Browser fixtures do not expose a native window.
          });
      }, 180);
    };
    try {
      void getCurrentWindow().onResized(persistNativeSize).then((unlisten) => {
        if (disposed) unlisten();
        else stopResizing = unlisten;
      });
    } catch {
      // Browser fixtures do not expose a native window.
    }
    let minimumTimer: ReturnType<typeof setTimeout> | undefined;
    const updateMinimum = () => {
      if (minimumTimer) clearTimeout(minimumTimer);
      minimumTimer = setTimeout(() => {
        const height = intrinsicGlobalBarMinimumHeight(bar);
        suppressResizeUntil = Date.now() + 500;
        void setGlobalBarMinimumSize({ width: 280, height }, preferredHeight)
          .catch(() => {
            // Browser fixtures do not expose a native command surface.
          });
      }, 60);
    };
    const observer = new ResizeObserver(updateMinimum);
    const brand = bar.querySelector<HTMLElement>(".global-bar__brand");
    const benches = bar.querySelector<HTMLElement>(".global-bar__benches");
    const minimal = bar.querySelector<HTMLElement>(".global-bar__minimal");
    const priorityMenu = bar.querySelector<HTMLElement>(".stove-priority-menu");
    if (brand) observer.observe(brand);
    if (benches) observer.observe(benches);
    if (minimal) observer.observe(minimal);
    if (priorityMenu) observer.observe(priorityMenu);
    const mutations = new MutationObserver(updateMinimum);
    mutations.observe(bar, { childList: true, subtree: true });
    void Promise.all([getCurrentWindow().outerSize(), getCurrentWindow().scaleFactor()])
      .then(([size, scaleFactor]) => {
        preferredHeight = size.height / scaleFactor;
        lastKnownWidth = size.width / scaleFactor;
      })
      .catch(() => {
        // Browser fixtures do not expose a native window.
      })
      .finally(updateMinimum);
    return () => {
      disposed = true;
      if (resizeTimer) clearTimeout(resizeTimer);
      if (minimumTimer) clearTimeout(minimumTimer);
      positionPersistence.flush();
      stopMoving?.();
      stopResizing?.();
      observer.disconnect();
      mutations.disconnect();
      detach();
      resizeCleanups.forEach((cleanup) => cleanup());
    };
  }, []);
}
