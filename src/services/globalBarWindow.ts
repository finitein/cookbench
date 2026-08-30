import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

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

export function attachGlobalBarDragHandle(handle: HTMLElement) {
  handle.setAttribute("data-tauri-drag-region", "");
  const drag = (event: PointerEvent) => {
    if ((event.target as Element | null)?.closest(
      "button, a, input, select, textarea, [data-resize-direction]",
    )) return;
    void getCurrentWindow().startDragging();
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
    void getCurrentWindow().startResizeDragging(direction);
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
