import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  attachGlobalBarDragHandle, attachGlobalBarResizeHandle, createGlobalBarDockController,
  createGlobalBarDockTransport, prepareNativeGlobalBarDocument, intrinsicGlobalBarMinimumHeight,
  globalBarMinimumRequestKey, recordGlobalBarSize, setGlobalBarMinimumSize,
} from "../services/globalBarWindow";

/** Keeps native Global Bar actions tied to local pointer, focus, and resize gestures. */
export function useGlobalBarWindow() {
  useEffect(() => {
    prepareNativeGlobalBarDocument();
    const bar = document.querySelector<HTMLElement>(".global-bar");
    if (!bar) return;
    let disposed = false;
    let lastMinimumRequest: string | undefined;
    let updateMinimum = () => {};
    let wasCollapsed = false;
    const dock = createGlobalBarDockController(createGlobalBarDockTransport(), (state) => {
      document.documentElement.dataset.cookbenchDockState = state.phase;
      if (wasCollapsed && !state.collapsed) {
        lastMinimumRequest = undefined;
        updateMinimum();
      }
      wasCollapsed = state.collapsed;
    }, () => updateMinimum());
    dock.setGuards({
      pointerInside: bar.matches(":hover"),
      focused: bar.contains(document.activeElement),
      menuOpen: bar.dataset.menuOpen === "true",
    });
    let stopDock = () => {};
    void dock.initialize().then((unlisten) => { if (disposed) unlisten(); else stopDock = unlisten; });
    const detach = attachGlobalBarDragHandle(bar, () => dock.start());
    const endDrag = () => dock.endDrag();
    const endResize = () => dock.settleResize();
    window.addEventListener("pointerup", endDrag);
    window.addEventListener("pointercancel", endDrag);
    window.addEventListener("pointerup", endResize);
    window.addEventListener("pointercancel", endResize);
    const enter = () => { dock.setGuards({ pointerInside: true }); if (dock.state().collapsed) dock.reveal(); };
    const leave = () => dock.setGuards({ pointerInside: false });
    const focusIn = () => dock.setGuards({ focused: true });
    const focusOut = (event: FocusEvent) => { if (!bar.contains(event.relatedTarget as Node | null)) dock.setGuards({ focused: false }); };
    bar.addEventListener("pointerenter", enter); bar.addEventListener("pointerleave", leave);
    bar.addEventListener("focusin", focusIn); bar.addEventListener("focusout", focusOut);
    const menuObserver = new MutationObserver(() => dock.setGuards({ menuOpen: bar.dataset.menuOpen === "true" }));
    menuObserver.observe(bar, { attributes: true, attributeFilter: ["data-menu-open"] });
    let suppressResizeUntil = 0;
    let preferredHeight: number | undefined;
    let lastKnownWidth: number | undefined;
    const resizeHandles = ["North", "South", "East", "West", "NorthEast", "NorthWest", "SouthEast", "SouthWest"] as const;
    const resizeCleanups = resizeHandles.map((direction) => {
      const handle = document.createElement("div"); handle.className = "global-bar__resize-handle"; bar.append(handle);
      const detachResize = attachGlobalBarResizeHandle(handle, direction, () => {
        dock.startResize();
      }, () => dock.settleResize());
      return () => { detachResize(); handle.remove(); };
    });
    let stopResizing: (() => void) | undefined;
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    const persistNativeSize = () => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        if (dock.state().collapsed) return;
        // A resize event is not release evidence. Keep its guard active until
        // the local pointer lifecycle ends, then refresh through the settled
        // interaction callback.
        void Promise.all([getCurrentWindow().outerSize(), getCurrentWindow().scaleFactor()]).then(([{ width, height }, scaleFactor]) => {
          const size = { width: width / scaleFactor, height: height / scaleFactor };
          const programmaticHeightOnly = Date.now() < suppressResizeUntil && lastKnownWidth != null && Math.abs(lastKnownWidth - size.width) < 1;
          lastKnownWidth = size.width;
          if (!programmaticHeightOnly) { preferredHeight = size.height; return recordGlobalBarSize(size); }
        }).catch(() => undefined);
      }, 180);
    };
    try { void getCurrentWindow().onResized(persistNativeSize).then((unlisten) => { if (disposed) unlisten(); else stopResizing = unlisten; }); } catch { /* browser fixture */ }
    let minimumTimer: ReturnType<typeof setTimeout> | undefined;
    updateMinimum = () => {
      if (minimumTimer) clearTimeout(minimumTimer);
      minimumTimer = setTimeout(() => {
        if (dock.state().collapsed) return;
        const minimum = { width: 280, height: intrinsicGlobalBarMinimumHeight(bar) };
        const request = globalBarMinimumRequestKey(minimum, preferredHeight);
        if (request === lastMinimumRequest) return;
        lastMinimumRequest = request;
        suppressResizeUntil = Date.now() + 500;
        void setGlobalBarMinimumSize(minimum, preferredHeight).then(() => {
          if (dock.state().docked) dock.refresh();
        }).catch(() => {
          if (lastMinimumRequest === request) lastMinimumRequest = undefined;
        });
      }, 60);
    };
    const observer = new ResizeObserver(updateMinimum);
    [".global-bar__brand", ".global-bar__benches", ".global-bar__minimal", ".stove-priority-menu"].forEach((selector) => {
      const element = bar.querySelector<HTMLElement>(selector); if (element) observer.observe(element);
    });
    const mutations = new MutationObserver(updateMinimum); mutations.observe(bar, { childList: true, subtree: true });
    void Promise.all([getCurrentWindow().outerSize(), getCurrentWindow().scaleFactor()]).then(([size, scaleFactor]) => {
      preferredHeight = size.height / scaleFactor; lastKnownWidth = size.width / scaleFactor;
    }).catch(() => undefined).finally(updateMinimum);
    return () => {
      disposed = true; if (resizeTimer) clearTimeout(resizeTimer); if (minimumTimer) clearTimeout(minimumTimer);
      stopResizing?.(); stopDock(); dock.dispose(); delete document.documentElement.dataset.cookbenchDockState;
      menuObserver.disconnect(); observer.disconnect(); mutations.disconnect(); detach(); resizeCleanups.forEach((cleanup) => cleanup());
      window.removeEventListener("pointerup", endDrag); window.removeEventListener("pointercancel", endDrag);
      window.removeEventListener("pointerup", endResize); window.removeEventListener("pointercancel", endResize);
      bar.removeEventListener("pointerenter", enter); bar.removeEventListener("pointerleave", leave);
      bar.removeEventListener("focusin", focusIn); bar.removeEventListener("focusout", focusOut);
    };
  }, []);
}
