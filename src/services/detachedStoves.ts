import { invoke } from "@tauri-apps/api/core";
import type { StoveWire } from "../types/stove";

const DETACHED_LABEL_PREFIX = "stove-";

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
  clear: (stoveId: string) => invoke<boolean>("clear_detached_stove", { stoveId }),
  recordPosition: (stoveId: string, x: number, y: number) => (
    invoke<boolean>("record_detached_stove_position", { stoveId, x, y })
  ),
};

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
