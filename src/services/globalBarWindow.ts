import { invoke } from "@tauri-apps/api/core";

export type GlobalBarSize = { width: number; height: number };
export type GlobalBarWorkArea = { width: number; height: number };
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

export type GlobalBarChromeMode = "full" | "minimal";

/** Matches `.global-bar { min-height: 104px }`. */
export const FULL_GLOBAL_BAR_MIN_HEIGHT = 104;

/**
 * Compact Minimal width floor: brand (52) + burner (74) + mode/priority
 * controls + paddings, clamped to the native usable floor (280).
 * Measuring `.global-bar__minimal` itself is useless — it stretches with
 * `minmax(0, 1fr)` to the current (often Full) window width.
 */
export const MINIMAL_GLOBAL_BAR_COMPACT_WIDTH = 280;

/**
 * Minimal mode must shrink to content; Full may keep a remembered user height.
 * Full always respects the CSS floor so a persisted Minimal size (~92–97px)
 * cannot stick after restart or a Minimal→Full toggle when the native window
 * still clips content (measurements cannot climb out of that clip alone).
 */
export function preferredHeightForGlobalBarMode(
  mode: GlobalBarChromeMode,
  contentHeight: number,
  fullPreferredHeight?: number,
): number | undefined {
  const content = Math.max(80, Math.ceil(contentHeight));
  if (mode === "minimal") {
    return content;
  }
  const remembered =
    fullPreferredHeight != null && Number.isFinite(fullPreferredHeight)
      ? Math.ceil(fullPreferredHeight)
      : 0;
  return Math.max(content, remembered, FULL_GLOBAL_BAR_MIN_HEIGHT);
}

/**
 * Minimal mode must shrink to a compact width; Full may keep a remembered user
 * width so Full↔Minimal toggles restore the wide chrome instead of sticking at
 * the Minimal compact size (mirror of preferredHeightForGlobalBarMode).
 */
export function preferredWidthForGlobalBarMode(
  mode: GlobalBarChromeMode,
  contentWidth: number,
  fullPreferredWidth?: number,
): number | undefined {
  const content = Math.max(MINIMAL_GLOBAL_BAR_COMPACT_WIDTH, Math.ceil(contentWidth));
  if (mode === "minimal") {
    return content;
  }
  const remembered =
    fullPreferredWidth != null && Number.isFinite(fullPreferredWidth)
      ? Math.ceil(fullPreferredWidth)
      : 0;
  return Math.max(content, remembered);
}

export function globalBarMinimumRequestKey(
  size: GlobalBarSize,
  preferredHeight?: number,
  preferredWidth?: number,
): string {
  const minimum = clampGlobalBarSize(size);
  const preferred = preferredHeight != null && Number.isFinite(preferredHeight)
    ? Math.max(minimum.height, Math.ceil(preferredHeight))
    : "auto";
  const preferredW = preferredWidth != null && Number.isFinite(preferredWidth)
    ? Math.max(minimum.width, Math.ceil(preferredWidth))
    : "auto";
  return `${minimum.width}:${minimum.height}:${preferred}:${preferredW}`;
}

export type GlobalBarDockPhase = "undocked" | "dockedExpanded" | "dockedCollapsed";
export type GlobalBarDockState = { phase: GlobalBarDockPhase; docked: boolean; collapsed: boolean; bestEffort: boolean };
export type GlobalBarDragStart = { token: number; completed: boolean; releaseConfirmed: boolean; state?: GlobalBarDockState };
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
const TOP_DOCK_REVEAL_ARM_DELAY_MS = 150;

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
  let revealArmTimer: ReturnType<typeof setTimeout> | undefined;
  let revealArmed = false;
  let activeToken: number | undefined;
  let pendingStart = false;
  let pendingFinish = false;
  let resizePending = false;
  let resizeGeneration = 0;
  let releaseUnconfirmed = false;
  let pointerEnded = false;
  let disposed = false;
  // Auto-hide only after the user has found the expanded dock this session.
  // Restoring a docked Bar must not immediately collapse it off-screen.
  let hideArmed = false;
  const clearRevealArm = () => {
    if (revealArmTimer) clearTimeout(revealArmTimer);
    revealArmTimer = undefined;
  };
  const apply = (next: GlobalBarDockState) => {
    if (disposed) return;
    const enteredCollapsed = next.collapsed && !state.collapsed;
    state = next;
    if (enteredCollapsed) {
      clearRevealArm();
      revealArmed = false;
      // Always arm after the anti-flicker window. Pointer may already be inside
      // (collapse moved under the cursor, or the user entered during the delay);
      // reveal() still no-ops until armed, and the hook reveals on a post-arm
      // pointermove so hover works without requiring an extra leave/enter.
      revealArmTimer = setTimeout(() => {
        revealArmTimer = undefined;
        if (!disposed && state.collapsed) revealArmed = true;
      }, TOP_DOCK_REVEAL_ARM_DELAY_MS);
    } else if (!next.collapsed) {
      clearRevealArm();
      revealArmed = false;
    }
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
    if (disposed || !hideArmed || hasGuard() || releaseUnconfirmed || state.phase !== "dockedExpanded" || state.bestEffort || activeToken != null || pendingStart || pendingFinish) return;
    collapseTimer = setTimeout(() => {
      collapseTimer = undefined;
      if (!disposed && hideArmed && !hasGuard() && !releaseUnconfirmed && state.phase === "dockedExpanded" && !state.bestEffort && activeToken == null && !pendingStart && !pendingFinish) safe(transport.collapse());
    }, 600);
  };
  const setGuards = (next: Partial<GlobalBarDockGuards>) => {
    const pointerWasInside = guards.pointerInside;
    guards = { ...guards, ...next };
    if (state.collapsed && pointerWasInside && !guards.pointerInside) {
      clearRevealArm();
      revealArmed = true;
    }
    if (guards.pointerInside && state.docked) {
      hideArmed = true;
    }
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
      if (state.docked) hideArmed = true;
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
    resizeGeneration += 1;
    releaseUnconfirmed = false;
    setGuards({ resizing: false });
    safe(transport.refreshGeometry());
  };
  return {
    start() {
      if (activeToken != null || pendingStart || pendingFinish || disposed) return;
      if (resizePending) settleResize();
      releaseUnconfirmed = false;
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
        if (result.completed) {
          const releasedDuringStart = pointerEnded;
          pointerEnded = false;
          releaseUnconfirmed = !result.releaseConfirmed && !releasedDuringStart;
          if (releasedDuringStart) safe(transport.refreshGeometry());
          onInteractionSettled?.(); scheduleCollapse(); return;
        }
        activeToken = result.token;
        if (pointerEnded) finish();
      }).catch(() => { pendingStart = false; pointerEnded = false; if (!disposed) onInteractionSettled?.(); scheduleCollapse(); });
    },
    endDrag() {
      const shouldRefresh = releaseUnconfirmed && !resizePending;
      pointerEnded = true;
      releaseUnconfirmed = false;
      if (shouldRefresh) safe(transport.refreshGeometry());
      finish();
      scheduleCollapse();
    },
    setGuards,
    interactionActive: () => pendingStart || activeToken != null || pendingFinish || resizePending || releaseUnconfirmed,
    canRefresh: () => !disposed && !releaseUnconfirmed && !pendingStart && activeToken == null && !pendingFinish && !guards.resizing,
    refresh() { if (!disposed && !releaseUnconfirmed && !pendingStart && activeToken == null && !pendingFinish && !guards.resizing) safe(transport.refreshGeometry()); },
    reveal() {
      if (!disposed && state.collapsed && revealArmed) {
        revealArmed = false;
        clearRevealArm();
        safe(transport.reveal());
      }
    },
    startResize() {
      if (disposed) return;
      if (resizePending) settleResize();
      releaseUnconfirmed = false;
      const generation = resizeGeneration + 1;
      resizeGeneration = generation;
      resizePending = true;
      setGuards({ resizing: true });
      void transport.waitForPointerRelease().then((released) => {
        if (disposed || generation !== resizeGeneration || !resizePending) return;
        if (released) settleResize();
        else releaseUnconfirmed = true;
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
      disposed = true; clearCollapse(); clearRevealArm(); unlisten?.();
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
  onFailure?: () => void,
) {
  handle.dataset.resizeDirection = direction;
  handle.setAttribute("aria-hidden", "true");
  const resize = (event: PointerEvent) => {
    event.preventDefault();
    event.stopPropagation();
    onStart?.();
    void import("@tauri-apps/api/window")
      .then(({ getCurrentWindow }) => getCurrentWindow().startResizeDragging(direction))
      .catch(() => onFailure?.());
  };
  handle.addEventListener("pointerdown", resize);
  return () => handle.removeEventListener("pointerdown", resize);
}

export function recordGlobalBarSize(size: GlobalBarSize) {
  return invoke<void>("record_global_bar_size", clampGlobalBarSize(size));
}

export function setGlobalBarMinimumSize(
  size: GlobalBarSize,
  preferredHeight?: number,
  preferredWidth?: number,
) {
  return invoke<void>("set_global_bar_minimum_size", {
    ...clampGlobalBarSize(size),
    preferredHeight,
    preferredWidth,
  });
}

/** The current monitor work area, in logical pixels, from the native shell. */
export function getGlobalBarWorkArea() {
  return invoke<GlobalBarWorkArea>("get_global_bar_work_area");
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

/**
 * Compact width from fixed chrome pieces — not the stretched Minimal column,
 * which tracks the current native window and cannot shrink itself.
 */
export function intrinsicGlobalBarMinimumWidth(bar: HTMLElement): number {
  if (bar.classList.contains("global-bar--minimal")) {
    const brand = bar.querySelector<HTMLElement>(".global-bar__brand");
    const cluster = bar.querySelector<HTMLElement>(
      ".global-bar__minimal-burner, .global-bar__minimal-empty, .stove-burner-wrap--compact, .global-bar__minimal-mark",
    );
    const menu = bar.querySelector<HTMLElement>(".stove-priority-menu");
    const brandW = brand?.offsetWidth || 52;
    const clusterW = cluster?.offsetWidth || 74;
    // mode toggle + priority trigger sit beside the burner once the window is compact
    const controls = 24 + 5 + 24;
    const chrome = 12 + brandW + 8 + clusterW + 12 + controls + 10;
    const menuW = menu ? Math.ceil(menu.getBoundingClientRect().width) + 24 : 0;
    return Math.max(MINIMAL_GLOBAL_BAR_COMPACT_WIDTH, Math.ceil(chrome), menuW);
  }
  // Full benches stretch with the window; do not treat that as a content floor.
  // Preferred Full width comes from remembered size (see preferredWidthForGlobalBarMode).
  return MINIMAL_GLOBAL_BAR_COMPACT_WIDTH;
}

/**
 * Find the narrowest Full width at or above the user's preferred width that
 * lets wrapped benches fit inside the current monitor work area. Re-evaluating
 * from the user width prevents a fit-induced resize feedback loop.
 */
export function preferredFullWidthForGlobalBarWorkArea(
  bar: HTMLElement,
  workArea: GlobalBarWorkArea,
  preferredWidth = Math.ceil(bar.getBoundingClientRect().width),
): number | undefined {
  const maximumWidth = Math.floor(workArea.width);
  const maximumHeight = Math.floor(workArea.height);
  const currentWidth = Math.ceil(bar.getBoundingClientRect().width);
  const baselineWidth = Math.max(MINIMAL_GLOBAL_BAR_COMPACT_WIDTH, Math.ceil(preferredWidth));
  const currentHeight = intrinsicGlobalBarMinimumHeight(bar);
  if (!Number.isFinite(maximumWidth) || !Number.isFinite(maximumHeight) || maximumWidth < baselineWidth) return undefined;

  const metrics = [...bar.querySelectorAll<HTMLElement>(".global-bar__bench")].flatMap((bench) => {
    const grid = bench.querySelector<HTMLElement>(".global-bar__bench-stoves");
    const item = grid?.querySelector<HTMLElement>(".global-bar__item");
    if (!grid || !item) return [];
    const gridBox = grid.getBoundingClientRect();
    const itemBox = item.getBoundingClientRect();
    const benchBox = bench.getBoundingClientRect();
    const style = getComputedStyle(grid);
    const columnGap = Number.parseFloat(style.columnGap) || 0;
    const rowGap = Number.parseFloat(style.rowGap) || 0;
    const itemWidth = itemBox.width;
    const itemHeight = itemBox.height;
    if (gridBox.width <= 0 || gridBox.height <= 0 || itemWidth <= 0 || itemHeight <= 0) return [];
    return [{
      count: grid.querySelectorAll(".global-bar__item").length,
      width: gridBox.width,
      itemWidth,
      itemHeight,
      columnGap,
      rowGap,
      overhead: Math.max(0, benchBox.height - gridBox.height),
      benchHeight: benchBox.height,
    }];
  });
  if (metrics.length === 0) return undefined;

  const staticHeight = Math.max(0, currentHeight - metrics.reduce((sum, metric) => sum + metric.benchHeight, 0));
  const candidates = new Set<number>([baselineWidth, maximumWidth]);
  for (const metric of metrics) {
    const maxColumns = Math.max(1, Math.floor((metric.width + (maximumWidth - currentWidth) + metric.columnGap) / (metric.itemWidth + metric.columnGap)));
    for (let columns = 1; columns <= maxColumns; columns += 1) {
      candidates.add(Math.ceil(currentWidth + columns * (metric.itemWidth + metric.columnGap) - metric.columnGap - metric.width));
    }
  }

  for (const width of [...candidates].filter((value) => value >= baselineWidth && value <= maximumWidth).sort((left, right) => left - right)) {
    const projectedHeight = staticHeight + metrics.reduce((sum, metric) => {
      const columns = Math.max(1, Math.floor((metric.width + (width - currentWidth) + metric.columnGap) / (metric.itemWidth + metric.columnGap)));
      const rows = Math.ceil(metric.count / columns);
      const gridHeight = rows * metric.itemHeight + Math.max(0, rows - 1) * metric.rowGap;
      return sum + metric.overhead + gridHeight;
    }, 0);
    if (projectedHeight <= maximumHeight) return width;
  }

  return maximumWidth;
}

export function recordGlobalBarPosition(x: number, y: number) {
  return invoke<void>("record_global_bar_position", { x, y });
}
