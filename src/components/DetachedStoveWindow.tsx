import type { StoveWire } from "../types/stove";
import { createPositionPersistence, detachedStoveTransport } from "../services/detachedStoves";
import { DetachedStoveBar } from "./DetachedStoveBar";

export type DetachedStoveWindowProps = {
  stove: StoveWire;
  onActivate?: (stove: StoveWire) => void;
};

export function DetachedStoveWindow({ stove, onActivate }: DetachedStoveWindowProps) {
  useEffect(() => {
    const persistence = createPositionPersistence(({ x, y }) => {
      void detachedStoveTransport.recordPosition(stove.id, x, y);
    });
    let disposed = false;
    let stop: (() => void) | undefined;
    try {
      void getCurrentWebviewWindow().onMoved(({ payload }) => {
        persistence.schedule(payload);
      }).then((unlisten) => {
        if (disposed) unlisten();
        else stop = unlisten;
      });
    } catch {
      // Browser fixtures do not expose a native window; production Tauri does.
    }
    return () => {
      disposed = true;
      persistence.flush();
      stop?.();
    };
  }, [stove.id]);

  return (
    <main className="shell" aria-label="Cookbench detached Stove">
      <DetachedStoveBar
        stove={stove}
        onActivate={onActivate}
        onClear={stove.retainedCompletion ? (current) => { void detachedStoveTransport.clear(current.id); } : undefined}
      />
    </main>
  );
}
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useEffect } from "react";
