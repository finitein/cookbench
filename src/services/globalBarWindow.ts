import { invoke } from "@tauri-apps/api/core";

export type GlobalBarSize = { width: number; height: number };
export type GlobalBarResizeDirection =
  | "East"
  | "North"
  | "NorthEast"
  | "NorthWest"
  | "South"
  | "SouthEast"
  | "SouthWest"
  | "West";
export function prepareNativeGlobalBarDocument(root: HTMLElement = document.documentElement) {
  root.dataset.cookbenchNative = "true";
}

export function clampGlobalBarSize({ width, height }: GlobalBarSize): GlobalBarSize {
  return {
    width: Math.max(280, Math.ceil(width)),
    height: Math.max(80, Math.ceil(height)),
  };
}

export type GlobalBarDockPhase = "undocked" | "dockedExpanded" | "dockedCollapsed";
export type GlobalBarDockState = { phase: GlobalBarDockPhase; docked: boolean; collapsed: boolean; bestEffort: boolean };
export type GlobalBarDragStart = { token: number; completed: boolean; state?: GlobalBarDockState };
export type GlobalBarDockGuards = { pointerInside: boolean; focused: boolean; menuOpen: boolean; resizing: boolean };
export type GlobalBarDockTransport = {
  getState(): Promise<GlobalBarDockState>;
  listen(handler: (state: GlobalBarDockState) => void): Promise<() => void>;
  startDrag(): Promise<GlobalBarDragStart>;
  finishDrag(token: number): Promise<GlobalBarDockState>;
  setGuards(input: GlobalBarDockGuards): Promise<GlobalBarDockState>;
  collapse(): Promise<GlobalBarDockState>;
  reveal(): Promise<GlobalBarDockState>;
  refreshGeometry(): Promise<GlobalBarDockState>;
  waitForPointerRelease(): Promise<boolean>;
};

const EMPTY_DOCK_GUARDS: GlobalBarDockGuards = { pointerInside: false, focused: false, menuOpen: false, resizing: false };

export function createGlobalBarDockTransport(): GlobalBarDockTransport {
  return {
    getState: () => invoke<GlobalBarDockState>("get_global_bar_dock_state"),
    listen: async (handler) => {
      const { listen } = await import("@tauri-apps/api/event");
      return listen<GlobalBarDockState>("cookbench://global-bar-dock-state-changed", ({ payload }) => handler(payload));
    },
    startDrag: () => invoke<GlobalBarDragStart>("start_global_bar_drag"),
    finishDrag: (token) => invoke<GlobalBarDockState>("finish_global_bar_drag", { token }),
    setGuards: (input) => invoke<GlobalBarDockState>("set_global_bar_dock_guards", { input }),
    collapse: () => invoke<GlobalBarDockState>("request_global_bar_dock_collapse"),
    reveal: () => invoke<GlobalBarDockState>("reveal_global_bar_dock_command"),
    refreshGeometry: () => invoke<GlobalBarDockState>("refresh_global_bar_dock_geometry"),
    waitForPointerRelease: () => invoke<boolean>("wait_for_global_bar_pointer_release"),
  };
}

/** Native dock lifecycle, kept independent of React and Tauri globals for deterministic tests. */
export function createGlobalBarDockController(
  transport: GlobalBarDockTransport,
  onState?: (state: GlobalBarDockState) => void,
  onInteractionSettled?: () => void,
) {
  let state: GlobalBarDockState = { phase: "undocked", docked: false, collapsed: false, bestEffort: false };
  let guards = { ...EMPTY_DOCK_GUARDS };
  let collapseTimer: ReturnType<typeof setTimeout> | undefined;
  let activeToken: number | undefined;
  let pendingStart = false;
  let pendingFinish = false;
  let resizePending = false;
  let pointerEnded = false;
  let disposed = false;
  const apply = (next: GlobalBarDockState) => {
    if (disposed) return;
    state = next;
    onState?.(next);
    scheduleCollapse();
  };
  const clearCollapse = () => { if (collapseTimer) clearTimeout(collapseTimer); collapseTimer = undefined; };
  const hasGuard = () => Object.values(guards).some(Boolean);
  const safe = (promise: Promise<GlobalBarDockState>, settled = false) => void promise.then((next) => {
    apply(next);
    if (settled && !disposed) onInteractionSettled?.();
  }).catch(() => {
    if (settled && !disposed) onInteractionSettled?.();
  });
  const scheduleCollapse = () => {
    clearCollapse();
    if (disposed || hasGuard() || state.phase !== "dockedExpanded" || state.bestEffort || activeToken != null || pendingStart || pendingFinish) return;
    collapseTimer = setTimeout(() => {
      collapseTimer = undefined;
      if (!disposed && !hasGuard() && state.phase === "dockedExpanded" && !state.bestEffort && activeToken == null && !pendingStart && !pendingFinish) safe(transport.collapse());
    }, 600);
  };
  const setGuards = (next: Partial<GlobalBarDockGuards>) => {
    guards = { ...guards, ...next };
    clearCollapse();
    if (!disposed) safe(transport.setGuards(guards));
    scheduleCollapse();
  };
  const finish = () => {
    if (activeToken == null || pendingFinish || disposed) return;
    const token = activeToken;
    activeToken = undefined;
    pendingFinish = true;
    pointerEnded = false;
    clearCollapse();
    void transport.finishDrag(token).then((next) => {
      pendingFinish = false;
      apply(next);
      if (!disposed) onInteractionSettled?.();
      scheduleCollapse();
    }).catch(() => {
      pendingFinish = false;
      if (!disposed) onInteractionSettled?.();
      scheduleCollapse();
    });
  };
  const settleResize = () => {
    if (disposed || !resizePending) return;
    resizePending = false;
    setGuards({ resizing: false });
    safe(transport.refreshGeometry());
  };
  return {
    start() {
      if (activeToken != null || pendingStart || pendingFinish || disposed) return;
      pendingStart = true;
      pointerEnded = false;
      clearCollapse();
      void transport.startDrag().then((result) => {
        pendingStart = false;
        if (disposed) {
          if (!result.completed) void transport.finishDrag(result.token).catch(() => undefined);
          return;
        }
        if (result.state) apply(result.state);
        if (result.completed) { pointerEnded = false; onInteractionSettled?.(); scheduleCollapse(); return; }
        activeToken = result.token;
        if (pointerEnded) finish();
      }).catch(() => { pendingStart = false; pointerEnded = false; if (!disposed) onInteractionSettled?.(); scheduleCollapse(); });
    },
    endDrag() { pointerEnded = true; finish(); },
    setGuards,
    interactionActive: () => pendingStart || activeToken != null || pendingFinish || resizePending,
    canRefresh: () => !disposed && !pendingStart && activeToken == null && !pendingFinish && !guards.resizing,
    refresh() { if (!disposed && !pendingStart && activeToken == null && !pendingFinish && !guards.resizing) safe(transport.refreshGeometry()); },
    reveal() { if (!disposed) safe(transport.reveal()); },
    startResize() {
      if (disposed || resizePending) return;
      resizePending = true;
      setGuards({ resizing: true });
      void transport.waitForPointerRelease().then((released) => {
        if (!disposed && released) settleResize();
      }).catch(() => undefined);
    },
    settleResize,
    async initialize() {
      await transport.getState().then(apply).catch(() => undefined);
      if (disposed) return () => {};
      return transport.listen(apply).then((unlisten) => {
        if (disposed) { unlisten(); return () => {}; }
        return unlisten;
      }).catch(() => () => {});
    },
    state: () => state,
    dispose(unlisten?: () => void) {
      disposed = true; clearCollapse(); unlisten?.();
      void transport.setGuards(EMPTY_DOCK_GUARDS).catch(() => undefined);
      if (activeToken != null && !pendingFinish) { const token = activeToken; activeToken = undefined; pendingFinish = true; void transport.finishDrag(token).catch(() => undefined); }
    },
  };
}

export function attachGlobalBarDragHandle(handle: HTMLElement, onDragStart?: () => void) {
  const drag = (event: PointerEvent) => {
    if ((event.target as Element | null)?.closest(
      "button, a, input, select, textarea, [data-resize-direction]",
    )) return;
    onDragStart?.();
  };
  handle.addEventListener("pointerdown", drag);
  return () => handle.removeEventListener("pointerdown", drag);
}

export function attachGlobalBarResizeHandle(
  handle: HTMLElement,
  direction: GlobalBarResizeDirection,
  onStart?: () => void,
) {
  handle.dataset.resizeDirection = direction;
  handle.setAttribute("aria-hidden", "true");
  const resize = (event: PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    onStart?.();
    void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().startResizeDragging(direction)).catch(() => undefined);
  };
  handle.addEventListener("pointerdown", resize);
  return () => handle.removeEventListener("pointerdown", resize);
}

export function recordGlobalBarSize(size: GlobalBarSize) {
  return invoke<void>("record_global_bar_size", clampGlobalBarSize(size));
}

export function setGlobalBarMinimumSize(size: GlobalBarSize, preferredHeight?: number) {
  return invoke<void>("set_global_bar_minimum_size", {
    ...clampGlobalBarSize(size),
    preferredHeight,
  });
}

/** Measures only visible content, never the native window-filling Bar itself. */
export function intrinsicGlobalBarMinimumHeight(bar: HTMLElement): number {
  const barTop = bar.getBoundingClientRect().top;
  const contentBottom = [
    bar.querySelector<HTMLElement>(".global-bar__brand"),
    bar.querySelector<HTMLElement>(".global-bar__benches"),
    bar.querySelector<HTMLElement>(".global-bar__minimal"),
    bar.querySelector<HTMLElement>(".stove-priority-menu"),
    bar.querySelector<HTMLElement>(".global-bar__tooltip"),
  ].reduce((bottom, element) => {
    if (!element) return bottom;
    return Math.max(bottom, element.getBoundingClientRect().bottom - barTop);
  }, 0);
  return Math.max(80, Math.ceil(contentBottom + 11));
}

export function recordGlobalBarPosition(x: number, y: number) {
  return invoke<void>("record_global_bar_position", { x, y });
}
