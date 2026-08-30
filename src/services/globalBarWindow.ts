import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

export type GlobalBarSize = { width: number; height: number };
export type GlobalBarWidth = "compact" | "standard" | "wide";

export function globalBarContentWidth(size: GlobalBarWidth) {
  switch (size) {
    case "compact": return 360;
    case "wide": return 900;
    case "standard": return 640;
  }
}

export function prepareNativeGlobalBarDocument(root: HTMLElement = document.documentElement) {
  root.dataset.cookbenchNative = "true";
}

export function applyGlobalBarWidth(size: GlobalBarWidth, root: HTMLElement = document.documentElement) {
  root.dataset.cookbenchBarSize = size;
}

export function globalBarWindowSize(content: GlobalBarSize): GlobalBarSize {
  return { width: content.width + 24, height: content.height + 24 };
}

export function clampGlobalBarSize({ width, height }: GlobalBarSize): GlobalBarSize {
  return {
    width: Math.max(120, Math.min(1024, Math.ceil(width))),
    height: Math.max(80, Math.min(720, Math.ceil(height))),
  };
}

export function attachGlobalBarDragHandle(handle: HTMLElement) {
  handle.setAttribute("data-tauri-drag-region", "");
  handle.setAttribute("title", "Move Cookbench");
  const drag = (event: PointerEvent) => {
    if ((event.target as Element | null)?.closest("button, a, input, select, textarea")) return;
    void getCurrentWebviewWindow().startDragging();
  };
  handle.addEventListener("pointerdown", drag);
  return () => handle.removeEventListener("pointerdown", drag);
}

export function resizeGlobalBarWindow(size: GlobalBarSize) {
  return invoke<void>("resize_global_bar", clampGlobalBarSize(size));
}

export function recordGlobalBarPosition(x: number, y: number) {
  return invoke<void>("record_global_bar_position", { x, y });
}
