import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  attachGlobalBarDragHandle, attachGlobalBarResizeHandle, createGlobalBarDockController,
  createGlobalBarDockTransport, prepareNativeGlobalBarDocument, intrinsicGlobalBarMinimumHeight,
  intrinsicGlobalBarMinimumWidth, globalBarMinimumRequestKey, preferredHeightForGlobalBarMode,
  preferredWidthForGlobalBarMode, recordGlobalBarSize, setGlobalBarMinimumSize,
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
    let fullPreferredHeight: number | undefined;
    let fullPreferredWidth: number | undefined;
    let lastKnownWidth: number | undefined;
    let lastKnownHeight: number | undefined;
    let lastChromeMode: "full" | "minimal" | undefined;
    const chromeMode = (): "full" | "minimal" => (
      bar.classList.contains("global-bar--minimal") ? "minimal" : "full"
    );
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
          // Mode flips programmatically change height and/or width; skip persist so
          // Minimal collapse cannot overwrite remembered Full chrome. User drags
          // settle after the suppress window expires.
          const programmaticResize = Date.now() < suppressResizeUntil;
          lastKnownWidth = size.width;
          lastKnownHeight = size.height;
          if (!programmaticResize) {
            // Remember Full-mode size separately so Minimal collapse cannot
            // permanently shrink the restored Full chrome.
            if (chromeMode() === "full") {
              fullPreferredHeight = size.height;
              fullPreferredWidth = size.width;
            }
            return recordGlobalBarSize(size);
          }
        }).catch(() => undefined);
      }, 180);
    };
    try { void getCurrentWindow().onResized(persistNativeSize).then((unlisten) => { if (disposed) unlisten(); else stopResizing = unlisten; }); } catch { /* browser fixture */ }
    let minimumTimer: ReturnType<typeof setTimeout> | undefined;
    updateMinimum = () => {
      if (minimumTimer) clearTimeout(minimumTimer);
      minimumTimer = setTimeout(() => {
        if (dock.state().collapsed) return;
        const mode = chromeMode();
        if (lastChromeMode != null && lastChromeMode !== mode) {
          // Mode flips must re-apply size even when content metrics match.
          lastMinimumRequest = undefined;
        }
        lastChromeMode = mode;
        // Keep native min width at the usable floor; preferredWidth drives shrink/restore.
        const minimum = { width: 280, height: intrinsicGlobalBarMinimumHeight(bar) };
        const contentWidth = intrinsicGlobalBarMinimumWidth(bar);
        const preferredHeight = preferredHeightForGlobalBarMode(mode, minimum.height, fullPreferredHeight);
        const preferredWidth = preferredWidthForGlobalBarMode(mode, contentWidth, fullPreferredWidth);
        const request = globalBarMinimumRequestKey(minimum, preferredHeight, preferredWidth);
        if (request === lastMinimumRequest) return;
        lastMinimumRequest = request;
        suppressResizeUntil = Date.now() + 500;
        void setGlobalBarMinimumSize(minimum, preferredHeight, preferredWidth).then(() => {
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
    const mutations = new MutationObserver(updateMinimum);
    mutations.observe(bar, { childList: true, subtree: true, attributes: true, attributeFilter: ["class"] });
    void Promise.all([getCurrentWindow().outerSize(), getCurrentWindow().scaleFactor()]).then(([size, scaleFactor]) => {
      const restoredHeight = size.height / scaleFactor;
      const restoredWidth = size.width / scaleFactor;
      lastKnownWidth = restoredWidth;
      lastKnownHeight = restoredHeight;
      // Seed Full preference from the restored window only when launching in Full
      // mode. Minimal launches must shrink to content instead of keeping blank
      // chrome from a prior Full session.
      if (chromeMode() === "full") {
        fullPreferredHeight = restoredHeight;
        fullPreferredWidth = restoredWidth;
      }
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
