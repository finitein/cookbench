import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  attachGlobalBarDragHandle, attachGlobalBarResizeHandle, createGlobalBarDockController,
  createGlobalBarDockTransport, prepareNativeGlobalBarDocument, intrinsicGlobalBarMinimumHeight,
  recordGlobalBarSize, setGlobalBarMinimumSize,
} from "../services/globalBarWindow";

/** Keeps native Global Bar actions tied to local pointer, focus, and resize gestures. */
export function useGlobalBarWindow() {
  useEffect(() => {
    prepareNativeGlobalBarDocument();
    const bar = document.querySelector<HTMLElement>(".global-bar");
    if (!bar) return;
    let disposed = false;
    const dock = createGlobalBarDockController(createGlobalBarDockTransport(), (state) => {
      document.documentElement.dataset.cookbenchDockState = state.phase;
    }, () => updateMinimum());
    dock.setGuards({
      pointerInside: bar.matches(":hover"),
      focused: bar.contains(document.activeElement),
      menuOpen: bar.dataset.menuOpen === "true",
    });
    let stopDock = () => {};
    void dock.initialize().then((unlisten) => { if (disposed) unlisten(); else stopDock = unlisten; });
    const detach = attachGlobalBarDragHandle(bar, () => dock.start());
    const endDrag = () => { dock.endDrag(); dock.setGuards({ resizing: false }); };
    window.addEventListener("pointerup", endDrag);
    window.addEventListener("pointercancel", endDrag);
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
      const detachResize = attachGlobalBarResizeHandle(handle, direction, () => dock.setGuards({ resizing: true }));
      return () => { detachResize(); handle.remove(); };
    });
    let stopResizing: (() => void) | undefined;
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    const persistNativeSize = () => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        dock.setGuards({ resizing: false }); dock.refresh();
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
    const updateMinimum = () => {
      if (minimumTimer) clearTimeout(minimumTimer);
      minimumTimer = setTimeout(() => {
        suppressResizeUntil = Date.now() + 500;
        void setGlobalBarMinimumSize({ width: 280, height: intrinsicGlobalBarMinimumHeight(bar) }, preferredHeight).then(() => dock.refresh()).catch(() => undefined);
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
      bar.removeEventListener("pointerenter", enter); bar.removeEventListener("pointerleave", leave);
      bar.removeEventListener("focusin", focusIn); bar.removeEventListener("focusout", focusOut);
    };
  }, []);
}
