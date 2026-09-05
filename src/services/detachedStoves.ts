import { invoke } from "@tauri-apps/api/core";
import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { StoveWire } from "../types/stove";

const DETACHED_LABEL_PREFIX = "stove-";

/** Matches Rust detach_stove_key default; keep content-tight on Linux/X11. */
export const DETACHED_STOVE_WINDOW_SIZE = { width: 164, height: 104 } as const;

/** Re-assert size after WebKit/GTK may inflate the first frame past inner_size. */
export async function applyDetachedWindowSize(
  size: { width: number; height: number } = DETACHED_STOVE_WINDOW_SIZE,
) {
  const window = getCurrentWindow();
  // WebKitGTK may ignore setSize while the window is locked non-resizable.
  try {
    await window.setResizable(true);
  } catch {
    // Some hosts reject setResizable; still attempt setSize below.
  }
  await window.setSize(new LogicalSize(size.width, size.height));
}

export type DetachedWindowResponse = { stoveId: string; label: string };

export function detachedWindowLabel(stoveId: string): string {
  const bytes = new TextEncoder().encode(stoveId);
  return `${DETACHED_LABEL_PREFIX}${[...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("")}`;
}

export function stoveForDetachedWindow(stoves: readonly StoveWire[], label: string): StoveWire | undefined {
  return stoves.find((stove) => detachedWindowLabel(stove.id) === label);
}

export const detachedStoveTransport = {
  detach: (stoveId: string) => invoke<DetachedWindowResponse>("detach_stove", { stoveId }),
  close: (stoveId: string) => invoke<boolean>("close_detached_bar", { stoveId }),
  clear: (stoveId: string) => invoke<boolean>("clear_detached_stove", { stoveId }),
  recordPosition: (stoveId: string, x: number, y: number) => (
    invoke<boolean>("record_detached_stove_position", { stoveId, x, y })
  ),
};

export function startDetachedWindowDrag() {
  return getCurrentWindow().startDragging();
}

export type WindowPosition = { x: number; y: number };

export function createPositionPersistence(
  persist: (position: WindowPosition) => void,
  delayMs = 180,
) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  let pending: WindowPosition | undefined;
  const flush = () => {
    if (timer) clearTimeout(timer);
    timer = undefined;
    if (pending) persist(pending);
    pending = undefined;
  };
  return {
    schedule(position: WindowPosition) {
      pending = position;
      if (timer) clearTimeout(timer);
      timer = setTimeout(flush, delayMs);
    },
    flush,
  };
}
